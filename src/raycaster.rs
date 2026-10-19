//! CPU-рейкастер (DDA по Лодеву, как Wolf3D) в буфер 320x200.
//! Рендер по индексам палитры + Doom-colormap (16 ступеней) + дизеринг Байера.

use crate::map::{Map, Tile};
use crate::palette::{build_colormap, light_band, BAYER4, PALETTE, NUM_LIGHTS};
use crate::textures::{TexId, Textures, TEX_SIZE};

pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 400;
pub const HORIZON: i32 = 200;

/// Масштаб UI: панели/HUD рисуются в «дизайновых» координатах 320×200
/// и растягиваются на весь буфер 640×400. Мир (рейкастер) рисуется
/// сразу в реальных пикселях — текстуры и стены от этого только чётче.
pub const UI_SCALE: i32 = 2;
/// Ширина/высота UI в дизайновых координатах.
pub const UI_W: i32 = (SCREEN_W / 2) as i32; // 320
pub const UI_H: i32 = (SCREEN_H / 2) as i32; // 200
/// Горизонт в дизайновых координатах (для речевых пузырей над спрайтами).
pub const UI_HORIZON: i32 = HORIZON / 2; // 100
const MAX_DIST: f32 = 24.0;
const FOV_SCALE: f32 = 0.66; // ~66° FOV

