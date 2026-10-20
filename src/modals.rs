//! Модальные окна в стиле DOS-лавки (ТЗ №3, №4).
//!
//! Пока окно открыто, весь игровой мир «заморожен» (движение, брожение,
//! очередь): в main.rs игровой Update-чейн гейтится `run_if(modal_closed)`,
//! а эта система рисует/обрабатывает само окно и продолжает работать.
//!
//! - `Supply` — окно снабжения (тайл `Tile::Supply`): закупка
//!   сухарей/сахара. Банки и бутылки НЕ продаются (ТЗ №4): посуда
//!   многоразовая, её возвращает мойка. Товар приходит через 3–5 сек.
//! - `VesselPick` — выбор пустой посуды со стеллажа (клавиши 1/2/3).
//! - `Sponge` — мини-игра мойки (ТЗ №4): выбор грязной посуды (1/2/3)
//!   и протирание её губкой (WASD/стрелки) до 80% чистоты.

use bevy::prelude::*;
use rand::prelude::*;

use crate::day_cycle::{GameRng, GameState};
use crate::inventory::{HandItem, Inventory};
use crate::sprites::Vessel;

/// Открытое модальное окно (мир на паузе).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Modal {
    #[default]
    None,
    /// Окно снабжения.
    Supply,
    /// Выбор посуды на стеллаже.
    VesselPick,
    /// Мойка посуды (ТЗ №4).
    Sponge,
}

/// Сетка мойки: 8×8 клеток грязи поверх крупной иконки посуды.
pub const SPONGE_N: usize = 8;
/// Сколько клеток нужно вытереть (ТЗ №4: ≥80%).
pub const SPONGE_DONE: u8 = ((SPONGE_N * SPONGE_N) as f32 * 0.8) as u8 + 1;
/// Радиус губки в клетках.
const SPONGE_R: f32 = 1.8;
/// Скорость губки (WASD/стрелки), клеток в секунду.
const SPONGE_SPEED: f32 = 5.0;

/// Состояние мини-игры мойки: выбор посуды или протирание.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SpongeStage {
    /// Выбор грязной посуды (клавиши 1/2/3).
    Picker,
    /// Протирание губкой.
    Wipe,
}

#[derive(Clone, Debug)]
pub struct Sponge {
    pub stage: SpongeStage,
    /// Какой вид посуды моем сейчас.
    pub vessel: Vessel,
    /// `dirty[y*8+x]` — в клетке ещё грязь.
    pub dirty: [bool; SPONGE_N * SPONGE_N],
    /// Положение губки (центр клетки, 0..8).
    pub cx: f32,
    pub cy: f32,
    /// Сколько клеток уже вытерто.
    pub wiped: u8,
}

impl Default for Sponge {
    fn default() -> Self {
        Self {
            stage: SpongeStage::Picker,
            vessel: Vessel::Glass,
            dirty: [false; SPONGE_N * SPONGE_N],
            cx: (SPONGE_N as f32) / 2.0,
            cy: (SPONGE_N as f32) / 2.0,
            wiped: 0,
        }
    }
}

impl Sponge {
    /// Начать мыть конкретную посуду: сетка целиком в грязи.
    pub fn begin(vessel: Vessel) -> Self {
        Self {
            stage: SpongeStage::Wipe,
            vessel,
            dirty: [true; SPONGE_N * SPONGE_N],
            cx: (SPONGE_N as f32) / 2.0,
            cy: (SPONGE_N as f32) / 2.0,
            wiped: 0,
        }
    }

    /// Доля вытертой грязи, 0..1.
    pub fn ratio(&self) -> f32 {
        self.wiped as f32 / (SPONGE_N * SPONGE_N) as f32
    }

    /// Стереть всё в радиусе губки. true — посуда вымыта (≥80%).
    pub fn erase(&mut self) -> bool {
        for gy in 0..SPONGE_N {
            for gx in 0..SPONGE_N {
                let i = gy * SPONGE_N + gx;
                if !self.dirty[i] {
                    continue;
                }
                let dx = gx as f32 + 0.5 - self.cx;
                let dy = gy as f32 + 0.5 - self.cy;
                if dx * dx + dy * dy <= SPONGE_R * SPONGE_R {
                    self.dirty[i] = false;
                    self.wiped += 1;
                }
            }
        }
        self.wiped >= SPONGE_DONE
    }

