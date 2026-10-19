//! HUD (Doom-статусбар), облачка диалогов, экраны меню — всё рисуется
//! прямо в палитровом буфере 320x200.

use crate::brewing::{ferment_progress, BarrelPhase, BarrelSignal, Barrels};
use crate::customer::Customer;
use crate::day_cycle::{GameState, Phase};
use crate::font::{frame_rect, put, rect, Font};
use crate::inventory::{HandItem, Inventory};
use crate::palette::idx;
use crate::raycaster::{GameCam, Renderer, SCREEN_H, SCREEN_W, UI_H, UI_HORIZON, UI_W};
use crate::sprites::{vessel_icon, NpcKind, Vessel, TRANSPARENT};

pub const BAR_Y: i32 = 168;

pub fn draw_frame(
    renderer: &mut Renderer,
    font: &Font,
    state: &GameState,
    inv: &Inventory,
    barrels: &Barrels,
    customers: &CustomerView<'_>,
    t: f32,
    cam: &GameCam,
    map: &crate::map::Map,
) {
    match state.phase {
        Phase::Menu => draw_menu(&mut renderer.indices, font, t),
        Phase::Shopping => draw_shopping(&mut renderer.indices, font, state, inv, t),
        Phase::DayEnd => draw_dayend(&mut renderer.indices, font, state, t),
        Phase::Final => draw_final(&mut renderer.indices, font, t),
        Phase::Tutorial => {
            let bubbles = bubble_layout(customers, font, cam);
            draw_hud(&mut renderer.indices, font, state, inv, barrels, t, map, cam, &bubbles);
            draw_bubbles(&mut renderer.indices, font, &bubbles);
            draw_tutorial(&mut renderer.indices, font, state, customers, cam, t);
        }
        Phase::Playing => {
            let bubbles = bubble_layout(customers, font, cam);
            draw_hud(&mut renderer.indices, font, state, inv, barrels, t, map, cam, &bubbles);
            draw_bubbles(&mut renderer.indices, font, &bubbles);
        }
    }
}

/// Список покупателей для рендера/облачков.
pub struct CustomerView<'a> {
    pub list: &'a [CustomerRender],
}

pub struct CustomerRender {
    pub kind: NpcKind,
    pub pos: Vec2,
    pub bubble: Option<String>,
}

use bevy::prelude::Vec2;

/// Рассчитанное облачко речи (его прямоугольник) — чтобы HUD мог его пропустить.
struct Bubble {
    bx: i32,
    by: i32,
    w: i32,
    h: i32,
    screen_x: i32,
    lines: [String; 2],
}

impl Bubble {
    /// Пересекаются ли прямоугольники.
    fn hits(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.bx < x + w && self.bx + self.w > x && self.by < y + h && self.by + self.h > y
    }
}

fn bubble_layout(view: &CustomerView<'_>, font: &Font, cam: &GameCam) -> Vec<Bubble> {
    let dir_x = cam.angle.cos();
    let dir_y = cam.angle.sin();
    let plane_x = -dir_y * 0.66;
    let plane_y = dir_x * 0.66;
    let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
    let mut out = Vec::new();
    for c in view.list.iter() {
        let Some(text) = &c.bubble else { continue };
        let sx = c.pos.x - cam.x;
        let sy = c.pos.y - cam.y;
        let tx = inv_det * (dir_y * sx - dir_x * sy);
        let ty = inv_det * (-plane_y * sx + plane_x * sy);
        if ty <= 0.2 {
            continue;
        }
        let screen_x = ((UI_W as f32 / 2.0) * (1.0 + tx / ty)) as i32;
        // верх головы — как у спрайта в рендере (масштаб 1.2, кламп как в raycaster)
        let sprite_h = (UI_H as f32 / ty * 1.2).min(160.0) as i32;
        let feet = UI_HORIZON + (0.5 * UI_H as f32 / ty) as i32;
        let top = feet - sprite_h;
        // перенос текста на 2 строки
        let max_chars = 26;
        let words: Vec<&str> = text.split(' ').collect();
        let mut lines = [String::new(), String::new()];
        for w in words {
            let li = if lines[0].is_empty() {
                0
            } else if lines[0].chars().count() + w.chars().count() + 1 <= max_chars
                && lines[1].is_empty()
            {
                0
            } else {
                1
            };
            if !lines[li].is_empty() {
                lines[li].push(' ');
            }
            lines[li].push_str(w);
        }
        let w = font.width(&lines[0]).max(font.width(&lines[1])) as i32 + 8;
        let h = if lines[1].is_empty() { 13 } else { 25 };
        if screen_x < -w || screen_x > UI_W + w {
            continue;
        }
        // прижимаем облачко к краям экрана, чтобы текст не уходил за кадр
        let bx = (screen_x - w / 2).clamp(0, UI_W - w);
        let by = top - h - 4;
        if by < 0 {
            continue;
        }
        out.push(Bubble { bx, by, w, h, screen_x, lines });
    }
    out
}

