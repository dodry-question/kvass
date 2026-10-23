//! Отладочная консоль: физическая клавиша ` (Backquote / ё) в меню и в игре.
//!
//! Пока консоль открыта, весь игровой мир, музыка и звуки стоят. Команды:
//! `day 1`, `day 2`, `day 3` — перейти на день; `stop` — остановить время;
//! `play` — продолжить. `play` без активного `stop` ничего не делает.
//!
//! Текст команд читается из `KeyboardInput::text`, поэтому в консоли можно
//! печатать как в обычном поле ввода (латиница; команды английские).

use bevy::ecs::message::MessageReader;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::ambush::{BanditAmbush, BanditControl};
use crate::brewing::{Barrels, BrewQte, FirstOverbrew};
use crate::customer::Customer;
use crate::day_cycle::{
    give_start_package, start_day, norm_for, Dirty, GameState, MAX_DAY, PAUSE_FROM_PLAYING,
};
use crate::interact::{CoolQte, FillState};
use crate::inventory::Inventory;
use crate::modals::Modals;
use crate::raycaster::GameCam;

#[derive(Resource)]
pub struct DebugConsole {
    pub open: bool,
    pub input: String,
    pub pending: Option<String>,
    /// Последние сообщения/команды (снизу вверх в оверлее).
    pub log: Vec<String>,
    /// `stop` активен: игровое время, NPC и бочки стоят, игрок может ходить.
    pub time_stopped: bool,
}

impl Default for DebugConsole {
    fn default() -> Self {
        Self {
            open: false,
            input: String::new(),
            pending: None,
            log: Vec::new(),
            time_stopped: false,
        }
    }
}

impl DebugConsole {
    pub fn log(&mut self, text: impl Into<String>) {
        self.log.push(text.into());
        if self.log.len() > 5 {
            self.log.remove(0);
        }
    }
}

/// Открыть/закрыть консоль. `Backquote` — физическая клавиша, не зависит
/// от раскладки. ESC закрывает уже открытую консоль.
pub fn toggle_console(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut console: ResMut<DebugConsole>,
) {
    if keys.just_pressed(KeyCode::Backquote) {
        console.open = !console.open;
        console.input.clear();
        if console.open {
            console.log("КОНСОЛЬ ОТЛАДКИ");
            console.log("day 1..3 | stop | play | band | unband");
        }
        keys.clear();
        return;
    }
    if console.open && keys.just_pressed(KeyCode::Escape) {
        console.open = false;
        console.input.clear();
        keys.clear();
    }
}

/// Набор текста и Enter. Работает только при открытой консоли.
pub fn input_console(
    mut console: ResMut<DebugConsole>,
    keys: Res<ButtonInput<KeyCode>>,
    mut events: MessageReader<KeyboardInput>,
) {
    let open = console.open;
    for ev in events.read() {
        if !open || ev.state != ButtonState::Pressed {
            continue;
        }
        if ev.key_code == KeyCode::Backquote
            || ev.key_code == KeyCode::Enter
            || ev.key_code == KeyCode::NumpadEnter
            || ev.key_code == KeyCode::Backspace
        {
            continue;
        }
        if let Some(text) = ev.text.as_ref() {
            for ch in text.chars() {
                if ch.is_control() || console.input.chars().count() >= 48 {
                    continue;
                }
                console.input.push(ch);
            }
        }
    }
    if !open {
        return;
    }
    if keys.just_pressed(KeyCode::Backspace) {
        console.input.pop();
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        let cmd = console.input.trim().to_string();
        console.input.clear();
        if cmd.is_empty() {
            console.log("ПУСТАЯ КОМАНДА");
        } else {
            console.pending = Some(cmd);
        }
    }
}