#[derive(bevy::prelude::Resource)]
pub struct GameCam {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

pub struct Renderer {
    /// Палитровые индексы кадра.
    pub indices: Vec<u8>,
    /// z-буфер по колонкам (глубина стены).
    pub zbuf: Vec<f32>,
    /// Верхняя отрисованная строка стены по колонкам (для полуприлавка).
    pub ztop: Vec<i32>,
    colormap: [[u8; NUM_LIGHTS]; 24],
    /// RGBA выход.
    pub rgba: Vec<u8>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            indices: vec![0; SCREEN_W * SCREEN_H],
            zbuf: vec![0.0; SCREEN_W],
            ztop: vec![0; SCREEN_W],
            colormap: build_colormap(),
            rgba: vec![0; SCREEN_W * SCREEN_H * 4],
        }
    }

    pub fn render(&mut self, map: &Map, tex: &Textures, cam: &GameCam, flicker: f32) {
        let mut idx = std::mem::take(&mut self.indices);
        idx.fill(0);
        let dir_x = cam.angle.cos();
        let dir_y = cam.angle.sin();
        let plane_x = -dir_y * FOV_SCALE;
        let plane_y = dir_x * FOV_SCALE;

        // --- Потолок и пол (flat casting, от экранных строк) ---
        for y in 0..SCREEN_H {
            let is_floor = y > HORIZON as usize;
            let p = y as i32 - HORIZON;
            if p == 0 {
                continue;
            }
            let row_dist = (0.5 * SCREEN_H as f32 / p.abs() as f32).min(MAX_DIST * 2.0);
            let step_x = row_dist * (plane_x * 2.0) / SCREEN_W as f32;
            let step_y = row_dist * (plane_y * 2.0) / SCREEN_W as f32;
            let mut fx = cam.x + row_dist * (dir_x - plane_x);
            let mut fy = cam.y + row_dist * (dir_y - plane_y);
            // виньетка по строке/колонке заранее
            let ey = (y as f32 - SCREEN_H as f32 / 2.0).abs() / (SCREEN_H as f32 / 2.0);
            for x in 0..SCREEN_W {
                let (tx, ty) = (
                    ((fx * TEX_SIZE as f32) as i32).rem_euclid(TEX_SIZE as i32) as usize,
                    ((fy * TEX_SIZE as f32) as i32).rem_euclid(TEX_SIZE as i32) as usize,
                );
                let c = if is_floor {
                    tex.texel(TexId::Floor, tx, ty)
                } else {
                    tex.texel(TexId::Ceiling, tx, ty)
                };
                let mut bright = (1.0 - row_dist / (MAX_DIST * 1.5)).max(0.15) * flicker;
                if y & 1 == 1 {
                    bright *= 0.75;
                }
                let ex = (x as f32 - SCREEN_W as f32 / 2.0).abs() / (SCREEN_W as f32 / 2.0);
                bright *= 1.0 - 0.25 * ex * ey;
                let band = light_band(bright, BAYER4[(x & 3) as usize][(y & 3) as usize]);
                idx[y * SCREEN_W + x] = self.colormap[c as usize][band];
                fx += step_x;
                fy += step_y;
            }
        }

        // --- Стены (DDA; за низким прилавком луч продолжается к дальней стене) ---
        for x in 0..SCREEN_W {
            let camera_x = 2.0 * x as f32 / SCREEN_W as f32 - 1.0;
            let rdx = dir_x + plane_x * camera_x;
            let rdy = dir_y + plane_y * camera_x;
            let mut map_x = cam.x.floor() as i32;
            let mut map_y = cam.y.floor() as i32;
            let delta_x = if rdx.abs() < 1e-9 { f32::MAX } else { (1.0 / rdx).abs() };
            let delta_y = if rdy.abs() < 1e-9 { f32::MAX } else { (1.0 / rdy).abs() };
            let (step_x, side_x) = if rdx < 0.0 {
                (-1i32, (cam.x - map_x as f32) * delta_x)
            } else {
                (1i32, (map_x as f32 + 1.0 - cam.x) * delta_x)
            };
            let (step_y, side_y) = if rdy < 0.0 {
                (-1i32, (cam.y - map_y as f32) * delta_y)
            } else {
                (1i32, (map_y as f32 + 1.0 - cam.y) * delta_y)
            };
            let (mut side_x, mut side_y) = (side_x, side_y);
            let mut side = 0usize; // 0 = x-грань, 1 = y-грань
            let mut steps = 0;
            // первая попавшаяся стена; если это прилавок — ещё и стена за ним
            let mut first: Option<(Tile, usize, f32)> = None;
            let mut second: Option<(Tile, usize, f32)> = None;
            loop {
                if side_x < side_y {
                    side_x += delta_x;
                    map_x += step_x;
                    side = 0;
                } else {
                    side_y += delta_y;
                    map_y += step_y;
                    side = 1;
                }
                let t = map.get(map_x, map_y);
                if t.ray_solid() {
                    let perp = (if side == 0 {
                        (map_x as f32 - cam.x + (1 - step_x) as f32 / 2.0) / rdx
                    } else {
                        (map_y as f32 - cam.y + (1 - step_y) as f32 / 2.0) / rdy
                    })
                    .max(0.05);
                    if first.is_none() {
                        first = Some((t, side, perp));
                        if t != Tile::Counter {
                            break;
                        }
                    } else {
                        second = Some((t, side, perp));
                        break;
                    }
                }
                steps += 1;
                if steps > 128 {
                    break;
                }
            }
            let Some((hit_tile, side, perp)) = first else { continue };
            self.zbuf[x] = perp;
            // Перспектива честная: высота стены — по реальному
            // перпендикулярному расстоянию (кламп расстояния давал
            // «сжатие» объектов вблизи: они переставали расти).
            let perp_d = perp.max(0.2);
            let full_h = (SCREEN_H as f32 / perp_d) as i32;
            // прилавок — низкая стена до пола, за ней виден дальняя стена и
            // покупатели. Кромка должна идти заметно ниже горизонта (а не
            // практически на нём, как было при 0.48 — тогда людей резало
            // ровно посередине экрана). 0.40: видно от колен вверх, и человек
            // читается как стоящий за прилавком, а не «на» нём.
            let wall_h = if hit_tile == Tile::Counter {
                (full_h as f32 * 0.40) as i32
            } else {
                full_h
            };
            let end_raw = HORIZON + full_h / 2;
            let start_raw = end_raw - wall_h;
            let end = end_raw.min(SCREEN_H as i32);
            let start = start_raw.max(0);
            // Спрайты: над высокой стеной НИЧЕГО не видно (раньше головы
            // покупателей из-за масштаба 1.2 торчали над стенами); у прилавка —
            // видно то, что выступает над его низким торцом.
            self.ztop[x] = if hit_tile == Tile::Counter { start } else { 0 };
            let wall_x = if side == 0 {
                cam.y + perp * rdy
            } else {
                cam.x + perp * rdx
            };
            let wall_x = wall_x - wall_x.floor();
            let mut tex_x = (wall_x * TEX_SIZE as f32) as usize % TEX_SIZE;
            if (side == 0 && rdx > 0.0) || (side == 1 && rdy < 0.0) {
                tex_x = TEX_SIZE - tex_x - 1;
            }
            let base_tex = hit_tile.tex().unwrap_or(TexId::TileWall);
            let dark = if side == 1 { 0.8 } else { 1.0 };
            let ex = (x as f32 - SCREEN_W as f32 / 2.0).abs() / (SCREEN_W as f32 / 2.0);
            for y in start..end {
                let tex_y = (((y - start_raw) as f32 * TEX_SIZE as f32 / wall_h.max(1) as f32)
                    as usize)
                    .min(TEX_SIZE - 1);
                // верх кафельной стены — штукатурка с баннером
                let c = if base_tex == TexId::TileWall && tex_y < 22 {
                    tex.texel(TexId::PlasterBanner, tex_x, tex_y)
                } else {
                    tex.texel(base_tex, tex_x, tex_y)
                };
                let mut bright = (1.0 - perp / MAX_DIST).max(0.15) * flicker * dark;
                if y & 1 == 1 {
                    bright *= 0.75;
                }
                let ey = (y as f32 - SCREEN_H as f32 / 2.0).abs() / (SCREEN_H as f32 / 2.0);
                bright *= 1.0 - 0.25 * ex * ey;
                let band = light_band(bright, BAYER4[(x & 3) as usize][(y & 3) as usize]);
                idx[y as usize * SCREEN_W + x] = self.colormap[c as usize][band];
            }
            // над прилавком — дальняя стена (иначе виден только потолок)
            if hit_tile == Tile::Counter && start > 0 {
                if let Some((t2, side2, perp2)) = second {
                    let full2 = (SCREEN_H as f32 / perp2.max(0.2)) as i32;
                    let end2_raw = HORIZON + full2 / 2;
                    let start2_raw = end2_raw - full2;
                    let end2 = end2_raw.min(SCREEN_H as i32);
                    let start2 = start2_raw.max(0);
                    let dark2 = if side2 == 1 { 0.8 } else { 1.0 };
                    let tex2 = t2.tex().unwrap_or(TexId::TileWall);
                    let wall_x2 = if side2 == 0 {
                        cam.y + perp2 * rdy
                    } else {
                        cam.x + perp2 * rdx
                    };
                    let wall_x2 = wall_x2 - wall_x2.floor();
                    let mut tx2 = (wall_x2 * TEX_SIZE as f32) as usize % TEX_SIZE;
                    if (side2 == 0 && rdx > 0.0) || (side2 == 1 && rdy < 0.0) {
                        tx2 = TEX_SIZE - tx2 - 1;
                    }
                    for y in start2..start {
                        let tex_y =
                            (((y - start2_raw) as f32 * TEX_SIZE as f32 / full2.max(1) as f32)
                                as usize)
                                .min(TEX_SIZE - 1);
                        let c = if tex2 == TexId::TileWall && tex_y < 22 {
                            tex.texel(TexId::PlasterBanner, tx2, tex_y)
                        } else {
                            tex.texel(tex2, tx2, tex_y)
                        };
                        let mut bright = (1.0 - perp2 / MAX_DIST).max(0.15) * flicker * dark2;
                        if y & 1 == 1 {
                            bright *= 0.75;
                        }
                        let ey = (y as f32 - SCREEN_H as f32 / 2.0).abs() / (SCREEN_H as f32 / 2.0);
                        bright *= 1.0 - 0.25 * ex * ey;
                        let band = light_band(bright, BAYER4[(x & 3) as usize][(y & 3) as usize]);
                        idx[y as usize * SCREEN_W + x] = self.colormap[c as usize][band];
                    }
                }
            }
        }

        self.indices = idx;
    }

    /// Конверсия индексов палитры в RGBA.
    pub fn present(&mut self) {
        for i in 0..SCREEN_W * SCREEN_H {
            let c = PALETTE[self.indices[i] as usize];
            let o = i * 4;
            self.rgba[o] = c[0];
            self.rgba[o + 1] = c[1];
            self.rgba[o + 2] = c[2];
            self.rgba[o + 3] = 255;
        }
    }

    /// Billboard-спрайты 32x64 с отсечением по z-буферу колонок.
    pub fn draw_sprites(
        &mut self,
        sheet: &crate::sprites::SpriteSheet,
        sprites: &[(bevy::prelude::Vec2, crate::sprites::NpcKind)],
        cam: &GameCam,
        flicker: f32,
    ) {
        let dir_x = cam.angle.cos();
        let dir_y = cam.angle.sin();
        let plane_x = -dir_y * FOV_SCALE;
        let plane_y = dir_x * FOV_SCALE;
        let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
        // сортировка: дальние первыми
        let mut order: Vec<usize> = (0..sprites.len()).collect();
        order.sort_by(|&a, &b| {
            let da = (sprites[a].0 - bevy::prelude::Vec2::new(cam.x, cam.y)).length_squared();
            let db = (sprites[b].0 - bevy::prelude::Vec2::new(cam.x, cam.y)).length_squared();
            db.partial_cmp(&da).unwrap()
        });
        for &si in &order {
            let (pos, kind) = &sprites[si];
            let sx = pos.x - cam.x;
            let sy = pos.y - cam.y;
            let tx = inv_det * (dir_y * sx - dir_x * sy);
            let ty = inv_det * (-plane_y * sx + plane_x * sy);
            if ty <= 0.15 {
                continue;
            }
            let screen_x = ((SCREEN_W as f32 / 2.0) * (1.0 + tx / ty)) as i32;
            // люди чуть выше тайла, ноги точно на линии пола; высота —
            // честная перспектива (кламп 160px «сплющивал» людей вблизи)
            const SPRITE_SCALE: f32 = 1.2;
            let sprite_h = ((SCREEN_H as f32 / ty * SPRITE_SCALE).max(1.0)) as usize;
            let sprite_w = sprite_h / 2; // 32x64
            let feet = HORIZON as f32 + (0.5 * SCREEN_H as f32) / ty;
            // Клампим только диапазон отрисовки, а НЕ начало спрайта:
            // иначе у близко стоящего человека (голова выше кадра) текстура
            // растягивалась по обрезанному ящику и он «обрезался» посередине.
            let y0_raw = feet - sprite_h as f32;
            let y0 = y0_raw.max(0.0) as i32;
            let y1 = feet.min(SCREEN_H as f32) as i32;
            let x0 = (screen_x - sprite_w as i32 / 2).max(0);
            let x1 = (screen_x + sprite_w as i32 / 2).min(SCREEN_W as i32);
            let data = sheet.npc_sprite(*kind);
            let bright = (1.0 - ty / MAX_DIST).max(0.1) * flicker;
            if std::env::var("KVASS_DEBUG_SPRITES").is_ok() {
                let mid = ((x0 + x1) / 2).clamp(0, SCREEN_W as i32 - 1) as usize;
                eprintln!(
                    "sprite {} ty={ty:.2} sx={screen_x} box=({x0},{y0})-({x1},{y1}) ztop={} zbuf={:.2}",
                    kind.name(),
                    self.ztop[mid],
                    self.zbuf[mid],
                );
            }
            for x in x0..x1 {
                // стена ближе спрайта — видна только её верхняя часть (над прилавком)
                let occluded = self.zbuf[x as usize] < ty;
                let ztop = self.ztop[x as usize];
                let tex_x = ((x - (screen_x - sprite_w as i32)) * crate::sprites::SPR_W as i32
                    / sprite_w.max(1) as i32)
                    .min(crate::sprites::SPR_W as i32 - 1) as usize;
                for y in y0..y1 {
                    if occluded && y >= ztop {
                        continue;
                    }
                    // берём строку текстуры по НЕклампированной верхней точке —
                    // обрезка экрана не должна смещать начало текстуры
                    let tex_y = (((y as f32 - y0_raw) * crate::sprites::SPR_H as f32
                        / sprite_h.max(1) as f32)
                        .clamp(0.0, (crate::sprites::SPR_H - 1) as f32))
                        as usize;
                    let c = data[tex_y * crate::sprites::SPR_W + tex_x];
                    if c == crate::sprites::TRANSPARENT {
                        continue;
                    }
                    let mut b = bright;
                    if y & 1 == 1 {
                        b *= 0.75;
                    }
                    let band = light_band(b, BAYER4[(x as usize & 3) as usize][(y as usize & 3) as usize]);
                    self.indices[y as usize * SCREEN_W + x as usize] =
                        self.colormap[c as usize][band];
                }
            }
        }
    }
}