/// Облачко речи над головой + текст.
fn draw_bubbles(buf: &mut [u8], font: &Font, bubbles: &[Bubble]) {
    for b in bubbles {
        // облачко
        rect(buf, SCREEN_W, SCREEN_H, b.bx, b.by, b.w, b.h, idx::WHITE);
        frame_rect(buf, SCREEN_W, SCREEN_H, b.bx, b.by, b.w, b.h, idx::BLACK);
        // хвостик к голове
        put(buf, SCREEN_W, SCREEN_H, b.screen_x - 1, b.by + b.h, idx::BLACK);
        put(buf, SCREEN_W, SCREEN_H, b.screen_x, b.by + b.h + 1, idx::BLACK);
        put(buf, SCREEN_W, SCREEN_H, b.screen_x + 1, b.by + b.h, idx::BLACK);
        font.draw(buf, SCREEN_W, SCREEN_H, b.bx + 4, b.by + 2, &b.lines[0], idx::BLACK, None);
        if !b.lines[1].is_empty() {
            font.draw(buf, SCREEN_W, SCREEN_H, b.bx + 4, b.by + 14, &b.lines[1], idx::BLACK, None);
        }
    }
}

fn draw_hud(
    buf: &mut [u8],
    font: &Font,
    state: &GameState,
    inv: &Inventory,
    barrels: &Barrels,
    t: f32,
    map: &crate::map::Map,
    cam: &GameCam,
    bubbles: &[Bubble],
) {
    // --- Индикаторы бочек (сверху слева) ---
    let mut keys: Vec<(i32, i32)> = barrels.map.keys().copied().collect();
    keys.sort();
    for (i, k) in keys.iter().enumerate() {
        let b = &barrels.map[k];
        let x = 2;
        let y = 2 + i as i32 * 14;
        let (bg, text): (u8, String) = match b.phase {
            BarrelPhase::Empty => {
                let s = match b.step {
                    0 => format!("Б{} ПУСТА", i + 1),
                    1 => format!("Б{} +СУХАРИ", i + 1),
                    _ => format!("Б{} +ВОДА", i + 1),
                };
                (idx::METAL_DARK, s)
            }
            BarrelPhase::Ferment(_) => {
                let p = (ferment_progress(b).unwrap_or(0.0) * 100.0) as i32;
                (idx::KVASS_MID, format!("Б{} БРОДИТ {}%", i + 1, p))
            }
            BarrelPhase::Ready(sec) => {
                let pulse = if (t * 4.0) as i32 % 2 == 0 { true } else { false };
                let bg = if pulse { idx::GREEN } else { idx::KVASS_LIGHT };
                (bg, format!("Б{} ГОТОВА {}/20 {:.0}", i + 1, b.portions, sec))
            }
            BarrelPhase::Overripe(sec) => {
                let pulse = (t * 6.0) as i32 % 2 == 0;
                let bg = if pulse { idx::RED } else { idx::RED_DARK };
                (bg, format!("Б{} ПЕРЕБРОД {:.0}!", i + 1, sec))
            }
            BarrelPhase::Ruined => (idx::BLACK, format!("Б{} ГРЯЗЬ! МОЙТЕ", i + 1)),
        };
        let w = font.width(&text) as i32 + 6;
        // облачко речи важнее: если оно накрыло индикатор — не рисуем индикатор
        if bubbles.iter().any(|b| b.hits(x, y, w, 13)) {
            continue;
        }
        rect(buf, SCREEN_W, SCREEN_H, x, y, w, 13, bg);
        frame_rect(buf, SCREEN_W, SCREEN_H, x, y, w, 13, idx::BLACK);
        font.draw(
            buf,
            SCREEN_W,
            SCREEN_H,
            x + 3,
            y + 2,
            &text,
            if bg == idx::BLACK { idx::RED } else { idx::WHITE },
            Some(idx::BLACK),
        );
        // мигание иконки при AlmostReady / переброде
        if matches!(b.phase, BarrelPhase::Ferment(tt) if tt <= 10.0) && (t * 6.0) as i32 % 2 == 0
        {
            frame_rect(buf, SCREEN_W, SCREEN_H, x - 1, y - 1, w + 2, 15, idx::LAMP);
        }
    }

    // --- Статусбар снизу ---
    for y in BAR_Y..UI_H {
        for x in 0..UI_W {
            let c = if (y - BAR_Y) % 4 == 0 {
                idx::WOOD_DARK
            } else {
                idx::WOOD_LIGHT
            };
            put(buf, SCREEN_W, SCREEN_H, x, y, c);
        }
    }
    rect(buf, SCREEN_W, SCREEN_H, 0, BAR_Y, UI_W, 1, idx::BLACK);

    // щиток «КВАС»
    rect(buf, SCREEN_W, SCREEN_H, 2, BAR_Y + 3, 30, 12, idx::RED);
    frame_rect(buf, SCREEN_W, SCREEN_H, 2, BAR_Y + 3, 30, 12, idx::BLACK);
    font.draw_scaled(buf, SCREEN_W, SCREEN_H, 5, BAR_Y + 5, "КВАС", idx::WHITE, Some(idx::BLACK), 1);

    // касса и норма
    font.draw(
        buf,
        SCREEN_W,
        SCREEN_H,
        36,
        BAR_Y + 2,
        &format!("{}К", state.money),
        idx::LAMP,
        Some(idx::BLACK),
    );
    font.draw(
        buf,
        SCREEN_W,
        SCREEN_H,
        36,
        BAR_Y + 12,
        &format!("НА {}К", state.norm),
        idx::WHITE,
        Some(idx::BLACK),
    );
    // полоска выполнения нормы
    let p = (state.day_earned as f32 / state.norm as f32).clamp(0.0, 1.0);
    rect(buf, SCREEN_W, SCREEN_H, 36, BAR_Y + 23, 40, 4, idx::SHADOW);
    rect(buf, SCREEN_W, SCREEN_H, 36, BAR_Y + 23, (40.0 * p) as i32, 4, idx::KVASS_LIGHT);

    // слоты рук (4)
    for i in 0..4 {
        let x = 90 + i as i32 * 22;
        let selected = i == inv.selected;
        rect(buf, SCREEN_W, SCREEN_H, x, BAR_Y + 3, 20, 26, idx::SHADOW);
        frame_rect(
            buf,
            SCREEN_W,
            SCREEN_H,
            x,
            BAR_Y + 3,
            20,
            26,
            if selected { idx::LAMP } else { idx::BLACK },
        );
        font.draw(buf, SCREEN_W, SCREEN_H, x + 7, BAR_Y + 21, &format!("{}", i + 1), idx::WHITE, Some(idx::BLACK));
        match &inv.hands[i] {
            None => {}
            Some(item) => match item {
                HandItem::Bucket => draw_bucket(buf, x + 2, BAR_Y + 5),
                HandItem::Croutons(n) => {
                    draw_croutons(buf, x + 2, BAR_Y + 5);
                    font.draw(buf, SCREEN_W, SCREEN_H, x + 13, BAR_Y + 5, &format!("{}", n), idx::WHITE, Some(idx::BLACK));
                }
                HandItem::Sugar(n) => {
                    draw_sugar(buf, x + 2, BAR_Y + 5);
                    font.draw(buf, SCREEN_W, SCREEN_H, x + 13, BAR_Y + 5, &format!("{}", n), idx::WHITE, Some(idx::BLACK));
                }
                HandItem::Empty(v, n) => {
                    blit_icon(buf, &vessel_icon(*v, false), x + 2, BAR_Y + 5);
                    font.draw(buf, SCREEN_W, SCREEN_H, x + 13, BAR_Y + 5, &format!("{}", n), idx::WHITE, Some(idx::BLACK));
                }
                HandItem::Full(v, n, over) => {
                    blit_icon(buf, &vessel_icon(*v, true), x + 2, BAR_Y + 5);
                    font.draw(buf, SCREEN_W, SCREEN_H, x + 13, BAR_Y + 5, &format!("{}", n), idx::WHITE, Some(idx::BLACK));
                    if *over && (t * 6.0) as i32 % 2 == 0 {
                        frame_rect(buf, SCREEN_W, SCREEN_H, x + 1, BAR_Y + 4, 18, 17, idx::RED);
                    }
                }
            },
        }
    }

    // время дня / обучение без времени
    if state.phase == Phase::Tutorial {
        font.draw(
            buf,
            SCREEN_W,
            SCREEN_H,
            186,
            BAR_Y + 3,
            "ОБУЧЕНИЕ",
            idx::WHITE,
            Some(idx::BLACK),
        );
        font.draw(
            buf,
            SCREEN_W,
            SCREEN_H,
            186,
            BAR_Y + 3 + font.line_at(1),
            &format!("ШАГ {}/7", state.tut_step + 1),
            idx::LAMP,
            Some(idx::BLACK),
        );
    } else {
        font.draw(
            buf,
            SCREEN_W,
            SCREEN_H,
            186,
            BAR_Y + 3,
            &format!("ДЕНЬ {}", state.day),
            idx::WHITE,
            Some(idx::BLACK),
        );
        let tp = (state.time_left / crate::day_cycle::day_len(state.day)).clamp(0.0, 1.0);
        let bar2_y = BAR_Y + 3 + font.line_at(1);
        rect(buf, SCREEN_W, SCREEN_H, 186, bar2_y, 50, 6, idx::SHADOW);
        rect(
            buf,
            SCREEN_W,
            SCREEN_H,
            186,
            bar2_y,
            (50.0 * tp) as i32,
            6,
            idx::KVASS_MID,
        );
        frame_rect(buf, SCREEN_W, SCREEN_H, 186, bar2_y, 50, 6, idx::BLACK);
        font.draw(
            buf,
            SCREEN_W,
            SCREEN_H,
            186,
            bar2_y + 8,
            &format!("КОНЕЦ {}М", (state.time_left / 60.0) as i32 + 1),
            idx::LAMP,
            Some(idx::BLACK),
        );
    }

    // склад (справа в статусбаре): строки складываются по метрикам
    let stock_y = BAR_Y + 2;
    font.draw(
        buf,
        SCREEN_W,
        SCREEN_H,
        240,
        stock_y,
        &format!("СКЛАД {} {}", inv.stock_croutons, inv.stock_sugar),
        idx::WHITE,
        Some(idx::BLACK),
    );
    font.draw(
        buf,
        SCREEN_W,
        SCREEN_H,
        240,
        stock_y + font.line_at(1),
        &format!("ПОС {} {} {}", inv.stock_glass, inv.stock_jar, inv.stock_bottle),
        idx::WHITE,
        Some(idx::BLACK),
    );

    // --- Строка диалога/уведомления над статусбаром ---
    rect(buf, SCREEN_W, SCREEN_H, 0, BAR_Y - 11, UI_W, 11, idx::SHADOW);
    let line = if state.msg_t > 0.0 {
        Some(state.msg.as_str())
    } else {
        None
    };
    if let Some(text) = line {
        let text: String = text.chars().take(44).collect();
        font.draw(buf, SCREEN_W, SCREEN_H, 3, BAR_Y - 10, &text, idx::LAMP, Some(idx::BLACK));
    }

    // --- Подсказка: на какой предмет смотришь ---
    // (в обучении её заменяет панель шага — не дублируем и не налегаем)
    let fx = cam.x + cam.angle.cos() * 0.9;
    let fy = cam.y + cam.angle.sin() * 0.9;
    use crate::map::Tile;
    let label: Option<&str> = if state.phase == Phase::Tutorial {
        None
    } else {
        match map.get(fx.floor() as i32, fy.floor() as i32) {
            Tile::Croutons => Some("СУХАРИ - НАЖМИТЕ E"),
            Tile::Sugar => Some("САХАР С ИЗЮМОМ - НАЖМИТЕ E"),
            Tile::Water => Some("КОЛОНКА С ВОДОЙ - НАЖМИТЕ E"),
            Tile::Shelf => Some("ПУСТАЯ ПОСУДА - НАЖМИТЕ E"),
            Tile::Barrel => Some("БОЧКА: E - РЕЦЕПТ, ДЕРЖИТЕ E - РОЗЛИВ"),
            Tile::Counter => Some("ПРИЛАВОК: E - ОТДАТЬ ПОКУПАТЕЛЮ"),
            Tile::Doorway => Some("ДВЕРЬ НА УЛИЦУ"),
            _ => None,
        }
    };
    if let Some(text) = label {
        let w = font.width(text) as i32 + 8;
        let x = ((UI_W - w) / 2).max(2);
        // пузырь речи важнее подсказки предмета
        if !bubbles.iter().any(|b| b.hits(x, BAR_Y - 24, w, 13)) {
            rect(buf, SCREEN_W, SCREEN_H, x, BAR_Y - 24, w, 13, idx::SHADOW);
            frame_rect(buf, SCREEN_W, SCREEN_H, x, BAR_Y - 24, w, 13, idx::WOOD_DARK);
            font.draw(
                buf,
                SCREEN_W,
                SCREEN_H,
                x + 4,
                BAR_Y - 22,
                text,
                idx::LAMP,
                Some(idx::BLACK),
            );
        }
    }
}