    /// Двигать губку (WASD/стрелки) и стереть на новом месте.
    pub fn shift(&mut self, dx: f32, dy: f32, dt: f32) {
        let step = SPONGE_SPEED * dt;
        self.cx = (self.cx + dx * step).clamp(0.4, SPONGE_N as f32 - 0.4);
        self.cy = (self.cy + dy * step).clamp(0.4, SPONGE_N as f32 - 0.4);
    }

    /// Есть ли грязная посуда какого-то вида (для закрытия мойки).
    pub fn any_dirty(inv: &Inventory) -> bool {
        inv.total_dirty() > 0
    }
}

/// Оплаченный заказ «в пути»: придёт через `t` секунд.
pub struct Delivery {
    pub kind: SupplyKind,
    pub t: f32,
}

/// Состояние модалок: что открыто + доставки из окна снабжения.
/// Один ресурс — чтобы render_frame укладывался в лимит параметров Bevy.
#[derive(Resource, Default)]
pub struct Modals {
    pub modal: Modal,
    /// Оплаченные заказы «в пути» (таймер тикает даже при открытой модалке).
    pub pending: Vec<Delivery>,
    /// Мини-игра мойки (живёт здесь же — снаружи модалка закрыта).
    pub sponge: Sponge,
}

impl Modals {
    #[inline]
    pub fn is_open(&self) -> bool {
        self.modal != Modal::None
    }
}

/// Что можно купить в окне снабжения (ТЗ №3/№4: только сырьё —
/// посуда многоразовая и продаётся в мойке, не здесь).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SupplyKind {
    Croutons,
    Sugar,
}

impl SupplyKind {
    pub fn price(self) -> i32 {
        match self {
            SupplyKind::Croutons => 20,
            SupplyKind::Sugar => 10,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SupplyKind::Croutons => "СУХАРИ",
            SupplyKind::Sugar => "САХАР С ИЗЮМОМ",
        }
    }

    /// Сколько штук приходит за одну покупку.
    pub fn qty(self) -> u8 {
        match self {
            SupplyKind::Croutons => 10,
            SupplyKind::Sugar => 10,
        }
    }

    /// Клавиша в модалке (1..2).
    pub fn key(self) -> u8 {
        match self {
            SupplyKind::Croutons => 1,
            SupplyKind::Sugar => 2,
        }
    }
}

const SUPPLY_KINDS: [SupplyKind; 2] = [SupplyKind::Croutons, SupplyKind::Sugar];

