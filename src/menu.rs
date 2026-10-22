//! Главное меню поверх обоев (assets/menu_bg.png).
//!
//! На фоне — пиксельные обои лавки, на них колонка кнопок:
//! ПРОДОЛЖИТЬ · ВХОД · ОБУЧЕНИЕ · НАСТРОЙКИ · ВЫХОД.
//!
//! - Слот «ПРОДОЛЖИТЬ» (0) занимает место всегда, но появляется только когда
//!   есть прогресс (`GameState::has_progress`) — так «оставлено место» под
//!   кнопку и раскладка не прыгает.
//! - ВХОД при первом запуске открывает окно с настоятельной рекомендацией
//!   пройти обучение (`MenuView::Advice`).
//! - ОБУЧЕНИЕ запускает обучение отдельно от основного цикла и БЕЗ
//!   возможности пропуска (`tut_locked`).
//!
//! Ввод — мышь (наведение/клик; курсор в меню всегда свободен) и клавиатура
//! (стрелки/W-S — выбор, ENTER/SPACE/E — активация). Раскладка задана здесь
//! один раз и используется рисованием (ui::draw_menu) и вводом.

use std::sync::OnceLock;

use bevy::audio::{GlobalVolume, Volume};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::canvas::{CV_H, CV_W};
use crate::day_cycle::{
    give_start_package, start_day, GameState, MenuView, PAUSE_FROM_TUTORIAL, Phase,
};

// =================== Раскладка кнопок ===================

/// Кнопка главного меню.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Btn {
    Continue,
    Enter,
    Tutorial,
    Settings,
    Exit,
}

impl Btn {
    pub fn label(self) -> &'static str {
        match self {
            Btn::Continue => "ПРОДОЛЖИТЬ",
            Btn::Enter => "ВХОД",
            Btn::Tutorial => "ОБУЧЕНИЕ",
            Btn::Settings => "НАСТРОЙКИ",
            Btn::Exit => "ВЫХОД",
        }
    }
}

pub const BTN_X: i32 = 150;
pub const BTN_W: i32 = 430;
pub const BTN_H: i32 = 62;
pub const BTN_GAP: i32 = 16;
pub const BTN_Y0: i32 = 268;
/// Слоты сверху вниз: 0 всегда резервируется под «ПРОДОЛЖИТЬ».
pub const SLOTS: [Btn; 5] = [Btn::Continue, Btn::Enter, Btn::Tutorial, Btn::Settings, Btn::Exit];

/// Верх кнопки в её слоте.
pub fn slot_y(slot: usize) -> i32 {
    BTN_Y0 + slot as i32 * (BTN_H + BTN_GAP)
}

/// Прямоугольник слота (x, y, w, h).
pub fn btn_rect(slot: usize) -> (i32, i32, i32, i32) {
    (BTN_X, slot_y(slot), BTN_W, BTN_H)
}

/// Видимая кнопка: порядковый индекс в меню (`vis` = `GameState::menu_sel`)
/// и слот на экране (`slot` — позиция, при «ПРОДОЛЖИТЬ» без прогресса
/// пропущен, но место под ним остаётся).
pub struct Entry {
    pub vis: usize,
    pub slot: usize,
    pub btn: Btn,
}

/// Раскладка видимых кнопок в порядке выбора с клавиатуры.
pub fn layout(has_continue: bool) -> Vec<Entry> {
    let mut out = Vec::with_capacity(SLOTS.len());
    for (slot, btn) in SLOTS.iter().enumerate() {
        if *btn == Btn::Continue && !has_continue {
            continue;
        }
        out.push(Entry {
            vis: out.len(),
            slot,
            btn: *btn,
        });
    }
    out
}

/// Какая видимая кнопка под курсором (или None).
pub fn hit(lay: &[Entry], mx: i32, my: i32) -> Option<usize> {
    lay.iter()
        .find(|e| inside(btn_rect(e.slot), mx, my))
        .map(|e| e.vis)
}

fn inside(r: (i32, i32, i32, i32), mx: i32, my: i32) -> bool {
    mx >= r.0 && mx < r.0 + r.2 && my >= r.1 && my < r.1 + r.3
}

// --- Экран настроек: панель и элементы (канвас-координаты) ---

pub const SET_PANEL: (i32, i32, i32, i32) = (390, 190, 500, 420);
pub const SET_MINUS: (i32, i32, i32, i32) = (436, 330, 56, 56);
pub const SET_PLUS: (i32, i32, i32, i32) = (788, 330, 56, 56);
pub const SET_BAR: (i32, i32, i32, i32) = (508, 344, 268, 28);
pub const SET_BACK: (i32, i32, i32, i32) = (515, 492, 250, 64);

// --- Окно рекомендации обучения (перед ВХОДом) ---