/// Обучение: панель инструкции сверху и мигающий маркер-стрелка на цель.
fn draw_tutorial(
    buf: &mut [u8],
    font: &Font,
    state: &GameState,
    view: &CustomerView<'_>,
    cam: &GameCam,
    t: f32,
) {
    let step = state.tut_step as usize;
    let Some(ts) = crate::tutorial::TUT_STEPS.get(step) else { return };

    // панель инструкции внизу (над статусбаром), внутри — пропуск
    let skip = "T - ПРОПУСТИТЬ ОБУЧЕНИЕ";
    let w = font
        .width(ts.text)
        .max(font.width(skip))
        .max(120) as i32
        + 12;
    let x = ((UI_W - w) / 2).max(2);
    rect(buf, SCREEN_W, SCREEN_H, x, BAR_Y - 50, w, 28, idx::SHADOW);
    frame_rect(buf, SCREEN_W, SCREEN_H, x, BAR_Y - 50, w, 28, idx::LAMP);
    font.draw(buf, SCREEN_W, SCREEN_H, x + 6, BAR_Y - 47, ts.text, idx::LAMP, Some(idx::BLACK));
    font.draw(buf, SCREEN_W, SCREEN_H, x + 6, BAR_Y - 35, skip, idx::WHITE, Some(idx::BLACK));

    // цель маркера: на последнем шаге — сама бабушка
    let target = if step == 6 {
        view.list.first().map(|c| (c.pos.x, c.pos.y))
    } else {
        Some(ts.target)
    };
    if let Some((tx, ty)) = target {
        // проекция (как у облачек)
        let dir_x = cam.angle.cos();
        let dir_y = cam.angle.sin();
        let plane_x = -dir_y * 0.66;
        let plane_y = dir_x * 0.66;
        let inv_det = 1.0 / (plane_x * dir_y - dir_x * plane_y);
        let sx = tx - cam.x;
        let sy = ty - cam.y;
        let px = inv_det * (dir_y * sx - dir_x * sy);
        let py = inv_det * (-plane_y * sx + plane_x * sy);
        if std::env::var("KVASS_DEBUG_SPRITES").is_ok() {
            eprintln!("marker step={step} target=({tx},{ty}) px={px:.2} py={py:.2}");
        }
        if py > 0.3 {
            let screen_x =
                ((UI_W as f32 / 2.0) * (1.0 + px / py)) as i32;
            let sprite_h = (UI_H as f32 / py * 1.2).min(160.0) as i32;
            let feet = UI_HORIZON + (0.5 * UI_H as f32 / py) as i32;
            let top = feet - sprite_h - 10;
            let cx = screen_x.clamp(10, UI_W - 11);
            let cy = top.clamp(16, BAR_Y - 56);
            if (t * 4.0) as i32 % 2 == 0 {
                // мигающая стрелка вниз на цель
                for row in 0..8 {
                    let half = 7 - row;
                    for dx in -half..=half {
                        put(buf, SCREEN_W, SCREEN_H, cx + dx, cy + row, idx::BLACK);
                    }
                }
                for row in 1..7 {
                    let half = 6 - row;
                    for dx in -half..=half {
                        put(buf, SCREEN_W, SCREEN_H, cx + dx, cy + row, idx::LAMP);
                    }
                }
            }
        } else {
            // цель за спиной — велим повернуться (справа сверху, мимо HUD)
            let hint = "ПОВЕРНИТЕСЬ - ЦЕЛЬ ЗА СПИНОЙ!";
            let hw = font.width(hint) as i32;
            let hx = (UI_W - hw - 4).max(2);
            rect(buf, SCREEN_W, SCREEN_H, hx - 4, 2, hw + 8, 13, idx::SHADOW);
            if (t * 4.0) as i32 % 2 == 0 {
                frame_rect(buf, SCREEN_W, SCREEN_H, hx - 4, 2, hw + 8, 13, idx::RED);
                font.draw(buf, SCREEN_W, SCREEN_H, hx, 4, hint, idx::RED, Some(idx::BLACK));
            }
        }
    }
}