/// Ввод в открытом модалке: покупка (Supply), выбор посуды (VesselPick)
/// или мини-игра мойки (Sponge).
pub fn modal_input(
    time: Res<Time>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut modals: ResMut<Modals>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    if !modals.is_open() {
        return;
    }
    let digit = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .iter()
    .position(|k| keys.just_pressed(*k));

    match modals.modal {
        Modal::Supply => {
            if let Some(i) = digit {
                // клавиши сверх списка (3/4 больше не продаются — ТЗ №4)
                let Some(kind) = SUPPLY_KINDS.into_iter().nth(i) else {
                    return;
                };
                if state.money < kind.price() {
                    state.notify(format!(
                        "НЕ ХВАТАЕТ ДЕНЕГ: {} К. НУЖНО {}",
                        kind.name(),
                        kind.price()
                    ));
                    return;
                }
                state.money -= kind.price();
                modals.pending.push(Delivery {
                    kind,
                    t: game_rng.0.random_range(3.0..5.0),
                });
                state.notify(format!(
                    "ЗАКАЗ ПРИНЯТ: {} — ПРИВЕЗУТ ЧЕРЕЗ 3-5 СЕК",
                    kind.name()
                ));
                crate::audio::play_sfx(&mut commands, &bank, "coin", 0.5);
            }
            if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyE) {
                state.notify("ОКНО СНАБЖЕНИЯ ЗАКРЫТО".into());
                modals.modal = Modal::None;
                keys.clear(); // ТЗ №1: закрытие не открывает паузу в этот же кадр
            }
        }
        Modal::VesselPick => {
            let vessel = match digit {
                Some(0) => Some(Vessel::Glass),
                Some(1) => Some(Vessel::Jar),
                Some(2) => Some(Vessel::Bottle),
                _ => None,
            };
            if let Some(v) = vessel {
                if inv.put(HandItem::Empty(v, 1)) {
                    crate::audio::play_sfx(&mut commands, &bank, "pop", 0.5);
                    state.notify(format!("ВЗЯТ ПУСТЫЙ {}", v.name()));
                } else {
                    state.notify("РУКИ ЗАНЯТЫ — ПОЛОЖИТЕ ЧТО-НИБУДЬ".into());
                }
                modals.modal = Modal::None;
                keys.clear(); // ТЗ №1
            } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyE) {
                modals.modal = Modal::None;
                keys.clear(); // ТЗ №1
            }
        }
        Modal::Sponge => {
            let dt = time.delta_secs();
            if modals.sponge.stage == SpongeStage::Picker {
                // выбор грязной посуды: 1 стакан, 2 банка, 3 бутылка
                let vessel = match digit {
                    Some(0) => Some(Vessel::Glass),
                    Some(1) => Some(Vessel::Jar),
                    Some(2) => Some(Vessel::Bottle),
                    _ => None,
                };
                if let Some(v) = vessel {
                    if inv.dirty_of(v) == 0 {
                        state.notify(format!("ГРЯЗНЫХ {} НЕТ", v.name()));
                    } else {
                        modals.sponge = Sponge::begin(v);
                        crate::audio::play_sfx(&mut commands, &bank, "water", 0.6);
                        state.notify(format!(
                            "МОЕМ {}: WASD/СТРЕЛКИ ВЕДУТ ГУБКУ ({} ШТ)",
                            v.name(),
                            inv.dirty_of(v)
                        ));
                    }
                    keys.clear(); // ТЗ №1
                } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyE) {
                    modals.modal = Modal::None;
                    keys.clear(); // ТЗ №1
                }
            } else {
                // протирание: губка ходит по сетке и стирает грязь
                let (dx, dy) = (
                    (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i32
                        - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i32,
                    (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) as i32
                        - (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) as i32,
                );
                if dx != 0 || dy != 0 {
                    modals.sponge.shift(dx as f32, dy as f32, dt);
                }
                let vessel = modals.sponge.vessel;
                if modals.sponge.erase() {
                    // «дзынь» — посуда чистая, уходит на склад
                    inv.clean_one(vessel);
                    crate::audio::play_sfx(&mut commands, &bank, "ding", 0.7);
                    state.face_set(1, 1.5);
                    if Sponge::any_dirty(&inv) {
                        modals.sponge = Sponge::default(); // назад к выбору
                        state.notify(format!(
                            "ДЗЫНЬ! {} ЧИСТЫЙ НА СКЛАДЕ. ЧЬЮ ЕЩЁ МОЕМ? (1/2/3)",
                            vessel.name()
                        ));
                    } else {
                        modals.modal = Modal::None;
                        modals.sponge = Sponge::default();
                        state.notify(format!(
                            "ДЗЫНЬ! ВСЯ ПОСУДА ЧИСТАЯ НА СКЛАДЕ ({})",
                            vessel.name()
                        ));
                        keys.clear(); // ТЗ №1
                    }
                } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyE) {
                    modals.sponge = Sponge::default(); // назад к выбору посуды
                    keys.clear(); // ТЗ №1
                }
            }
        }
        Modal::None => {}
    }
}

/// Тик доставок: товар доезжает через 3–5 сек и ложится на склад.
pub fn supply_tick(
    time: Res<Time>,
    mut modals: ResMut<Modals>,
    mut inv: ResMut<Inventory>,
    mut state: ResMut<GameState>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    if modals.pending.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let mut arrived: Vec<SupplyKind> = Vec::new();
    for d in modals.pending.iter_mut() {
        d.t -= dt;
        if d.t <= 0.0 {
            arrived.push(d.kind);
        }
    }
    modals.pending.retain(|d| d.t > 0.0);
    for kind in arrived {
        match kind {
            SupplyKind::Croutons => inv.stock_croutons += kind.qty(),
            SupplyKind::Sugar => inv.stock_sugar += kind.qty(),
        }
        state.notify(format!(
            "ДОСТАВКА: {} (+{}) НА СКЛАД",
            kind.name(),
            kind.qty()
        ));
        crate::audio::play_sfx(&mut commands, &bank, "coin", 0.6);
    }
}
