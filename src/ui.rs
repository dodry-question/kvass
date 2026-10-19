//! UI «Квасной лавки» на RGBA-канвасе 1280×800.
//!
//! Стиль — советская лавка: тёмные эмалевые таблички с заклёпками, красные
//! ленты «КВАС», бумажные ведомости на экранах, янтарные цифры кассы.
//! Мир остаётся палитровым рейкастером 640×400 (нижний слой), а весь
//! интерфейс рисуется на прозрачном канвасе поверх: текст плотнопиксельный,
//! читаемый, у каждой надписи своя подложка — ничего не налезает на текстуры.

use crate::brewing::{BarrelPhase, Barrels, BrewQte, BARREL_PORTIONS, QTE_BONUS, seconds_to_next};
use crate::canvas::{Canvas, CV_H, CV_W};
use crate::day_cycle::{GameState, Phase, day_len};
use crate::font::{Font, SZ_H, SZ_L, SZ_M, SZ_S, SZ_T, SZ_XL};
use crate::inventory::{HandItem, Inventory};
use crate::map::{Map, Tile};
use crate::palette::PALETTE;
use crate::raycaster::{GameCam, HORIZON, SCREEN_H};
use crate::sprites::{PropIcon, Vessel, TRANSPARENT, prop_icon, vessel_icon};

use bevy::prelude::Vec2;

// --- Цвета (те же краски, что в палитре мира, как RGBA) ---

pub mod col {
    pub const KVASS_L: [u8; 3] = [232, 184, 75];
    pub const KVASS: [u8; 3] = [184, 115, 51];
    pub const KVASS_D: [u8; 3] = [107, 58, 20];
    pub const WOOD: [u8; 3] = [200, 159, 107];
    pub const WOOD_D: [u8; 3] = [122, 82, 48];
    pub const TILE: [u8; 3] = [154, 168, 154];
    pub const TILE_D: [u8; 3] = [92, 107, 92];
    pub const PLASTER: [u8; 3] = [201, 192, 168];
    pub const RED: [u8; 3] = [194, 46, 46];
    pub const RED_D: [u8; 3] = [122, 26, 26];
    pub const GREEN: [u8; 3] = [46, 107, 58];
    pub const METAL: [u8; 3] = [138, 143, 148];
    pub const METAL_D: [u8; 3] = [74, 78, 82];
    pub const WHITE: [u8; 3] = [232, 228, 216];
    pub const BLACK: [u8; 3] = [18, 14, 11];
    pub const SHAD: [u8; 3] = [36, 30, 22];
    pub const LAMP: [u8; 3] = [255, 232, 160];
    pub const INK: [u8; 3] = [26, 21, 17];
    pub const SKY: [u8; 3] = [135, 168, 201];
    pub const LEAVES: [u8; 3] = [74, 122, 58];
    pub const SKIN: [u8; 3] = [217, 160, 109];
    pub const SKIN_D: [u8; 3] = [170, 116, 79];
}

// --- Константы вёрстки ---

/// Нижняя полка HUD: y 672..800 (бывший BAR_Y=168 в дизайн-координатах, ×4).
pub const BAR_Y: i32 = 672;
/// Полоса уведомлений над полкой.
pub const MSG_Y: i32 = 628;
/// Полоска подсказки «что перед вами».
pub const HINT_Y: i32 = 580;

fn blink(t: f32, hz: f32) -> bool {
    (t * hz) as i32 % 2 == 0
}

// =================== Точка входа ===================

/// customers: (позиция в мире, текст облачка, если есть).
#[allow(clippy::too_many_arguments)]
pub fn draw_frame(
    canvas: &mut Canvas,
    font: &Font,
    state: &GameState,
    inv: &Inventory,
    barrels: &Barrels,
    qte: &BrewQte,
    customers: &[(Vec2, Option<String>)],
    t: f32,
    cam: &GameCam,
    map: &Map,
    moving: bool,
) {
    match state.phase {
        Phase::Menu => draw_menu(canvas, font, t),
        Phase::Shopping => draw_shopping(canvas, font, state, inv, t),
        Phase::DayEnd => draw_dayend(canvas, font, state, t),
        Phase::Final => draw_final(canvas, font, t),
        Phase::Paused => {
            let bubbles = bubble_layout(customers, font, cam);
            draw_hud(canvas, font, state, inv, barrels, qte, t, map, cam, &bubbles);
            canvas.dim(150);
            draw_pause(canvas, font, state);
        }
        Phase::Playing | Phase::Tutorial => {
            let bubbles = bubble_layout(customers, font, cam);
            // предмет в руках — под HUD-слоем, чтобы плашки перекрывали его низ
            if !qte.active {
                draw_viewmodel(canvas, inv, t, moving);
            }
            draw_hud(canvas, font, state, inv, barrels, qte, t, map, cam, &bubbles);
            draw_bubbles(canvas, font, &bubbles);
            if state.phase == Phase::Tutorial {
                draw_tutorial(canvas, font, state, customers, cam, t);
            }
        }
    }
}

// =================== Панели и утилиты ===================

/// Тёмная эмалевая табличка: жёсткая чёрная обводка, скошенные углы,
/// пиксельная тень — стиль DOS-меню, никаких «веб-плашек».
fn panel(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    // жёсткая пиксельная тень со смещением
    c.rect(x + 5, y + 6, w, h, [0, 0, 0], 190);
    // доска
    c.rect(x, y, w, h, col::SHAD, 255);
    // толстая чёрная обводка (2px)
    c.frame(x, y, w, h, [0, 0, 0], 255);
    c.frame(x + 1, y + 1, w - 2, h - 2, [0, 0, 0], 255);
    // скос: светлая грань сверху/слева, тёмная снизу/справа
    c.rect(x + 2, y + 2, w - 4, 2, col::TILE, 130);
    c.rect(x + 2, y + 2, 2, h - 4, col::TILE, 100);
    c.rect(x + 2, y + h - 4, w - 4, 2, [0, 0, 0], 170);
    c.rect(x + w - 4, y + 2, 2, h - 4, [0, 0, 0], 170);
    // срубленные пиксельные углы
    for (cx0, cy0, dxy) in [
        (x, y, 0i32),
        (x + w - 3, y, 1),
        (x, y + h - 3, 2),
        (x + w - 3, y + h - 3, 3),
    ] {
        let _ = dxy;
        for i in 0..2 {
            c.blend(cx0 + i, cy0, [0, 0, 0], 255);
            c.blend(cx0, cy0 + i, [0, 0, 0], 255);
        }
    }
    // заклёпки
    for (rx, ry) in [
        (x + 8, y + 8),
        (x + w - 12, y + 8),
        (x + 8, y + h - 12),
        (x + w - 12, y + h - 12),
    ] {
        c.rect(rx, ry, 4, 4, col::METAL, 255);
        c.blend(rx + 1, ry + 1, col::WHITE, 120);
        c.blend(rx + 3, ry + 3, [0, 0, 0], 140);
    }
}