fn blit_icon(buf: &mut [u8], icon: &[u8], x: i32, y: i32) {
    for iy in 0..16 {
        for ix in 0..16 {
            let c = icon[iy * 16 + ix];
            if c != TRANSPARENT {
                put(buf, SCREEN_W, SCREEN_H, x + ix as i32, y + iy as i32, c);
            }
        }
    }
}

fn draw_bucket(buf: &mut [u8], x: i32, y: i32) {
    rect(buf, SCREEN_W, SCREEN_H, x + 1, y + 3, 12, 11, idx::METAL);
    frame_rect(buf, SCREEN_W, SCREEN_H, x + 1, y + 3, 12, 11, idx::BLACK);
    rect(buf, SCREEN_W, SCREEN_H, x + 3, y + 1, 8, 2, idx::METAL_DARK);
}

fn draw_croutons(buf: &mut [u8], x: i32, y: i32) {
    rect(buf, SCREEN_W, SCREEN_H, x + 2, y + 6, 10, 7, idx::WOOD_DARK);
    frame_rect(buf, SCREEN_W, SCREEN_H, x + 2, y + 6, 10, 7, idx::BLACK);
    put(buf, SCREEN_W, SCREEN_H, x + 4, y + 8, idx::WOOD_LIGHT);
    put(buf, SCREEN_W, SCREEN_H, x + 8, y + 10, idx::WOOD_LIGHT);
}

