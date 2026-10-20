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
    /// Глубина НИЗКОЙ мебели по колонкам (прилавок/бочка/мусорка),
    /// f32::MAX — в этой колонке низкой мебели нет. ТЗ №2: одной глубины
    /// стены за мебелью мало — спрайт за прилавком «простреливал» её фасад.
    pub zlow: Vec<f32>,
    /// Верхняя строка силуэта низкой мебели (крышка) по колонкам:
    /// спрайты глубже неё обрезаются ровно по этой линии.
    pub zlow_top: Vec<i32>,
    colormap: [[u8; NUM_LIGHTS]; 24],
    /// RGBA выход.
    pub rgba: Vec<u8>,
}

/// Низкая мебель: за ней видно дальнюю стену, а спрайты режутся по её крыше.
fn is_low(t: Tile) -> bool {
    matches!(t, Tile::Counter | Tile::Barrel | Tile::Trash)
}

/// Насколько высоко поднимается низкая мебель (доля полной стены).
fn low_frac(t: Tile) -> f32 {
    match t {
        Tile::Counter => 0.34,
        Tile::Barrel => 0.55,
        Tile::Trash => 0.45,
        _ => 1.0,
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            indices: vec![0; SCREEN_W * SCREEN_H],
            zbuf: vec![0.0; SCREEN_W],
            zlow: vec![f32::MAX; SCREEN_W],
            zlow_top: vec![SCREEN_H as i32; SCREEN_W],
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
            // первая ПОЛНАЯ стена за низкой мебелью — реальный ограничитель
            // глубины спрайтов (см. zbuf ниже)
            let mut far: Option<f32> = None;
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
                    let low = is_low(t);
                    if first.is_none() {
                        first = Some((t, side, perp));
                        if !low {
                            break;
                        }
                    } else {
                        // «второй» объект — для отрисовки над прилавком
                        // (может быть и сам прилавок: вид косой вдоль ряда)
                        if second.is_none() {
                            second = Some((t, side, perp));
                        }
                        if !low {
                            far = Some(perp);
                            break;
                        }
                        // очередная клетка низкой мебели — едем дальше за
                        // настоящей стеной, иначе спрайт за прилавком
                        // «закрыт» соседней клеткой прилавка (баг: бабушки
                        // не было видно, zbuf оказывался меньше её глубины)
                    }
                }
                steps += 1;
                if steps > 128 {
                    break;
                }
            }
            let Some((hit_tile, side, perp)) = first else {
                // Луч не попал ни в что (на закрытой карте не бывает): зовём
                // буферы в исходное, чтобы спрайты не ушли за «стену»
                // с прошлого кадра (ТЗ №2: протёкший z-буфер).
                self.zbuf[x] = f32::MAX;
                self.zlow[x] = f32::MAX;
                self.zlow_top[x] = SCREEN_H as i32;
                continue;
            };
            let low = is_low(hit_tile);
            // Прилавок/бочка/мусорка — низкая мебель: покупателя за ней НЕЛЬЗЯ
            // «резать» по кромке (баг «половина человека»). Ограничитель
            // спрайтов ПО ЗАДНЕЙ СТЕНЕ — far, а сама мебель учитывается
            // отдельной парой zlow/zlow_top (ТЗ №2: фасад прилавка нельзя
            // «простреливать» спрайтом, стоящим за ним).
            self.zbuf[x] = if low { far.unwrap_or(f32::MAX) } else { perp };
            // Перспектива честная: высота стены — по реальному
            // перпендикулярному расстоянию (кламп расстояния давал
            // «сжатие» объектов вблизи: они переставали расти).
            let perp_d = perp.max(0.2);
            let full_h = (SCREEN_H as f32 / perp_d) as i32;
            // прилавок — низкая стена до пола, за ней виден дальняя стена и
            // покупатели. Кромка должна идти заметно ниже горизонта (а не
            // практически на нём, как было при 0.48 — тогда людей резало
            // ровно посередине экрана). 0.34: видно от колен вверх, и человек
            // читается как стоящий за прилавком, а не «на» нём.
            let frac = low_frac(hit_tile);
            let wall_h = if low {
                (full_h as f32 * frac) as i32
            } else {
                full_h
            };
            let end_raw = HORIZON + full_h / 2;
            let start_raw = end_raw - wall_h;
            let end = end_raw.min(SCREEN_H as i32);
            let start = start_raw.max(0).min(SCREEN_H as i32);
            // Геометрия крышки считается ЗДЕСЬ (а не в блоке отрисовки):
            // её верхняя строка — это и есть линия, по которой спрайты за
            // мебелью обрезаются. Если мебель достаёт до верха кадра
            // (start == 0), крышки нет и всё за ней скрыто целиком.
            let lid_band_top = if low {
                // t выхода луча из тайла — ближайшая дальняя граница клетки
                let fx = if rdx.abs() < 1e-6 {
                    f32::MAX
                } else if rdx > 0.0 {
                    (map_x as f32 + 1.0 - cam.x) / rdx
                } else {
                    (map_x as f32 - cam.x) / rdx
                };
                let fy = if rdy.abs() < 1e-6 {
                    f32::MAX
                } else if rdy > 0.0 {
                    (map_y as f32 + 1.0 - cam.y) / rdy
                } else {
                    (map_y as f32 - cam.y) / rdy
                };
                let perp_exit = fx.min(fy).max(perp + 0.05);
                let full2 = SCREEN_H as f32 / perp_exit.max(0.2);
                let y_back = (HORIZON as f32 + full2 * (0.5 - frac)) as i32;
                if start > 0 {
                    y_back.max(0).min(start)
                } else {
                    0
                }
            } else {
                SCREEN_H as i32
            };
            self.zlow[x] = if low { perp } else { f32::MAX };
            self.zlow_top[x] = lid_band_top;
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
            // над прилавком/бочкой/мусоркой — дальняя стена (иначе виден
            // только потолок)
            if low && start > 0 {
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
            // Перспективная крышка низкой мебели (ТЗ №6): верхняя грань
            // прилавка/бочки/мусорки — от кромки фронта до задней кромки
            // тайла. Покупатель за низкой мебелью считается низкой
            // мебелью (zlow_top), крышка рисуется поверх дальних стен
            // (честное перекрытие).
            if low && start > 0 {
                // геометрия (perp_exit/край крышки) посчитана до отрисовки
                // фронта — она же задаёт линию обрезки спрайтов (zlow_top)
                let band_top = lid_band_top;
                let span = (start_raw - band_top).max(1);
                for y in band_top..start {
                    let ty2 = (((y - band_top) as f32 * TEX_SIZE as f32 / span as f32) as usize)
                        .min(TEX_SIZE - 1);
                    let c = tex.texel(base_tex, tex_x, ty2);
                    let mut bright = (1.0 - perp / MAX_DIST).max(0.25) * flicker * dark * 1.12;
                    if y & 1 == 1 {
                        bright *= 0.8;
                    }
                    let band =
                        light_band(bright.min(1.35), BAYER4[(x & 3) as usize][(y & 3) as usize]);
                    idx[y as usize * SCREEN_W + x] = self.colormap[c as usize][band];
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
            // Рост человека = доля полной стены-тайла (sprite_h = SCREEN_H/ty*SCALE,
            // полная стена = SCREEN_H/ty). 1.2 делал «переростков» с головой над
            // стеной; 0.75 — человек ~3/4 стены, голова ниже кромки, читается цельным.
            const SPRITE_SCALE: f32 = 0.75;
            let sprite_h = ((SCREEN_H as f32 / ty * SPRITE_SCALE).clamp(1.0, SCREEN_H as f32)) as usize;
            let sprite_w = sprite_h / 2; // 64x128 спрайты
            let feet = HORIZON as f32 + (SCREEN_H as i32 - HORIZON) as f32 / ty;
            // Клампим только диапазон отрисовки, а НЕ начало спрайта:
            // иначе у близко стоящего человека (голова выше кадра) текстура
            // растягивалась по обрезанному ящику и он «обрезался» посередине.
            let y0_raw = feet - sprite_h as f32;
            let y0 = y0_raw.max(0.0) as i32;
            let y1 = feet.min(SCREEN_H as f32) as i32;
            // необрезанная левая граница коробки — ОПОРА для tex_x (всегда
            // исходная, до .max(0): иначе у спрайта у края экрана текстура
            // сжимается). Раньше здесь была ошибка: опорой брался
            // screen_x - sprite_w вместо screen_x - sprite_w/2 — сдвиг на
            // полкоробки давал сэмпл текстуры с середины: левая половина
            // тела не читалась вовсе, справа тянулась кромка (баг «половина
            // спрайта», классика Lodev: tex_x = (x - draw_start_x) * W / w).
            let x_left = screen_x - sprite_w as i32 / 2;
            let x0 = x_left.max(0);
            let x1 = (screen_x + sprite_w as i32 / 2).min(SCREEN_W as i32);
            let data = sheet.npc_sprite(*kind);
            let bright = (1.0 - ty / MAX_DIST).max(0.1) * flicker;
            if std::env::var("KVASS_DEBUG_SPRITES").is_ok() {
                let mid = ((x0 + x1) / 2).clamp(0, SCREEN_W as i32 - 1) as usize;
                eprintln!(
                    "sprite {} ty={ty:.2} screen_x={screen_x} start_x={x0} end_x={x1} width={} \
                     box_y=({y0})-({y1}) sprite_h={sprite_h} zlow_top={} zlow={:.2} zbuf={:.2}",
                    kind.name(),
                    x1 - x0,
                    self.zlow_top[mid],
                    self.zlow[mid],
                    self.zbuf[mid],
                );
            }
            for x in x0..x1 {
                // ТЗ №2: две глубины на колонку. Полная стена ближе спрайта —
                // колонка закрыта целиком; низкая мебель ближе — спрайт
                // обрезается ровно по её крыше (за прилавком видно только
                // то, что выше его), а сам фасад «прострелить» нельзя.
                let xi = x as usize;
                let clip = if self.zbuf[xi] < ty {
                    0
                } else if self.zlow[xi] < ty {
                    self.zlow_top[xi]
                } else {
                    SCREEN_H as i32
                };
                let tex_x = ((x - x_left) * crate::sprites::SPR_W as i32
                    / sprite_w.max(1) as i32)
                    .clamp(0, crate::sprites::SPR_W as i32 - 1) as usize;
                for y in y0..y1 {
                    if y >= clip {
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