/// Красная лента-вывеска в том же ретро-стиле.
fn red_plate(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.rect(x + 5, y + 6, w, h, [0, 0, 0], 190);
    c.rect(x, y, w, h, col::RED, 255);
    c.frame(x, y, w, h, [0, 0, 0], 255);
    c.frame(x + 1, y + 1, w - 2, h - 2, [0, 0, 0], 255);
    for i in 0..2 {
        c.blend(x + i, y, [0, 0, 0], 255);
        c.blend(x, y + i, [0, 0, 0], 255);
        c.blend(x + w - 1 - i, y + h - 1, [0, 0, 0], 255);
        c.blend(x + w - 1, y + h - 1 - i, [0, 0, 0], 255);
    }
    c.rect(x + 2, y + 2, w - 4, 2, [255, 220, 220], 70);
    c.rect(x + 2, y + h - 4, w - 4, 2, col::RED_D, 255);
}

/// Линейка прогресса.
fn bar(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, frac: f32, fill: [u8; 3]) {
    c.rect(x, y, w, h, [16, 13, 10], 230);
    c.frame(x, y, w, h, col::BLACK, 255);
    let wl = ((w - 2) as f32 * frac.clamp(0.0, 1.0)).max(0.0) as i32;
    if wl > 0 {
        c.rect(x + 1, y + 1, wl, h - 2, fill, 255);
        c.rect(x + 1, y + 1, wl, 1, col::WHITE, 70);
    }
}

/// Клавиша-кейкап; возвращает ширину.
fn keycap(c: &mut Canvas, font: &Font, x: i32, y: i32, h: i32, key: &str) -> i32 {
    let w = (font.w(key, SZ_S) + 22).max(34);
    c.rect(x + 2, y + 3, w, h, [0, 0, 0], 140);
    c.rect(x, y, w, h, col::TILE, 255);
    c.rect(x, y, w, 2, col::WHITE, 90);
    c.rect(x, y + h - 2, w, 2, col::TILE_D, 255);
    c.frame(x, y, w, h, col::INK, 255);
    let tx = x + (w - font.w(key, SZ_S)) / 2;
    font.draw(c, tx, y + (h - font.cap_h(SZ_S)) / 2, key, SZ_S, col::INK, 255);
    w
}

/// Иконка 16×16 из палитрового спрайтсета с масштабом.
fn blit_icon(c: &mut Canvas, icon: &[u8], x: i32, y: i32, scale: i32) {
    for iy in 0..16i32 {
        for ix in 0..16i32 {
            let t = icon[(iy * 16 + ix) as usize];
            if t == TRANSPARENT {
                continue;
            }
            let rgb = PALETTE[t as usize];
            for dy in 0..scale {
                for dx in 0..scale {
                    c.blend(x + ix * scale + dx, y + iy * scale + dy, rgb, 255);
                }
            }
        }
    }
}

/// Спрайт предмета руки (для слотов, склада и рук от первого лица).
fn item_icon(item: &HandItem) -> Vec<u8> {
    match item {
        HandItem::Croutons(_) => prop_icon(PropIcon::Croutons),
        HandItem::Sugar(_) => prop_icon(PropIcon::Sugar),
        HandItem::Bucket => prop_icon(PropIcon::Bucket),
        HandItem::Empty(v, _) => vessel_icon(*v, false),
        HandItem::Full(v, _, _) => vessel_icon(*v, true),
    }
}

/// Количество единиц в предмете руки.
fn item_count(item: &HandItem) -> u8 {
    match item {
        HandItem::Croutons(n) | HandItem::Sugar(n) | HandItem::Empty(_, n) => *n,
        HandItem::Full(_, n, _) => *n,
        HandItem::Bucket => 0,
    }
}
/// Бумажный лист «квитанции».
fn paper_panel(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.rect(x + 6, y + 6, w, h, [0, 0, 0], 120);
    c.rect(x, y, w, h, col::PLASTER, 255);
    c.frame(x, y, w, h, col::BLACK, 255);
    c.frame(x + 3, y + 3, w - 6, h - 6, col::RED, 120);
}

// =================== Игровой HUD ===================

struct Bubble {
    bx: i32,
    by: i32,
    w: i32,
    h: i32,
    screen_x: i32,
    lines: Vec<String>,
}

impl Bubble {
    fn hits(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.bx < x + w && self.bx + self.w > x && self.by < y + h && self.by + self.h > y
    }
}

fn bubble_layout(
    customers: &[(Vec2, Option<String>)],
    font: &Font,
    cam: &GameCam,
) -> Vec<Bubble> {
    let dir_x = cam.angle.cos();
    let dir_y = cam.angle.sin();
    let plane_x = -dir_y * 0.66;
    let plane_y = dir_x * 0.66;
    let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
    let ts = SZ_M;
    let mut out = Vec::new();
    for (pos, text) in customers {
        let Some(text) = text else { continue };
        let sx = pos.x - cam.x;
        let sy = pos.y - cam.y;
        let tx = inv_det * (dir_y * sx - dir_x * sy);
        let ty = inv_det * (-plane_y * sx + plane_x * sy);
        if ty <= 0.2 {
            continue;
        }
        let screen_x = ((CV_W as f32 / 2.0) * (1.0 + tx / ty)) as i32;
        // масштаб Head Up: спрайт рисуется высотой SCREEN_H/d*1.2 в буфере,
        // канвас — ×2 к буферу.
        let sprite_h = ((SCREEN_H as f32 / ty) * 1.2 * 2.0) as i32;
        let feet = ((HORIZON as f32 + (0.5 * SCREEN_H as f32) / ty) * 2.0) as i32;
        let top = feet - sprite_h;

        // перенос слов: строка ≤ 430 px
        let mut lines: Vec<String> = vec![String::new()];
        for word in text.split(' ') {
            let li = lines.len() - 1;
            let cand_w = font.w(&lines[li], ts) + font.w(word, ts) + font.w(" ", ts);
            if !lines[li].is_empty() && cand_w <= 430 {
                lines[li].push(' ');
                lines[li].push_str(word);
            } else if !lines[li].is_empty() {
                lines.push(String::from(word));
            } else {
                lines[li].push_str(word);
            }
        }
        let w = lines
            .iter()
            .map(|l| font.w(l, ts))
            .max()
            .unwrap_or(0)
            .max(font.w("И", ts))
            + 24;
        let h = lines.len() as i32 * font.line(ts) + 20;
        if screen_x + w < 0 || screen_x - w > CV_W {
            continue;
        }
        let bx = (screen_x - w / 2).clamp(8, CV_W - 8 - w);
        // не залезаем на панель дня/нормы в правом верхнем углу
        let panel_x0 = CV_W - 8 - 300;
        let by = top - h - 12;
        let bx = if by < 152 && bx + w > panel_x0 {
            (panel_x0 - w - 8).max(8)
        } else {
            bx
        };
        if by < 4 || top < 0 {
            continue;
        }
        out.push(Bubble {
            bx,
            by,
            w,
            h,
            screen_x,
            lines,
        });
    }
    out
}