fn draw_sugar(buf: &mut [u8], x: i32, y: i32) {
    rect(buf, SCREEN_W, SCREEN_H, x + 3, y + 4, 9, 10, idx::PLASTER);
    frame_rect(buf, SCREEN_W, SCREEN_H, x + 3, y + 4, 9, 10, idx::BLACK);
    put(buf, SCREEN_W, SCREEN_H, x + 5, y + 7, idx::KVASS_LIGHT);
    put(buf, SCREEN_W, SCREEN_H, x + 9, y + 9, idx::KVASS_LIGHT);
}

// --- Экраны ---

fn clear(buf: &mut [u8], c: u8) {
    buf.fill(c);
}

fn draw_menu(buf: &mut [u8], font: &Font, t: f32) {
    clear(buf, idx::SHADOW);
    // «улица»: небо + зелень
    for y in 0..120i32 {
        for x in 0..UI_W {
            let c = if y < 70 {
                idx::SKY
            } else {
                idx::LEAVES
            };
            put(buf, SCREEN_W, SCREEN_H, x, y, c);
        }
    }
    rect(buf, SCREEN_W, SCREEN_H, 0, 120, UI_W, 80, idx::TILE_DARK);

    // --- Титул: одна строка, центрируется и сама подбирает масштаб под
    //     ширину экрана. Высота считается по метрикам шрифта, поэтому
    //     подзаголовок встаёт ровно под титулом — наложения нет. ---
    let mut ty = 6i32;
    ty += font.draw_fit(
        buf,
        SCREEN_W,
        SCREEN_H,
        4,
        ty,
        UI_W - 8,
        "КВАСНАЯ ЛАВКА",
        idx::RED,
        Some(idx::BLACK),
        4,
    );
    font.draw_fit(
        buf,
        SCREEN_W,
        SCREEN_H,
        4,
        ty,
        UI_W - 8,
        "VYATKA OWLS GAME JAM",
        idx::WHITE,
        Some(idx::BLACK),
        1,
    );

    // киоск с красно-белой крышей-тентом
    for r in 0..31 {
        let y = 66 + r;
        let half = 2 + (r * 42) / 30;
        let c = if (r / 2) % 2 == 0 { idx::RED } else { idx::PLASTER };
        for x in (160 - half)..=(160 + half) {
            put(buf, SCREEN_W, SCREEN_H, x, y, c);
        }
        put(buf, SCREEN_W, SCREEN_H, 160 - half, y, idx::BLACK);
        put(buf, SCREEN_W, SCREEN_H, 160 + half, y, idx::BLACK);
    }
    // коробка киоска
    rect(buf, SCREEN_W, SCREEN_H, 120, 97, 80, 45, idx::WOOD_LIGHT);
    frame_rect(buf, SCREEN_W, SCREEN_H, 120, 97, 80, 45, idx::BLACK);
    for i in 0..4 {
        rect(buf, SCREEN_W, SCREEN_H, 120, 105 + i * 8, 80, 2, idx::WOOD_DARK);
    }
    // вывеска «КВАС»
    rect(buf, SCREEN_W, SCREEN_H, 138, 100, 44, 11, idx::RED);
    frame_rect(buf, SCREEN_W, SCREEN_H, 138, 100, 44, 11, idx::BLACK);
    font.draw(buf, SCREEN_W, SCREEN_H, 144, 102, "КВАС", idx::WHITE, Some(idx::BLACK));
    // бочка с краном на прилавке киоска
    rect(buf, SCREEN_W, SCREEN_H, 150, 118, 20, 18, idx::WOOD_LIGHT);
    frame_rect(buf, SCREEN_W, SCREEN_H, 150, 118, 20, 18, idx::BLACK);
    rect(buf, SCREEN_W, SCREEN_H, 150, 123, 20, 2, idx::WOOD_DARK);
    rect(buf, SCREEN_W, SCREEN_H, 150, 130, 20, 2, idx::WOOD_DARK);
    rect(buf, SCREEN_W, SCREEN_H, 157, 136, 6, 5, idx::METAL);
    // титульные подсказки: складываются по измеренной высоте строки,
    // каждая центрируется — не наезжают и не вылезают за нижний край
    let hints = [
        ("ENTER - НАЧАТЬ (ОБУЧЕНИЕ)", idx::LAMP),
        ("WASD - ХОД, E - ДЕЙСТВИЕ", idx::WHITE),
        ("1-4 - СЛОТЫ, ESC - ПАУЗА", idx::WHITE),
    ];
    let mut hy = 150i32;
    for (text, c) in hints {
        hy += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, hy, UI_W - 8, text, c, Some(idx::BLACK), 1);
    }
    if (t * 2.0) as i32 % 2 == 0 {
        font.draw_fit(
            buf,
            SCREEN_W,
            SCREEN_H,
            4,
            hy,
            UI_W - 8,
            "INSERT COIN",
            idx::LAMP,
            Some(idx::BLACK),
            1,
        );
    }
}

