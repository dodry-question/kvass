//! Модальные окна в стиле DOS-лавки (ТЗ №3 и №4).
//!
//! Пока окно открыто, весь игровой мир «заморожен» (движение, брожение,
//! очередь): в main.rs игровой Update-чейн гейтится `run_if(modal_closed)`,
//! а эта система рисует/обрабатывает само окно и продолжает работать.
//!
//! - `Supply` — окно снабжения (тайл `Tile::Supply`): закупка
//!   сухарей/сахара/банок/бутылок (стаканы не продаются — они многоразовые,
//!   ТЗ №8). Товар приходит через 3–5 сек после оплаты.
//! - `VesselPick` — выбор пустой посуды со стеллажа (клавиши 1/2/3).

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
}

impl Modals {
    #[inline]
    pub fn is_open(&self) -> bool {
        self.modal != Modal::None
    }
}

/// Что можно купить в окне снабжения (ТЗ №3: без стаканов).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SupplyKind {
    Croutons,
    Sugar,
    Jar,
    Bottle,
}

impl SupplyKind {
    pub fn price(self) -> i32 {
        match self {
            SupplyKind::Croutons => 20,
            SupplyKind::Sugar => 10,
            SupplyKind::Jar => 10,
            SupplyKind::Bottle => 10,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SupplyKind::Croutons => "СУХАРИ",
            SupplyKind::Sugar => "САХАР С ИЗЮМОМ",
            SupplyKind::Jar => "БАНКИ",
            SupplyKind::Bottle => "БУТЫЛКИ",
        }
    }

    /// Сколько штук приходит за одну покупку.
    pub fn qty(self) -> u8 {
        match self {
            SupplyKind::Croutons => 10,
            SupplyKind::Sugar => 10,
            SupplyKind::Jar => 4,
            SupplyKind::Bottle => 4,
        }
    }

    /// Клавиша в модалке (1..4).
    pub fn key(self) -> u8 {
        match self {
            SupplyKind::Croutons => 1,
            SupplyKind::Sugar => 2,
            SupplyKind::Jar => 3,
            SupplyKind::Bottle => 4,
        }
    }
}

const SUPPLY_KINDS: [SupplyKind; 4] = [
    SupplyKind::Croutons,
    SupplyKind::Sugar,
    SupplyKind::Jar,
    SupplyKind::Bottle,
];

/// Ввод в открытом модалке: покупка (Supply) или выбор посуды (VesselPick).
pub fn modal_input(
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
            SupplyKind::Jar => inv.stock_jar += kind.qty(),
            SupplyKind::Bottle => inv.stock_bottle += kind.qty(),
        }
        state.notify(format!(
            "ДОСТАВКА: {} (+{}) НА СКЛАД",
            kind.name(),
            kind.qty()
        ));
        crate::audio::play_sfx(&mut commands, &bank, "coin", 0.6);
    }
}
