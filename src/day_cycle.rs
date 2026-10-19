//! Игровой день: фазы, норма денег, закупка сырья, спавн покупателей.

use bevy::prelude::*;
use rand::prelude::*;

use crate::customer::{spawn_customer, spot_taken, Customer};
use crate::sprites::NpcKind;

#[derive(Resource)]
pub struct GameRng(pub StdRng);

/// Грязь: непромытые бочки/переброды — повод для штрафа.
#[derive(Resource, Default)]
pub struct Dirty(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// Главное меню.
    Menu,
    /// Обучение: без времени, один гость, подсказки с маркером.
    Tutorial,
    /// Закупка сырья (клавиши 1-3, Enter — начать день).
    Shopping,
    /// Игровой день.
    Playing,
    /// Итоги дня.
    DayEnd,
    /// Финальная заставка.
    Final,
}

#[derive(Resource)]
pub struct GameState {
    pub day: u32,
    pub money: i32,
    pub day_earned: i32,
    pub norm: i32,
    pub time_left: f32,
    pub phase: Phase,
    pub fine: i32,
    pub fine_reason: String,
    pub spawn_t: f32,
    pub spawn_interval: f32,
    pub msg: String,
    pub msg_t: f32,
    pub day_passed: bool,
    /// Шаг обучения (0..=6), 7 — конец.
    pub tut_step: u8,
    /// Обучение пройдено/пропущено.
    pub tut_done: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            day: 1,
            money: 30,
            day_earned: 0,
            norm: 100,
            time_left: 240.0,
            phase: Phase::Menu,
            fine: 0,
            fine_reason: String::new(),
            spawn_t: 8.0,
            spawn_interval: 14.0,
            msg: String::new(),
            msg_t: 0.0,
            day_passed: false,
            tut_step: 0,
            tut_done: false,
        }
    }
}

impl GameState {
    pub fn notify(&mut self, text: String) {
        self.msg = text;
        self.msg_t = 3.0;
    }
}

/// Стартовый пакет дня 1 (он же выдаётся перед обучением и после него).
pub fn give_start_package(state: &mut GameState, inv: &mut crate::inventory::Inventory) {
    state.day = 1;
    state.money = 30;
    state.norm = norm_for(1);
    state.day_earned = 0;
    inv.stock_croutons = 2;
    inv.stock_sugar = 2;
    inv.stock_glass = 4;
    inv.stock_jar = 2;
    inv.stock_bottle = 1;
    inv.hands = [None; 4];
}

pub fn day_len(day: u32) -> f32 {
    210.0 + 30.0 * day as f32
}

pub fn norm_for(day: u32) -> i32 {
    let mut n = 100.0f32;
    for _ in 1..day {
        n *= 1.4;
    }
    n as i32
}

/// Кто может зайти в этот день.
pub fn customer_pool(day: u32) -> Vec<NpcKind> {
    use NpcKind::*;
    let mut v = vec![Grandma, Worker, Mama, Student];
    if day >= 2 {
        v.push(Worker);
        v.push(Mama);
        v.push(Student);
    }
    v
}

/// Фазы: меню, закупка, конец дня.
pub fn phase_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<crate::inventory::Inventory>,
    mut commands: Commands,
    mut barrels: ResMut<crate::brewing::Barrels>,
    customers: Query<Entity, With<Customer>>,
    mut dirty: ResMut<Dirty>,
) {
    match state.phase {
        Phase::Menu => {
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                give_start_package(&mut state, &mut inv);
                if state.tut_done {
                    state.phase = Phase::Shopping;
                    state.notify("НАЧАЛО ДНЯ 1. СЫРЬЁ: 1-3, ЗАТЕМ ENTER".into());
                } else {
                    state.phase = Phase::Tutorial;
                    state.tut_step = 0;
                    state.notify("ОБУЧЕНИЕ. W A S D - ХОД, МЫШЬ - ОБЗОР".into());
                }
            }
        }
        Phase::Shopping => {
            if keys.just_pressed(KeyCode::Digit1) && state.money >= 20 {
                state.money -= 20;
                inv.stock_croutons += 10;
                state.notify("КУПЛЕНО: СУХАРИ (10 ГОРСТЕЙ) -20 К.".into());
            }
            if keys.just_pressed(KeyCode::Digit2) && state.money >= 10 {
                state.money -= 10;
                inv.stock_sugar += 10;
                state.notify("КУПЛЕНО: САХАР+ИЗЮМ (10 ПОРЦИЙ) -10 К.".into());
            }
            if keys.just_pressed(KeyCode::Digit3) && state.money >= 15 {
                state.money -= 15;
                inv.stock_glass += 4;
                inv.stock_jar += 4;
                inv.stock_bottle += 4;
                state.notify("КУПЛЕНО: ПОСУДА (4+4+4) -15 К.".into());
            }
            if keys.just_pressed(KeyCode::Enter) {
                state.phase = Phase::Playing;
                state.time_left = day_len(state.day);
                state.spawn_t = 6.0;
                state.fine = 0;
                state.fine_reason.clear();
                // сброс мира: бочки, покупатели, грязь
                for b in barrels.map.values_mut() {
                    *b = crate::brewing::Barrel::new();
                }
                for e in customers.iter() {
                    commands.entity(e).despawn();
                }
                dirty.0 = 0;
                let d = state.day;
                state.notify(format!("ДЕНЬ {} НАЧАЛСЯ!", d));
            }
        }
        Phase::DayEnd => {
            if keys.just_pressed(KeyCode::Enter) {
                if state.day >= 4 {
                    state.phase = Phase::Final;
                    return;
                }
                // следующий день
                let _ = &mut commands;
                state.day += 1;
                state.norm = norm_for(state.day);
                state.day_earned = 0;
                state.phase = Phase::Shopping;
                state.spawn_interval = (14.0 - state.day as f32 * 2.0).max(6.0);
                let d = state.day;
                let n = state.norm;
                state.notify(format!("ДЕНЬ {}. НОРМА {} К. ЗАКУПИТЕ СЫРЬЁ.", d, n));
            }
        }
        Phase::Final => {
            if keys.just_pressed(KeyCode::Escape) {
                state.phase = Phase::Menu;
            }
        }
        Phase::Tutorial | Phase::Playing => {}
    }
}

/// Идёт игровой день: таймер, спавн, конец смены.
pub fn day_system(
    time: Res<Time>,
    mut state: ResMut<GameState>,
    customers: Query<&Customer>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let dt = time.delta_secs();
    if state.msg_t > 0.0 {
        state.msg_t -= dt;
    }
    state.time_left -= dt;
    if state.time_left <= 0.0 {
        state.time_left = 0.0;
        state.day_passed = state.money >= state.norm;
        state.phase = Phase::DayEnd;
        return;
    }
    // спавн покупателей
    state.spawn_t -= dt;
    if state.spawn_t <= 0.0 {
        let mut taken = [false; 4];
        for c in customers.iter() {
            taken[c.spot] = true;
        }
        let pool = customer_pool(state.day);
        let mut rng = &mut game_rng.0;
        let _ = spawn_customer(&mut commands, &pool, &taken, &mut rng);
        state.spawn_t = state.spawn_interval + rng.random_range(-3.0..3.0);
    }
}