fn draw_shopping(buf: &mut [u8], font: &Font, state: &GameState, inv: &Inventory, t: f32) {
    clear(buf, idx::SHADOW);
    // шапка: заголовок и строки складываются по измеренной высоте —
    // ничего не наезжает друг на друга
    let mut y = 6i32;
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, "СКЛАД", idx::KVASS_LIGHT, Some(idx::BLACK), 2);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, &format!("ДЕНЬ {}", state.day), idx::WHITE, Some(idx::BLACK), 1);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, &format!("КАССА {}К", state.money), idx::LAMP, Some(idx::BLACK), 1);
    let lines = [
        ("1 - СУХАРИ РЖАНЫЕ", "10 ГОРСТЕЙ - 20К", inv.stock_croutons),
        ("2 - САХАР+ИЗЮМ", "10 ПОРЦИЙ - 10К", inv.stock_sugar),
        ("3 - ПОСУДА", "4+4+4 ШТ - 15К", inv.stock_glass + inv.stock_jar + inv.stock_bottle),
    ];
    for (i, (a, b, n)) in lines.iter().enumerate() {
        let y = 56 + i as i32 * 24;
        rect(buf, SCREEN_W, SCREEN_H, 30, y - 3, 260, 24, idx::CEILING);
        frame_rect(buf, SCREEN_W, SCREEN_H, 30, y - 3, 260, 24, idx::WOOD_DARK);
        // пиктограмма товара слева
        match i {
            0 => draw_croutons(buf, 38, y - 2),
            1 => draw_sugar(buf, 38, y - 4),
            _ => {
                blit_icon(buf, &vessel_icon(Vessel::Glass, false), 36, y - 4);
                blit_icon(buf, &vessel_icon(Vessel::Jar, false), 54, y - 4);
                blit_icon(buf, &vessel_icon(Vessel::Bottle, false), 72, y - 4);
            }
        }
        let tx = if i == 2 { 94 } else { 62 };
        font.draw(buf, SCREEN_W, SCREEN_H, tx, y, a, idx::WHITE, Some(idx::BLACK));
        font.draw(buf, SCREEN_W, SCREEN_H, tx, y + 11, b, idx::TILE, Some(idx::BLACK));
        font.draw(buf, SCREEN_W, SCREEN_H, 240, y + 5, &format!("ЕСТЬ {}", n), idx::LAMP, Some(idx::BLACK));
    }
    let hint = if state.day == 1 {
        "НАЧАЛО: СУХАРИ 2, САХАР 2, ПОСУДА 4+4+1"
    } else {
        "СОВЕТ: ДЕРЖИТЕ 2 БОЧКИ НА РАЗНЫХ ФАЗАХ"
    };
    font.draw(buf, SCREEN_W, SCREEN_H, 30, 142, hint, idx::TILE, Some(idx::BLACK));
    if (t * 2.0) as i32 % 2 == 0 {
        font.draw_scaled(
            buf,
            SCREEN_W,
            SCREEN_H,
            56,
            158,
            "ENTER - ОТКРЫТЬ ЛАВКУ",
            idx::LAMP,
            Some(idx::BLACK),
            1,
        );
    }
}

