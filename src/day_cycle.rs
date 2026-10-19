//! Игровой день: фазы, норма денег, закупка сырья, спавн покупателей.

use bevy::prelude::*;
use rand::prelude::*;

use crate::customer::{spawn_customer, Customer, QUEUE_SPOTS};
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
    /// Игровой день.
    Playing,
    /// Итоги дня.
    DayEnd,
    /// Финальная заставка.
    Final,
    /// Пауза (ESC): продолжить / в меню / выход.
    Paused,
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
    /// Откуда открыта пауза (PAUSE_FROM_*).
    pub pause_prev: u8,
    /// Выбранный пункт меню паузы.
    pub pause_sel: u8,
    /// Лик героя (Doom-style): 0 спокойно, 1 рад (купил), 2 мимо/боль.
    pub face_mood: u8,
    /// Секунд, сколько держится текущее выражение лица.
    pub face_t: f32,
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
            spawn_t: 10.0,
            spawn_interval: 22.0,
            msg: String::new(),
            msg_t: 0.0,
            day_passed: false,
            tut_step: 0,
            tut_done: false,
            pause_prev: PAUSE_FROM_PLAYING,
            pause_sel: 0,
            face_mood: 0,
            face_t: 0.0,
        }
    }
}

impl GameState {
    pub fn notify(&mut self, text: String) {
        self.msg = text;
        self.msg_t = 3.0;
    }
    /// Выражение лица героя на `secs` секунд (Doom-style лик в HUD).
    pub fn face_set(&mut self, mood: u8, secs: f32) {
        self.face_mood = mood;
        self.face_t = secs;
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
    // ТЗ №8: стаканы многоразовые — стартовый пул 6 штук, не покупаются.
    inv.stock_glass = 6;
    inv.dirty_glasses = 0;
    inv.stock_jar = 2;
    inv.stock_bottle = 1;
    inv.hands = [None; 4];
}

/// Начало игрового дня (ТЗ №3: фазы закупки больше нет — день стартует
/// сразу, сырьё покупают в «окне снабжения» посреди смены).
pub fn start_day(
    state: &mut GameState,
    barrels: &mut crate::brewing::Barrels,
    dirty: &mut Dirty,
) {
    state.phase = Phase::Playing;
    state.time_left = day_len(state.day);
    state.spawn_t = 10.0;
    state.fine = 0;
    state.fine_reason.clear();
    // сброс мира: бочки и грязь (покупателей чистит вызывающий)
    for b in barrels.map.values_mut() {
        *b = crate::brewing::Barrel::new();
    }
    dirty.0 = 0;
    let d = state.day;
    let n = state.norm;
    state.notify(format!("ДЕНЬ {d} НАЧАЛСЯ! НОРМА {n} К. ОКНО СНАБЖЕНИЯ (E)"));
}

/// Дespawn всех покупателей (вызывается перед началом нового дня).
pub fn clear_customers(commands: &mut Commands, customers: &Query<Entity, With<Customer>>) {
    for e in customers.iter() {
        commands.entity(e).despawn();
    }
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

/// Откуда открыли паузу (для возврата).
pub const PAUSE_FROM_PLAYING: u8 = 0;
pub const PAUSE_FROM_TUTORIAL: u8 = 1;
/// Число пунктов в меню паузы.
pub const PAUSE_ITEMS: u8 = 3;
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
            let go = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space);
            if go {
                give_start_package(&mut state, &mut inv);
                if state.tut_done {
                    start_day(&mut state, &mut barrels, &mut dirty);
                } else {
                    state.phase = Phase::Tutorial;
                    state.tut_step = 0;
                    state.notify("ОБУЧЕНИЕ. W A S D - ХОД, МЫШЬ - ОБЗОР".into());
                }
            }
        }
        Phase::DayEnd => {
            let go = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space);
            if go {
                if state.day >= 4 {
                    state.phase = Phase::Final;
                    return;
                }
                // следующий день — сразу в игру (ТЗ №3: закупки нет)
                state.day += 1;
                state.norm = norm_for(state.day);
                state.day_earned = 0;
                // Поток покупателей щадящий: в день 1 один человек за раз,
                // дальше чуть плотнее, но без очереди в три ряда.
                state.spawn_interval = (22.0 - state.day as f32 * 2.5).max(11.0);
                clear_customers(&mut commands, &customers);
                start_day(&mut state, &mut barrels, &mut dirty);
            }
        }
        Phase::Final => {
            if keys.just_pressed(KeyCode::Escape) {
                state.phase = Phase::Menu;
            }
        }
        Phase::Playing => {
            if keys.just_pressed(KeyCode::Escape) {
                state.pause_prev = PAUSE_FROM_PLAYING;
                state.pause_sel = 0;
                state.phase = Phase::Paused;
            }
        }
        Phase::Tutorial => {
            if keys.just_pressed(KeyCode::Escape) {
                state.pause_prev = PAUSE_FROM_TUTORIAL;
                state.pause_sel = 0;
                state.phase = Phase::Paused;
            }
        }
        Phase::Paused => {
            let n = PAUSE_ITEMS.max(1);
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                state.pause_sel = (state.pause_sel + n - 1) % n;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                state.pause_sel = (state.pause_sel + 1) % n;
            }
            let ok = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE);
            if keys.just_pressed(KeyCode::Escape) {
                // ESC = продолжить
                ok_pause_resume(&mut state);
            } else if ok {
                match state.pause_sel.min(n - 1) {
                    0 => ok_pause_resume(&mut state),
                    1 => {
                        // в главное меню: сброс мира
                        for b in barrels.map.values_mut() {
                            *b = crate::brewing::Barrel::new();
                        }
                        for e in customers.iter() {
                            commands.entity(e).despawn();
                        }
                        dirty.0 = 0;
                        state.msg_t = 0.0;
                        state.phase = Phase::Menu;
                    }
                    _ => {
                        // выход из игры
                        std::process::exit(0);
                    }
                }
            }
        }
    }
}

/// Возврат из паузы к игре/обучению.
fn ok_pause_resume(state: &mut GameState) {
    state.phase = if state.pause_prev == PAUSE_FROM_TUTORIAL {
        Phase::Tutorial
    } else {
        Phase::Playing
    };
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
    if state.face_t > 0.0 {
        state.face_t -= dt;
        if state.face_t <= 0.0 {
            state.face_mood = 0;
        }
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
        let mut taken = [false; QUEUE_SPOTS.len()];
        for c in customers.iter() {
            if c.spot < taken.len() {
                taken[c.spot] = true;
            }
        }
        let pool = customer_pool(state.day);
        let mut rng = &mut game_rng.0;
        let _ = spawn_customer(&mut commands, &pool, &taken, &mut rng);
        state.spawn_t = state.spawn_interval + rng.random_range(-3.0..3.0);
    }
}