fn draw_bubbles(c: &mut Canvas, font: &Font, bubbles: &[Bubble]) {
    let ts = SZ_M;
    for b in bubbles {
        c.rect(b.bx + 4, b.by + 4, b.w, b.h, [0, 0, 0], 110);
        c.rect(b.bx, b.by, b.w, b.h, col::WHITE, 250);
        c.frame(b.bx, b.by, b.w, b.h, col::BLACK, 255);
        // пиксельная стрелка-хвостик вниз, строго к макушке покупателя
        let tx = b.screen_x.clamp(b.bx + 12, b.bx + b.w - 12);
        for dy in 0..12 {
            let half = 5 - dy / 2; // сужается: 5 → 0
            c.rect(tx - half - 1, b.by + b.h + dy, half * 2 + 3, 1, col::BLACK, 255);
        }
        for dy in 0..10 {
            let half = 5 - dy / 2;
            c.rect(tx - half, b.by + b.h + dy, half * 2 + 1, 1, col::WHITE, 250);
        }
        let mut ly = b.by + 10;
        for l in &b.lines {
            font.draw(c, b.bx + 12, ly, l, ts, col::INK, 255);
            ly += font.line(ts);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hud(
    c: &mut Canvas,
    font: &Font,
    state: &GameState,
    inv: &Inventory,
    barrels: &Barrels,
    qte: &BrewQte,
    t: f32,
    map: &Map,
    cam: &GameCam,
    bubbles: &[Bubble],
) {
    // --- Таблички бочек (верхний левый угол) ---
    let mut keys: Vec<(i32, i32)> = barrels.map.keys().copied().collect();
    keys.sort();
    for (i, k) in keys.iter().enumerate() {
        let b = &barrels.map[k];
        let (led, text): ([u8; 3], String) = match b.phase {
            BarrelPhase::Empty => {
                let s = match b.step {
                    0 => format!("Б{} ПУСТАЯ", i + 1),
                    1 => format!("Б{} СУХАРИ+ВОДА", i + 1),
                    _ => format!("Б{} НУЖЕН САХАР", i + 1),
                };
                (col::TILE_D, s)
            }
            BarrelPhase::Ferment(p) => (col::KVASS_L, format!("Б{} БРОДИТ {}%", i + 1, p as i32)),
            BarrelPhase::Ready(p) => (
                if blink(t, 4.0) { col::LAMP } else { col::KVASS_L },
                format!(
                    "Б{} ГОТОВА {}/{}, {:.0}С",
                    i + 1,
                    b.portions,
                    BARREL_PORTIONS,
                    seconds_to_next(b)
                ),
            ),
            BarrelPhase::Overripe(p) => (
                if blink(t, 6.0) { col::LAMP } else { col::RED },
                format!("Б{} ПЕРЕБРОД {}% {:.0}С!", i + 1, p as i32, seconds_to_next(b)),
            ),
            BarrelPhase::Ruined => (col::RED, format!("Б{} ГРЯЗЬ! МОЙТЬ (E)", i + 1)),
        };
        let x = 8;
        let y = 8 + i as i32 * 46;
        let w = font.w(&text, SZ_S) + 56;
        if bubbles.iter().any(|b| b.hits(x, y, w, 40)) {
            continue; // облачко важнее
        }
        panel(c, x, y, w, 40);
        c.rect(x + 10, y + 15, 5, 5, led, 255);
        font.draw(c, x + 26, y + 9, &text, SZ_S, col::WHITE, 255);
        // тревожная рамка: последние проценты брожения
        if matches!(b.phase, BarrelPhase::Ferment(p) if p >= 90.0) && blink(t, 6.0) {
            c.frame(x - 3, y - 3, w + 6, 46, col::LAMP, 255);
        }
    }

    // --- День/время/норма (верхний правый угол) --- всегда видима
    {
        let pw = 300i32;
        let x = CV_W - 8 - pw;
        let y = 8;
        let ph = 136;
        panel(c, x, y, pw, ph);
        let day = format!("ДЕНЬ {}   НОРМА {} К.", state.day, state.norm);
        font.draw(c, x + 20, y + 12, &day, SZ_S, col::WHITE, 255);
        // ход времени
        let tp = (state.time_left / day_len(state.day)).clamp(0.0, 1.0);
        bar(c, x + 20, y + 42, pw - 40, 12, tp, col::KVASS);
        let mins = (state.time_left / 60.0) as i32;
        let secs = (state.time_left as i32) % 60;
        let to_end = format!("КОНЕЦ ДНЯ {:02}:{:02}", mins, secs);
        font.draw(c, x + 20, y + 58, &to_end, SZ_S, col::LAMP, 255);
        // норма
        let frac = (state.day_earned as f32 / state.norm.max(1) as f32).clamp(0.0, 1.0);
        let done = state.day_earned >= state.norm;
        bar(
            c,
            x + 20,
            y + 88,
            pw - 40,
            12,
            frac,
            if done { col::GREEN } else { col::KVASS_L },
        );
        let earned = format!("{} / {} К.", state.day_earned, state.norm);
        font.draw(
            c,
            x + 20,
            y + 104,
            &earned,
            SZ_S,
            if done { col::GREEN } else { col::TILE },
            255,
        );
    }

    // --- «Нужна посуда» (верх по центру, мигает) ---
    let posuda_nul = inv.stock_glass + inv.stock_jar + inv.stock_bottle == 0;
    if posuda_nul && state.phase == Phase::Playing && blink(t, 2.0) {
        let warn = "ПОСУДА КОНЧИЛАСЬ! ЗАКУПИТЕ (3) ЗАВТРА";
        let ww = font.w(warn, SZ_M) + 32;
        let x = (CV_W - ww) / 2;
        panel(c, x, 8, ww, 44);
        c.rect(x + 2, 10, ww - 4, 40, col::RED_D, 255);
        font.draw(c, x + 16, 17, warn, SZ_M, col::LAMP, 255);
    }

    // --- Статусбар снизу ---
    c.rect(0, BAR_Y, CV_W, CV_H - BAR_Y, col::SHAD, 255);
    c.rect(0, BAR_Y, CV_W, 4, col::WOOD_D, 255);
    c.rect(0, BAR_Y + 4, CV_W, 2, col::BLACK, 200);
    for y in (BAR_Y + 16..CV_H).step_by(28) {
        c.rect(0, y, CV_W, 1, col::WOOD_D, 30);
    }

    // Касса (левый блок)
    {
        let x = 16;
        font.draw(c, x, BAR_Y + 16, "КАССА", SZ_S, col::TILE, 255);
        let money = format!("{} К.", state.money);
        font.draw(c, x, BAR_Y + 46, &money, SZ_XL, col::LAMP, 255);
    }

    // Лик героя (Doom-style) — реагирует на события дня
    draw_face(c, t, state.face_mood);

    // Слоты рук (4)
    for i in 0..4 {
        let x = 316 + i as i32 * 96;
        let y = BAR_Y + 14;
        let selected = i == inv.selected;
        c.rect(x + 3, y + 4, 88, 88, [0, 0, 0], 130);
        c.rect(x, y, 88, 88, col::METAL_D, 255);
        let (fc, fn_) = if selected { (col::LAMP, 255u8) } else { (col::BLACK, 255u8) };
        for k in 0..3 {
            c.frame(x - k, y - k, 88 + k * 2, 88 + k * 2, fc, fn_);
        }
        font.draw(c, x + 6, y + 6, &format!("{}", i + 1), SZ_S, col::TILE, 200);
        let iy = y + 16;
        match &inv.hands[i] {
            None => {}
            Some(item) => {
                blit_icon(c, &item_icon(item), x + 20, iy, 3);
                put_count(c, font, x, y, &item_count(item));
                if let HandItem::Full(_, _, true) = item {
                    if blink(t, 6.0) {
                        c.frame(x + 2, y + 2, 84, 84, col::RED, 255);
                    }
                }
            }
        }
    }

    // Склад (правее слотов)
    {
        let x = 712;
        font.draw(c, x, BAR_Y + 16, "СКЛАД", SZ_S, col::TILE, 255);
        blit_icon(c, &prop_icon(PropIcon::Croutons), x, BAR_Y + 46, 2);
        put_stock(c, font, x + 40, BAR_Y + 52, inv.stock_croutons);
        blit_icon(c, &prop_icon(PropIcon::Sugar), x + 92, BAR_Y + 46, 2);
        put_stock(c, font, x + 126, BAR_Y + 52, inv.stock_sugar);

        let vessels = [
            (Vessel::Glass, inv.stock_glass),
            (Vessel::Jar, inv.stock_jar),
            (Vessel::Bottle, inv.stock_bottle),
        ];
        let mut ix = x + 190;
        for (v, n) in vessels {
            blit_icon(c, &vessel_icon(v, false), ix, BAR_Y + 44, 2);
            put_stock(c, font, ix + 4, BAR_Y + 76, n);
            ix += 78;
        }
    }

    // Пауза-подсказка (правый край полки)
    {
        let hint = "ESC - ПАУЗА";
        let x = CV_W - 16 - font.w(hint, SZ_S);
        font.draw(c, x, BAR_Y + 16, hint, SZ_S, col::TILE_D, 230);
    }

    // --- Строка уведомления ---
    if state.msg_t > 0.0 {
        let alpha: u8 = if state.msg_t < 0.7 {
            ((state.msg_t / 0.7) * 210.0) as u8
        } else {
            210
        };
        c.rect(0, MSG_Y, CV_W, BAR_Y - MSG_Y, [0, 0, 0], alpha);
        font.draw_fit(c, 16, MSG_Y + 6, CV_W - 32, &state.msg, SZ_M, col::LAMP, 255);
    }

    // --- Подсказка: на какой предмет смотришь ---
    let label: Option<(&str, &str)> = if state.phase == Phase::Tutorial {
        None
    } else {
        let fx = cam.x + cam.angle.cos() * 0.9;
        let fy = cam.y + cam.angle.sin() * 0.9;
        match map.get(fx.floor() as i32, fy.floor() as i32) {
            Tile::Croutons => Some(("E", "СУХАРИ - ВЗЯТЬ ГОРСТЬ")),
            Tile::Sugar => Some(("E", "САХАР С ИЗЮМОМ - ВЗЯТЬ")),
            Tile::Water => Some(("E", "КОЛОНКА - НАБРАТЬ ВЕДРО ВОДЫ")),
            Tile::Shelf => Some(("E", "СТЕЛЛАЖ - ВЗЯТЬ ПУСТУЮ ПОСУДУ")),
            Tile::Barrel => {
                if !qte.active {
                    Some(("E", "БОЧКА: ДЕРЖИТЕ E - РОЗЛИВ / РЕЦЕПТ"))
                } else {
                    None
                }
            }
            Tile::Counter => Some(("E", "ПРИЛАВОК - ОТДАТЬ КВАС ПОКУПАТЕЛЮ")),
            Tile::Doorway => Some(("E", "ДВЕРЬ НА УЛИЦУ")),
            _ => None,
        }
    };
    if let Some((key, text)) = label {
        let w = font.w(text, SZ_S) + 100;
        // слева, чтобы не под предметом в руках (viewmodel по центру)
        let x = 8;
        if !qte.active && !bubbles.iter().any(|b| b.hits(x, HINT_Y, w, 44)) {
            panel(c, x, HINT_Y, w, 44);
            let kx = keycap(c, font, x + 10, HINT_Y + 6, 32, key);
            font.draw(c, x + kx + 16, HINT_Y + 12, text, SZ_S, col::LAMP, 255);
        }
    }

    // --- Мини-игра ускорения брожения ---
    if qte.active {
        draw_qte(c, font, qte, barrels);
    }
}

/// Число предметов в слоте (правый нижний угол).
fn put_count(c: &mut Canvas, font: &Font, x: i32, y: i32, n: &u8) {
    let s = format!("{n}");
    font.draw(c, x + 58, y + 66, &s, SZ_S, col::WHITE, 255);
}

/// Число на складе; ноль — красным.
fn put_stock(c: &mut Canvas, font: &Font, x: i32, y: i32, n: u8) {
    let s = format!("{n}");
    let colr = if n == 0 { col::RED } else { col::WHITE };
    font.draw(c, x, y, &s, SZ_S, colr, 255);
}

/// Полоска ускорения брожения.
fn draw_qte(c: &mut Canvas, font: &Font, qte: &BrewQte, barrels: &Barrels) {
    let w = 640i32;
    let x = (CV_W - w) / 2;
    let y = MSG_Y - 122;
    let h = 96;

    panel(c, x, y, w, h);
    let pct = qte
        .barrel
        .and_then(|k| barrels.map.get(&k))
        .and_then(|b| match b.phase {
            BarrelPhase::Ferment(p) | BarrelPhase::Ready(p) | BarrelPhase::Overripe(p) => Some(p),
            _ => None,
        })
        .unwrap_or(0.0) as i32;
    let title = format!("БРОДИТ {pct}% - ПРОБЕЛ В ЗОНУ (+{}%)", QTE_BONUS as i32);
    font.draw_fit_sh(c, x + 10, y + 12, w - 20, &title, SZ_M, col::LAMP, 255);

    let bx = x + 24;
    let bw = w - 48;
    let by = y + 52;
    let bh = 24;
    c.rect(bx, by, bw, bh, col::METAL_D, 255);
    c.frame(bx, by, bw, bh, col::BLACK, 255);
    let z0 = bx + (bw as f32 * qte.zone0) as i32;
    let zw = (((bw as f32) * (qte.zone1 - qte.zone0)) as i32).max(6);
    let zone = if qte.flash > 0.0 { col::GREEN } else { col::KVASS_L };
    c.rect(z0, by + 2, zw, bh - 4, zone, 255);
    c.frame(z0, by + 2, zw, bh - 4, col::BLACK, 180);
    let rx = bx + ((bw - 10) as f32 * qte.pos) as i32;
    c.rect(rx, by - 4, 10, bh + 8, col::LAMP, 255);
    c.frame(rx, by - 4, 10, bh + 8, col::BLACK, 255);

    if qte.flash.abs() > 0.05 {
        let (text, color) = if qte.flash > 0.0 {
            (format!("+{}%", QTE_BONUS as i32), col::GREEN)
        } else {
            ("МИМО!".to_string(), col::RED)
        };
        let fw = font.w(&text, SZ_L) + 24;
        let fx = (CV_W - fw) / 2;
        let fy = y - 46;
        c.rect(fx, fy, fw, 40, col::SHAD, 235);
        c.frame(fx, fy, fw, 40, color, 255);
        font.draw_fit_sh(c, fx + 10, fy + 6, fw - 20, &text, SZ_L, color, 255);
    }
}

/// Обучение: панель инструкции и мигающий маркер на цель.
fn draw_tutorial(
    c: &mut Canvas,
    font: &Font,
    state: &GameState,
    customers: &[(Vec2, Option<String>)],
    cam: &GameCam,
    t: f32,
) {
    let step = state.tut_step as usize;
    let Some(ts) = crate::tutorial::TUT_STEPS.get(step) else {
        return;
    };
    let ts_size = SZ_M;
    let skip = "ПРОПУСТИТЬ";
    let w = (font.w(ts.text, ts_size).max(font.w(skip, SZ_S) + 70) + 72).max(520);
    let x = ((CV_W - w) / 2).max(8);
    let y = HINT_Y - 96;
    panel(c, x, y, w, 88);
    font.draw(c, x + 24, y + 14, ts.text, ts_size, col::LAMP, 255);
    let kx = keycap(c, font, x + 24, y + 52, 26, "T");
    font.draw(c, x + kx + 34, y + 56, skip, SZ_S, col::TILE, 255);

    // цель: на последнем шаге — сама бабушка
    let target = if step == 6 {
        customers.first().map(|(p, _)| (p.x, p.y))
    } else {
        Some(ts.target)
    };
    let Some((tx, ty)) = target else { return };

    let dir_x = cam.angle.cos();
    let dir_y = cam.angle.sin();
    let plane_x = -dir_y * 0.66;
    let plane_y = dir_x * 0.66;
    let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
    let sx = tx - cam.x;
    let sy = ty - cam.y;
    let px = inv_det * (dir_y * sx - dir_x * sy);
    let py = inv_det * (-plane_y * sx + plane_x * sy);
    if py <= 0.3 {
        if blink(t, 4.0) {
            let hint = "ПОВЕРНИТЕСЬ - ЦЕЛЬ ЗА СПИНОЙ!";
            let w = font.w(hint, SZ_M) + 32;
            let x = (CV_W - w) / 2;
            c.rect(x, 8, w, 44, col::RED_D, 255);
            c.frame(x, 8, w, 44, col::BLACK, 255);
            font.draw(c, x + 16, 17, hint, SZ_M, col::LAMP, 255);
        }
        return;
    }
    let screen_x = ((CV_W as f32 / 2.0) * (1.0 + px / py)) as i32;
    let sprite_h = (((SCREEN_H as f32 / py) * 1.2 * 2.0) as i32).min(320);
    let feet = ((HORIZON as f32 + (0.5 * SCREEN_H as f32) / py) * 2.0) as i32;
    let top = feet - sprite_h - 24;
    let cx = screen_x.clamp(40, CV_W - 40);
    let cy = top.clamp(32, 440);
    if blink(t, 4.0) {
        for row in 0..24 {
            let half = (21 - row) / 2;
            for dx in -half..=half {
                c.blend(cx + dx, cy + row, col::BLACK, 255);
            }
            let half2 = ((21 - row) - 2) / 2;
            if half2 > 0 && row < 20 {
                for dx in -half2..=half2 {
                    c.blend(cx + dx, cy + row, col::LAMP, 255);
                }
            }
        }
    }
}

// =================== Экраны ===================

fn draw_menu(c: &mut Canvas, font: &Font, t: f32) {
    // уличный фон
    c.rect(0, 0, CV_W, 240, col::SKY, 255);
    c.rect(0, 240, CV_W, 200, col::LEAVES, 255);
    c.rect(0, 440, CV_W, CV_H - 440, col::TILE_D, 255);
    for yy in (460..CV_H).step_by(56) {
        c.rect(0, yy, CV_W, 2, [70, 84, 70], 255);
    }

    // киоск с красно-белым тентом
    for r in 0..30 {
        let y = 208 + r * 4;
        let half = 110 + (r * 50) / 30;
        let stripe = if (r / 2) % 2 == 0 { col::RED } else { col::PLASTER };
        c.rect(640 - half - 40, y, half * 2 + 80, 4, stripe, 255);
        c.rect(640 - half - 40, y, 4, 4, col::BLACK, 255);
        c.rect(640 + half + 36, y, 4, 4, col::BLACK, 255);
    }
    // коробка киоска
    c.rect(480, 328, 320, 120, col::WOOD, 255);
    for i in 0..4 {
        c.rect(480, 344 + i * 24, 320, 6, col::WOOD_D, 255);
    }
    c.frame(480, 328, 320, 120, col::BLACK, 255);
    // вывеска «КВАС»
    red_plate(c, 552, 352, 176, 52);
    font.draw_fit_sh(c, 552, 364, 176, "КВАС", SZ_L, col::WHITE, 255);
    // бочка с краном
    c.rect(600, 456, 88, 84, col::WOOD, 255);
    c.rect(600, 476, 88, 6, col::WOOD_D, 255);
    c.rect(600, 512, 88, 6, col::WOOD_D, 255);
    c.frame(600, 456, 88, 84, col::BLACK, 255);
    c.rect(632, 544, 24, 20, col::METAL, 255);
    c.frame(632, 544, 24, 20, col::BLACK, 255);

    // титул
    red_plate(c, 60, 24, 1160, 130);
    font.draw_fit_sh(c, 60, 44, 1160, "КВАСНАЯ ЛАВКА", SZ_H, col::WHITE, 255);
    font.draw_fit_sh(c, 60, 122, 1160, "VYATKA OWLS GAME JAM", SZ_S, col::PLASTER, 240);

    // подсказки с кейкапами
    let hints: [(&str, &str); 4] = [
        ("ENTER", "НАЧАТЬ ИГРУ"),
        ("WASD", "ХОД, МЫШЬ - ОБЗОР"),
        ("E", "ДЕЙСТВИЕ, 1-4 - СЛОТЫ"),
        ("ESC", "ПАУЗА"),
    ];
    let mut hy = 576;
    for (k, text) in hints {
        let kw = (font.w(k, SZ_S) + 22).max(34);
        let row_w = kw + 24 + font.w(text, SZ_M);
        let x0 = (CV_W - row_w) / 2;
        let kx = keycap(c, font, x0, hy, 32, k);
        font.draw_sh(c, x0 + kx + 24, hy + 6, text, SZ_M, col::WHITE, 255);
        hy += 46;
    }
    if blink(t, 2.0) {
        let coin = "ВСТАВЬТЕ МОНЕТУ";
        let w = font.w(coin, SZ_M);
        font.draw_sh(c, (CV_W - w) / 2, 762, coin, SZ_M, col::LAMP, 255);
    }
}

fn draw_shopping(c: &mut Canvas, font: &Font, state: &GameState, inv: &Inventory, t: f32) {
    c.rect(0, 0, CV_W, CV_H, col::PLASTER, 255);
    // шапка
    c.rect(0, 0, CV_W, 100, col::RED, 255);
    c.rect(0, 100, CV_W, 4, col::RED_D, 255);
    font.draw(c, 48, 26, "ЗАКУПКА СЫРЬЯ", SZ_T, col::WHITE, 255);
    let day = format!("ДЕНЬ {}", state.day);
    font.draw(c, CV_W - 48 - font.w(&day, SZ_L), 36, &day, SZ_L, col::WHITE, 240);

    let money = format!("КАССА: {} К.", state.money);
    font.draw(c, 48, 126, &money, SZ_L, col::INK, 255);
    let norm = format!("НОРМА ДНЯ: {} К.", state.norm);
    font.draw(c, CV_W - 48 - font.w(&norm, SZ_L), 126, &norm, SZ_L, col::INK, 240);

    // три карточки товара
    const CARD_H: i32 = 136;
    for i in 0..3u32 {
        let y = 192 + (i as i32) * (CARD_H + 18);
        c.rect(48, y, CV_W - 96, CARD_H, col::WHITE, 255);
        c.frame(48, y, CV_W - 96, CARD_H, col::BLACK, 255);
        c.frame(52, y + 4, CV_W - 104, CARD_H - 8, col::RED, 60);
        let (key, name, desc) = match i {
            0 => ("1", "СУХАРИ РЖАНЫЕ", "10 ГОРСТЕЙ - 20 К."),
            1 => ("2", "САХАР С ИЗЮМОМ", "10 ПОРЦИЙ - 10 К."),
            _ => ("3", "ПОСУДА", "4 СТАКАНА + 4 БАНКИ + 4 БУТЫЛКИ - 15 К."),
        };
        // иконки
        match i {
            0 => {
                blit_icon(c, &prop_icon(PropIcon::Croutons), 88, y + 30, 3);
                blit_icon(c, &prop_icon(PropIcon::Croutons), 140, y + 54, 3);
            }
            1 => {
                blit_icon(c, &prop_icon(PropIcon::Sugar), 96, y + 30, 3);
                blit_icon(c, &prop_icon(PropIcon::Sugar), 148, y + 54, 3);
            }
            _ => {
                blit_icon(c, &vessel_icon(Vessel::Glass, false), 84, y + 24, 3);
                blit_icon(c, &vessel_icon(Vessel::Jar, false), 148, y + 24, 3);
                blit_icon(c, &vessel_icon(Vessel::Bottle, false), 212, y + 24, 3);
            }
        }
        font.draw(c, 296, y + 20, name, SZ_L, col::INK, 255);
        font.draw(c, 296, y + 66, desc, SZ_S, [80, 66, 52], 255);
        let have = match i {
            0 => format!("В ЗАПАСЕ: {}", inv.stock_croutons),
            1 => format!("В ЗАПАСЕ: {}", inv.stock_sugar),
            _ => format!(
                "В ЗАПАСЕ: {}",
                inv.stock_glass + inv.stock_jar + inv.stock_bottle
            ),
        };
        font.draw(c, 296, y + 98, &have, SZ_M, col::GREEN, 255);
        let _ = keycap(c, font, CV_W - 216, y + (CARD_H - 40) / 2, 40, key);
    }

    // подсказка
    let hint = if state.day == 1 {
        "НАЧАЛО: ЗАКУПИТЕ СУХАРИ (1), САХАР (2) И ПОСУДУ (3) - ИЛИ СРАЗУ ЖМИТЕ ENTER"
    } else {
        "СОВЕТ: ДЕРЖИТЕ ДВЕ БОЧКИ НА РАЗНЫХ ФАЗАХ - КВАС ГОТОВИТСЯ НЕ СРАЗУ"
    };
    font.draw(c, 48, CV_H - 128, hint, SZ_S, [100, 84, 66], 255);
    if blink(t, 2.0) {
        let go = "ENTER - ОТКРЫТЬ ЛАВКУ";
        let gw = font.w(go, SZ_L);
        let gx = (CV_W - gw) / 2;
        c.rect(gx - 26, CV_H - 92, gw + 52, 58, col::SHAD, 248);
        c.frame(gx - 26, CV_H - 92, gw + 52, 58, col::RED, 255);
        font.draw(c, gx, CV_H - 80, go, SZ_L, col::LAMP, 255);
    }
}

fn draw_dayend(c: &mut Canvas, font: &Font, state: &GameState, t: f32) {
    c.rect(0, 0, CV_W, CV_H, col::SHAD, 255);
    let px = 190i32;
    let pw = CV_W - 2 * px;
    paper_panel(c, px, 72, pw, 560);
    // красный штемпель
    c.rect(px + 32, 106, pw - 64, 4, col::RED, 255);
    font.draw_fit_sh(c, px + 40, 124, pw - 80, &format!("ВЕДОМОСТЬ ДНЯ {}", state.day), SZ_T, col::INK, 255);
    let ok = state.day_passed;
    let verdict = if ok { "ВЫПОЛНЕНО!" } else { "НЕ ВЫПОЛНЕНО!" };
    font.draw_fit_sh(
        c,
        px + 40,
        196,
        pw - 80,
        verdict,
        SZ_XL,
        if ok { col::GREEN } else { col::RED },
        255,
    );

    let rows = [
        ("ЗАРАБОТАНО ЗА ДЕНЬ", format!("{} К.", state.day_earned)),
        ("КАССА", format!("{} К.", state.money)),
        ("НОРМА ДНЯ", format!("{} К.", state.norm)),
        ("ШТРАФЫ", format!("{} К.", state.fine)),
    ];
    let mut y = 280i32;
    for (label, value) in rows {
        font.draw(c, px + 80, y, label, SZ_M, col::INK, 255);
        // пунктирная линейка между подписью и суммой
        let lx = px + 100 + font.w(label, SZ_M);
        let rx = px + pw - 120 - font.w(&value, SZ_L);
        let mut xx = lx + 16;
        while xx < rx {
            c.rect(xx, y + font.cap_h(SZ_M) - 6, 6, 2, col::INK, 110);
            xx += 18;
        }
        font.draw(c, rx, y - 4, &value, SZ_L, col::INK, 255);
        y += 60;
    }
    if state.fine > 0 {
        let fr = format!("ПРИЧИНА: {}", state.fine_reason);
        font.draw(c, px + 80, y + 4, &fr, SZ_S, col::RED_D, 255);
    }
    if !ok {
        font.draw(c, px + 80, 588, "НОРМА НЕ СДАНА. ЗАВТРА ТОЧНО ПОВЕЗЁТ...", SZ_S, [110, 70, 56], 255);
    }
    if blink(t, 2.0) {
        let go = if state.day >= 4 {
            "ENTER - ФИНАЛ: СЕТЬ ОТКРЫВАЕТСЯ"
        } else {
            "ENTER - СЛЕДУЮЩИЙ ДЕНЬ"
        };
        font.draw_fit_sh(c, 0, 656, CV_W, go, SZ_XL, col::LAMP, 255);
    }
}

fn draw_final(c: &mut Canvas, font: &Font, t: f32) {
    c.rect(0, 0, CV_W, CV_H, col::RED, 255);
    // флаг с флагштоком
    c.rect(744, 40, 8, 220, col::BLACK, 255);
    c.rect(732, 256, 32, 10, col::BLACK, 255);
    for i in 0..6 {
        let stripe = if i % 2 == 0 { col::LAMP } else { col::WHITE };
        c.rect(752, 52 + i * 28, 220, 28, stripe, 255);
    }
    c.frame(752, 52, 220, 168, col::BLACK, 255);
    star(c, 862, 96, 26, col::RED, 255);

    let lines = [
        ("СЕТЬ ОТКРЫТА!", SZ_T),
        ("ВЫ ОТКРЫЛИ СЕТЬ КВАСНЫХ ЛАВОК В ГОРОДЕ.", SZ_M),
        ("ВАШ КВАС СЛАВЕН НА ВСЮ ОКРУГУ.", SZ_M),
        ("СПАСИБО, ХОЗЯИН!", SZ_L),
    ];
    let mut y = 330i32;
    for (text, size) in lines {
        y += font.draw_fit_sh(c, 0, y, CV_W, text, size, col::WHITE, 255) + 12;
    }
    if blink(t, 2.0) {
        font.draw_fit_sh(c, 0, 712, CV_W, "ESC - В МЕНЮ", SZ_L, col::LAMP, 255);
    }
}

/// Пятиконечная звезда с заливкой.
fn star(c: &mut Canvas, cx: i32, cy: i32, r: i32, colr: [u8; 3], a: u8) {
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(10);
    for i in 0..10 {
        let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rr = if i % 2 == 0 { r as f32 } else { r as f32 * 0.42 };
        pts.push((cx as f32 + rr * ang.cos(), cy as f32 + rr * ang.sin()));
    }
    for i in 0..10 {
        line(c, pts[i], pts[(i + 1) % 10], colr, a);
    }
    let n = pts.len();
    for dy in -r..=r {
        for dx in -r..=r {
            let px = (cx + dx) as f32;
            let py = (cy + dy) as f32;
            let mut inside = false;
            let mut k = n - 1;
            for j in 0..n {
                let (x1, y1) = pts[k];
                let (x2, y2) = pts[j];
                if (y1 > py) != (y2 > py) {
                    let xin = (x2 - x1) * (py - y1) / (y2 - y1) + x1;
                    if px < xin {
                        inside = !inside;
                    }
                }
                k = j;
            }
            if inside {
                c.blend(cx + dx, cy + dy, colr, a);
            }
        }
    }
}

fn line(c: &mut Canvas, p0: (f32, f32), p1: (f32, f32), colr: [u8; 3], a: u8) {
    let steps = ((p1.0 - p0.0).abs() + (p1.1 - p0.1).abs()) as i32;
    let steps = steps.max(1);
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        c.blend(
            (p0.0 + f * (p1.0 - p0.0)) as i32,
            (p0.1 + f * (p1.1 - p0.1)) as i32,
            colr,
            a,
        );
    }
}

// =================== Меню паузы (ESC) ===================

fn draw_pause(c: &mut Canvas, font: &Font, state: &GameState) {
    let pw = 500i32;
    let x = (CV_W - pw) / 2;
    let y = 200;
    let h = 400;
    panel(c, x, y, pw, h);
    font.draw_fit_sh(c, x, y + 24, pw, "ПЕРЕРЫВ", SZ_T, col::LAMP, 255);
    font.draw_fit_sh(c, x + 20, y + 98, pw - 40, "ЛАВКА ЗАКРЫТА НА ОБЕД", SZ_S, col::TILE, 240);

    let items = ["ПРОДОЛЖИТЬ", "В ГЛАВНОЕ МЕНЮ", "ВЫХОД ИЗ ИГРЫ"];
    let mut iy = y + 156;
    for (i, item) in items.iter().enumerate() {
        let selected = i == state.pause_sel as usize;
        if selected {
            c.rect(x + 36, iy - 8, pw - 72, 46, col::RED_D, 255);
            // маркер-треугольник вправо
            for dx in 0..12 {
                let half = dx / 2;
                for dy in -half..=half {
                    c.blend(x + 52 + dx, iy + 12 + dy, col::LAMP, 255);
                }
            }
        }
        font.draw(
            c,
            x + 78,
            iy,
            item,
            SZ_L,
            if selected { col::LAMP } else { col::WHITE },
            255,
        );
        iy += 58;
    }
    font.draw_fit_sh(
        c,
        x + 20,
        y + h - 72,
        pw - 40,
        "W/S - ВЫБОР   ENTER - ОК",
        SZ_S,
        col::TILE,
        240,
    );
    font.draw_fit_sh(
        c,
        x + 20,
        y + h - 40,
        pw - 40,
        "ESC - ПРОДОЛЖИТЬ",
        SZ_S,
        col::TILE,
        240,
    );
}

// ============ предмет в руках от первого лица (Minecraft/Doom viewmodel) ============

/// Повёрнутый спрайт 16×16: nearest-сэмплинг по обратному повороту —
/// остаётся крупным пикселем, наклон как у предмета в руке в Minecraft.
fn blit_icon_rot(c: &mut Canvas, icon: &[u8], cx: i32, cy: i32, scale: f32, angle: f32) {
    let (sin, cos) = angle.sin_cos();
    let half = (16.0 * scale * (cos.abs() + sin.abs()) / 2.0) as i32;
    for oy in -half..=half {
        for ox in -half..=half {
            // обратный поворот: экранный пиксель → пиксель спрайта
            let fx = ox as f32 * cos + oy as f32 * sin;
            let fy = -ox as f32 * sin + oy as f32 * cos;
            let sx = (fx / scale + 7.5).round() as i32;
            let sy = (fy / scale + 7.5).round() as i32;
            if sx < 0 || sy < 0 || sx >= 16 || sy >= 16 {
                continue;
            }
            let ci = icon[(sy * 16 + sx) as usize];
            if ci == TRANSPARENT {
                continue;
            }
            c.blend(cx + ox, cy + oy, PALETTE[ci as usize], 255);
        }
    }
}

/// Выбранный предмет в правом нижнем углу: крупный, под углом −18°,
/// покачивается при ходьбе, в покое слегка дышит.
fn draw_viewmodel(c: &mut Canvas, inv: &Inventory, t: f32, moving: bool) {
    let Some(item) = &inv.hands[inv.selected] else {
        return;
    };
    let icon = item_icon(item);
    let (bob, sway) = if moving {
        (6.0 * (t * 6.1).sin(), 7.0 * (t * 3.1).sin())
    } else {
        (2.5 * (t * 2.2).sin(), 2.5 * (t * 1.7).sin())
    };
    let cx = CV_W - 200 + sway as i32;
    let cy = 560 + bob as i32;
    blit_icon_rot(c, &icon, cx, cy, 8.0, -0.32);
}

// ============ лик героя (Doom-style face в нижней панели) ============

/// Лицо продавца-пилотки: смотрит чуть в сторону, моргает; реагирует:
/// 0 — спокойно, 1 — улыбка (купил/попал), 2 — хмурая (мимо/нет товара).
fn draw_face(c: &mut Canvas, t: f32, mood: u8) {
    let x = 236;
    let y = BAR_Y + 26;
    let s = 60i32;
    // плашка под лицом
    c.rect(x + 3, y + 4, s, s, [0, 0, 0], 130);
    c.rect(x, y, s, s, col::METAL_D, 255);
    c.frame(x, y, s, s, col::BLACK, 255);
    let fx = x + 7;
    let fy = y + 8;
    // лицо (скруглённое)
    for row in 0..44 {
        let inx = if row < 4 || row >= 40 { 6 } else { 2 };
        c.rect(fx + inx, fy + row, 44 - inx * 2, 1, col::SKIN, 255);
    }
    // уши
    c.rect(fx, fy + 20, 4, 10, col::SKIN, 255);
    c.rect(fx + 40, fy + 20, 4, 10, col::SKIN, 255);
    // пилотка квасника: белый колпак с козырьком
    c.rect(fx + 2, fy - 2, 40, 10, col::WHITE, 255);
    c.rect(fx, fy + 6, 44, 5, col::METAL, 255);
    c.frame(fx, fy - 2, 44, 13, col::BLACK, 255);
    c.rect(fx + 8, fy + 1, 8, 3, col::RED, 255); // красная звёздочка-крапинка
    // брови
    let brow_dy = if mood == 2 { 2 } else { 0 };
    c.rect(fx + 8, fy + 15 - brow_dy, 10, 2, col::BLACK, 255);
    c.rect(fx + 26, fy + 15 - brow_dy, 10, 2, col::BLACK, 255);
    if mood == 2 {
        // хмурые сходящиеся брови
        c.rect(fx + 16, fy + 17, 4, 2, col::BLACK, 255);
        c.rect(fx + 24, fy + 17, 4, 2, col::BLACK, 255);
    }
    // глаза (моргают)
    let blinking = (t % 3.4) < 0.12;
    let eye_y = fy + 21;
    for ex in [fx + 9, fx + 27] {
        if blinking {
            c.rect(ex, eye_y + 2, 8, 2, col::BLACK, 255);
        } else {
            c.rect(ex, eye_y, 8, 6, col::WHITE, 255);
            let px = if mood == 1 { ex + 5 } else { ex + 2 };
            c.rect(px, eye_y + 1, 4, 4, col::INK, 255);
        }
    }
    // нос
    c.rect(fx + 20, fy + 22, 4, 8, col::SKIN_D, 255);
    // рот по настроению
    match mood {
        1 => {
            // улыбка
            c.rect(fx + 13, fy + 34, 18, 2, col::BLACK, 255);
            c.rect(fx + 11, fy + 32, 3, 2, col::BLACK, 255);
            c.rect(fx + 30, fy + 32, 3, 2, col::BLACK, 255);
        }
        2 => {
            // скривился
            c.rect(fx + 13, fy + 33, 18, 2, col::BLACK, 255);
            c.rect(fx + 15, fy + 35, 6, 2, col::BLACK, 255);
            c.rect(fx + 25, fy + 31, 6, 2, col::BLACK, 255);
            // капля пота
            c.rect(fx + 36, fy + 18, 3, 6, col::SKY, 255);
            c.rect(fx + 37, fy + 24, 2, 3, col::SKY, 255);
        }
        _ => {
            // спокойно
            c.rect(fx + 14, fy + 33, 16, 2, col::BLACK, 255);
        }
    }
    // щёки и подбородок
    if mood == 0 {
        c.blend(fx + 8, fy + 30, col::RED, 60);
        c.blend(fx + 32, fy + 30, col::RED, 60);
    }
    // лёгкая щетина
    for (sx, sy) in [(12, 38), (20, 40), (28, 38), (16, 41), (32, 40)] {
        c.blend(fx + sx, fy + sy, col::METAL_D, 140);
    }
}