fn draw_dayend(buf: &mut [u8], font: &Font, state: &GameState, t: f32) {
    clear(buf, idx::SHADOW);
    let ok = state.day_passed;
    let verdict = if ok { "ВЫПОЛНЕНО!" } else { "НЕ ВЫПОЛНЕНО!" };
    let col = if ok { idx::KVASS_LIGHT } else { idx::RED };
    // шапка складывается по метрикам — заголовок и вердикт не наезжают
    let mut y = 16i32;
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, &format!("ДЕНЬ {}", state.day), idx::WHITE, Some(idx::BLACK), 2);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, verdict, col, Some(idx::BLACK), 2);
    y += 2;
    let rows = [
        ("ЗАРАБОТАНО ЗА ДЕНЬ", format!("{} К.", state.day_earned)),
        ("КАССА", format!("{} К.", state.money)),
        ("НОРМА ДНЯ", format!("{} К.", state.norm)),
        ("ШТРАФЫ", format!("{} К.", state.fine)),
    ];
    let row_h = font.line_at(1);
    for (a, b) in rows {
        font.draw(buf, SCREEN_W, SCREEN_H, 50, y, a, idx::WHITE, Some(idx::BLACK));
        font.draw(buf, SCREEN_W, SCREEN_H, 220, y, &b, idx::LAMP, Some(idx::BLACK));
        y += row_h;
    }
    if !ok {
        font.draw_fit(
            buf,
            SCREEN_W,
            SCREEN_H,
            4,
            y + 2,
            UI_W - 8,
            "НОРМА НЕ СДАНА. ЗАВТРА ТОЧНО ПОВЕЗЁТ...",
            idx::TILE,
            Some(idx::BLACK),
            1,
        );
    }
    if (t * 2.0) as i32 % 2 == 0 {
        font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, 150, UI_W - 8, "ENTER - СЛЕДУЮЩИЙ ДЕНЬ", idx::LAMP, Some(idx::BLACK), 1);
    }
}