pub const ADV_PANEL: (i32, i32, i32, i32) = (270, 160, 740, 480);
pub const ADV_TRAIN: (i32, i32, i32, i32) = (316, 528, 320, 64);
pub const ADV_SKIP: (i32, i32, i32, i32) = (656, 528, 308, 64);

// =================== Обои ===================

pub struct MenuBg {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

static BG: OnceLock<Option<MenuBg>> = OnceLock::new();

/// Обои меню (RGBA 8 бит). PNG встроен прямо в exe (как шрифт):
/// главное меню с картинкой работает из любого каталога запуска,
/// независимо от того, что лежит рядом с exe.
pub fn bg() -> &'static Option<MenuBg> {
    BG.get_or_init(|| {
        let img = image::load_from_memory(include_bytes!("../assets/menu_bg.png"))
            .ok()?
            .into_rgba8();
        let (w, h) = img.dimensions();
        Some(MenuBg {
            w: w as usize,
            h: h as usize,
            px: img.into_raw(),
        })
    })
}

// =================== Ввод ===================

/// Курсор в координатах канваса UI. Кадр — не всё окно, а область 4:3
/// (см. main::stage_43): курсор в чёрных полосках — мимо кнопок.
fn cursor_canvas(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<(i32, i32)> {
    let w = windows.single().ok()?;
    let p = w.cursor_position()?;
    let (sx, sy, sw, sh) = crate::stage_43(w.resolution.width(), w.resolution.height());
    if p.x < sx || p.y < sy || p.x >= sx + sw || p.y >= sy + sh {
        return None;
    }
    Some((
        ((p.x - sx) / sw * CV_W as f32) as i32,
        ((p.y - sy) / sh * CV_H as f32) as i32,
    ))
}

fn sfx(commands: &mut Commands, bank: &Option<Res<crate::audio::SoundBank>>, name: &str, vol: f32) {
    if let Some(b) = bank {
        crate::audio::play_sfx(commands, b, name, vol);
    }
}

/// ВХОД: начать основной игровой цикл с первого дня.
fn start_game(
    state: &mut GameState,
    inv: &mut crate::inventory::Inventory,
    barrels: &mut crate::brewing::Barrels,
    dirty: &mut crate::day_cycle::Dirty,
) {
    give_start_package(state, inv);
    start_day(state, barrels, dirty);
    state.menu_view = MenuView::Main;
    state.menu_sel = 0;
}

/// Обучение из меню: всегда без пропуска (T); `after_game` — после обучения
/// сразу начать день (рекомендация) или вернуться в меню (кнопка ОБУЧЕНИЕ).
fn start_tutorial(
    state: &mut GameState,
    inv: &mut crate::inventory::Inventory,
    after_game: bool,
) {
    give_start_package(state, inv);
    state.phase = Phase::Tutorial;
    state.tut_step = 0;
    state.tut_locked = true;
    state.tut_after_game = after_game;
    state.menu_view = MenuView::Main;
    state.menu_sel = 0;
    state.notify("ОБУЧЕНИЕ. W A S D - ХОД, МЫШЬ - ОБЗОР".into());
}

/// Ввод меню: мышь + клавиатура. Работает только в `Phase::Menu`.
pub fn menu_input(
    time: Res<Time>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<crate::inventory::Inventory>,
    mut barrels: ResMut<crate::brewing::Barrels>,
    mut dirty: ResMut<crate::day_cycle::Dirty>,
    mut gvol: ResMut<GlobalVolume>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut commands: Commands,
    bank: Option<Res<crate::audio::SoundBank>>,
) {
    if state.phase != Phase::Menu {
        return;
    }
    // курсор в меню всегда свободен (игровой захват — только в мире)
    for mut c in &mut cursors {
        if c.grab_mode != CursorGrabMode::None || !c.visible {
            c.grab_mode = CursorGrabMode::None;
            c.visible = true;
        }
    }
    // уведомления («ОБУЧЕНИЕ ПРОЙДЕНО») гаснут и на экране меню
    if state.msg_t > 0.0 {
        state.msg_t = (state.msg_t - time.delta_secs()).max(0.0);
    }

    let cur = cursor_canvas(&windows);
    let clicked = mouse.just_pressed(MouseButton::Left);

    match state.menu_view {
        // ---------- Главное меню ----------
        MenuView::Main => {
            let lay = layout(state.has_progress);
            let n = lay.len().max(1) as u8;
            if state.menu_sel >= n {
                state.menu_sel = 0;
            }
            // наведение курсора = выбор пункта
            let mut hover = None;
            if let Some((mx, my)) = cur {
                hover = hit(&lay, mx, my);
                if let Some(i) = hover {
                    state.menu_sel = i as u8;
                }
            }
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                state.menu_sel = (state.menu_sel + n - 1) % n;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                state.menu_sel = (state.menu_sel + 1) % n;
            }
            let mut act = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE);
            if clicked && hover.is_some() {
                state.menu_sel = hover.unwrap() as u8;
                act = true;
            }
            if act {
                let btn = lay[state.menu_sel as usize].btn;
                sfx(&mut commands, &bank, "ding", 0.85);
                keys.clear(); // ТЗ №1: клавиша не протекает в другие системы
                match btn {
                    Btn::Continue => {
                        // возобновить то, откуда ушли в паузе/меню
                        state.phase = if state.pause_prev == PAUSE_FROM_TUTORIAL {
                            Phase::Tutorial
                        } else {
                            Phase::Playing
                        };
                        state.menu_view = MenuView::Main;
                    }
                    Btn::Enter => {
                        if state.tut_done {
                            start_game(&mut state, &mut inv, &mut barrels, &mut dirty);
                        } else {
                            // в начале игры — окно с рекомендацией пройти обучение
                            state.menu_view = MenuView::Advice;
                            state.menu_sel = 0;
                        }
                    }
                    Btn::Tutorial => start_tutorial(&mut state, &mut inv, false),
                    Btn::Settings => {
                        state.menu_view = MenuView::Settings;
                        state.menu_sel = 0;
                    }
                    Btn::Exit => std::process::exit(0),
                }
            }
        }
        // ---------- Окно рекомендации обучения ----------
        MenuView::Advice => {
            let rects = [ADV_TRAIN, ADV_SKIP];
            let mut hover = None;
            if let Some((mx, my)) = cur {
                hover = rects.iter().position(|&r| inside(r, mx, my));
                if let Some(i) = hover {
                    state.menu_sel = i as u8;
                }
            }
            if keys.just_pressed(KeyCode::ArrowLeft)
                || keys.just_pressed(KeyCode::ArrowUp)
                || keys.just_pressed(KeyCode::KeyA)
            {
                state.menu_sel = 0;
            }
            if keys.just_pressed(KeyCode::ArrowRight)
                || keys.just_pressed(KeyCode::ArrowDown)
                || keys.just_pressed(KeyCode::KeyD)
            {
                state.menu_sel = 1;
            }
            if state.menu_sel > 1 {
                state.menu_sel = 0;
            }
            if keys.just_pressed(KeyCode::Escape) {
                // отмена — ни в игру, ни в обучение
                state.menu_view = MenuView::Main;
                state.menu_sel = 0;
                keys.clear();
                return;
            }
            let mut act = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE);
            if clicked && hover.is_some() {
                state.menu_sel = hover.unwrap() as u8;
                act = true;
            }
            if act {
                sfx(&mut commands, &bank, "ding", 0.85);
                keys.clear();
                if state.menu_sel == 0 {
                    start_tutorial(&mut state, &mut inv, true);
                } else {
                    start_game(&mut state, &mut inv, &mut barrels, &mut dirty);
                }
            }
        }
        // ---------- Настройки ----------
        MenuView::Settings => {
            let mut delta = 0i32;
            let mut hover: Option<u8> = None;
            if let Some((mx, my)) = cur {
                if inside(SET_MINUS, mx, my) || inside(SET_PLUS, mx, my) {
                    hover = Some(0);
                    if clicked {
                        delta = if inside(SET_MINUS, mx, my) { -1 } else { 1 };
                    }
                } else if inside(SET_BACK, mx, my) {
                    hover = Some(1);
                }
            }
            if let Some(i) = hover {
                state.menu_sel = i;
            }
            if keys.just_pressed(KeyCode::ArrowUp)
                || keys.just_pressed(KeyCode::KeyW)
                || keys.just_pressed(KeyCode::ArrowDown)
                || keys.just_pressed(KeyCode::KeyS)
            {
                // два пункта: 0 — громкость, 1 — назад
                state.menu_sel = if state.menu_sel == 0 { 1 } else { 0 };
            }
            if state.menu_sel == 0 {
                if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
                    delta = -1;
                }
                if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
                    delta = 1;
                }
            }
            if delta != 0 {
                let v = ((state.menu_vol as i32 + delta).clamp(0, 10)) as u8;
                if v != state.menu_vol {
                    state.menu_vol = v;
                    *gvol = GlobalVolume::new(Volume::Linear(v as f32 / 10.0));
                    sfx(&mut commands, &bank, "ding", 0.9);
                }
            }
            let back = keys.just_pressed(KeyCode::Escape)
                || (state.menu_sel == 1
                    && (keys.just_pressed(KeyCode::Enter)
                        || keys.just_pressed(KeyCode::NumpadEnter)
                        || keys.just_pressed(KeyCode::Space)
                        || keys.just_pressed(KeyCode::KeyE)));
            if back {
                state.menu_view = MenuView::Main;
                state.menu_sel = 0;
                keys.clear();
            }
        }
    }
}