/// Выполнить команду из `pending`. Консоль остаётся открытой, чтобы можно
/// было сразу ввести следующую (закрытие — ` / ESC).
pub fn execute_console(
    mut console: ResMut<DebugConsole>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    mut dirty: ResMut<Dirty>,
    mut cam: ResMut<GameCam>,
    mut modals: ResMut<Modals>,
    mut overbrew: ResMut<FirstOverbrew>,
    mut brew_qte: ResMut<BrewQte>,
    mut cool: ResMut<CoolQte>,
    mut fill: ResMut<FillState>,
    mut ambush: ResMut<BanditAmbush>,
    mut control: ResMut<BanditControl>,
    customers: Query<(Entity, Option<&crate::ambush::AmbushBandit>), With<Customer>>,
    mut commands: Commands,
) {
    let Some(cmd) = console.pending.take() else {
        return;
    };
    let lower = cmd.to_lowercase();
    let mut parts = lower.split_whitespace();
    let head = parts.next().unwrap_or("");
    if control.day != state.day {
        control.reset_day(state.day);
    }
    match head {
        "day" => {
            let Some(day) = parts.next().and_then(|s| s.parse::<u32>().ok()) else {
                console.log("ФОРМАТ: day 1 | day 2 | day 3");
                return;
            };
            if !(1..=MAX_DAY).contains(&day) {
                console.log(format!("ДОСТУПНЫ ДНИ 1..{MAX_DAY}"));
                return;
            }
            for (e, _) in customers.iter() {
                commands.entity(e).despawn();
            }
            ambush.reset();
            control.reset_day(day);
            control.force_next = false;
            give_start_package(&mut state, &mut inv);
            state.day = day;
            state.norm = norm_for(day);
            state.spawn_interval = (22.0 - day as f32 * 2.5).max(11.0);
            state.tut_done = true;
            state.tut_step = 0;
            state.tut_locked = false;
            state.tut_after_game = false;
            state.menu_view = crate::day_cycle::MenuView::Main;
            state.menu_sel = 0;
            state.pause_prev = PAUSE_FROM_PLAYING;
            state.pause_sel = 0;
            state.day_passed = false;
            state.msg_t = 0.0;
            state.alarm_t = 0.0;
            state.face_mood = 0;
            state.face_t = 0.0;
            state.wash_bar = 0.0;
            overbrew.reset();
            brew_qte.reset();
            cool.reset();
            *fill = FillState::default();
            *modals = Modals::default();
            start_day(&mut state, &mut barrels, &mut dirty, &mut cam);
            console.log(format!("ДЕНЬ {day} ЗАГРУЖЕН"));
        }
        "stop" => {
            if console.time_stopped {
                console.log("ВРЕМЯ УЖЕ ОСТАНОВЛЕНО");
            } else {
                console.time_stopped = true;
                console.log("ВРЕМЯ ОСТАНОВЛЕНО");
            }
        }
        "play" => {
            if console.time_stopped {
                console.time_stopped = false;
                console.log("ВРЕМЯ ПОШЛО");
            } else {
                console.log("ВРЕМЯ И ТАК ИДЁТ");
            }
        }
        "band" => {
            // Следующий переход склад → касса запустит сцену гарантированно,
            // даже если включён `unband` или исчерпан дневной лимит.
            control.force_next = true;
            if control.disabled {
                console.log("СЛЕДУЮЩИЙ ВЫХОД СО СКЛАДА: БАНДИТ ГАРАНТИРОВАН (UNBAND НЕ МЕШАЕТ)");
            } else {
                console.log("СЛЕДУЮЩИЙ ВЫХОД СО СКЛАДА: БАНДИТ ГАРАНТИРОВАН");
            }
        }
        "unband" => {
            if control.disabled {
                // Повторный unband — снова случайные появления из-за угла.
                control.disabled = false;
                control.force_next = false;
                console.log(format!(
                    "БАНДИТЫ ВКЛЮЧЕНЫ. ШАНС ОБЫЧНЫЙ, СЕГОДНЯ {}/{}",
                    control.spawned_today,
                    BanditControl::max_for_day(state.day)
                ));
            } else {
                // «Вовсе» — ни одного случайного бандита из-за угла,
                // но команда `band` продолжает работать.
                control.disabled = true;
                control.force_next = false;
                for (e, bandit) in customers.iter() {
                    if bandit.is_some() {
                        commands.entity(e).despawn();
                    }
                }
                ambush.reset();
                console.log("БАНДИТЫ ВЫКЛЮЧЕНЫ (КОМАНДА BAND ВСЁ ЕЩЁ РАБОТАЕТ)");
            }
        }
        "help" => {
            console.log("day 1..3 | stop | play | band | unband");
        }
        _ => {
            console.log(format!("НЕИЗВЕСТНАЯ КОМАНДА: {cmd}"));
        }
    }
}