fn draw_final(buf: &mut [u8], font: &Font, t: f32) {
    clear(buf, idx::RED);
    // флаг: чёрный шест и полосатое полотнище
    rect(buf, SCREEN_W, SCREEN_H, 0, 0, UI_W, 60, idx::RED);
    rect(buf, SCREEN_W, SCREEN_H, 148, 8, 2, 38, idx::BLACK);
    rect(buf, SCREEN_W, SCREEN_H, 146, 42, 6, 3, idx::BLACK);
    for i in 0..6 {
        let c = if i % 2 == 0 { idx::LAMP } else { idx::WHITE };
        rect(buf, SCREEN_W, SCREEN_H, 150, 10 + i * 4, 24, 4, c);
    }
    frame_rect(buf, SCREEN_W, SCREEN_H, 150, 10, 24, 24, idx::BLACK);
    // текст складывается по метрикам шрифта — строки идут одна под
    // другой без наложения и одинакового кегля
    let mut y = 66i32;
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, "СЕТЬ ОТКРЫТА!", idx::LAMP, Some(idx::BLACK), 3);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, "ВЫ ОТКРЫЛИ СЕТЬ КВАСНЫХ ЛАВОК!", idx::WHITE, Some(idx::BLACK), 1);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, "КВАС СЛАВЕН НА ВЕСЬ ГОРОД.", idx::WHITE, Some(idx::BLACK), 1);
    y += font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y, UI_W - 8, "СПАСИБО, ХОЗЯИН!", idx::LAMP, Some(idx::BLACK), 1);
    if (t * 2.0) as i32 % 2 == 0 {
        font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, y + 2, UI_W - 8, "ESC - В МЕНЮ", idx::WHITE, Some(idx::BLACK), 1);
    }
}
