//! Взаимодействие (E / ЛКМ): станции, бочка, розлив, выдача покупателям.

use bevy::prelude::*;
use rand::prelude::*;

use crate::brewing::{BarrelPhase, Barrels};
use crate::customer::{CustState, Customer};
use crate::day_cycle::{GameState, GameRng, Phase};
use crate::inventory::{HandItem, Inventory};
use crate::map::Tile;
use crate::raycaster::GameCam;
use crate::sprites::Vessel;

/// Прогресс розлива/промывки бочки.
#[derive(Resource, Default)]
pub struct FillState {
    pub progress: f32,
    pub target: (i32, i32),
}

fn front_tile(cam: &GameCam) -> (i32, i32) {
    let fx = cam.x + cam.angle.cos() * 0.9;
    let fy = cam.y + cam.angle.sin() * 0.9;
    (fx.floor() as i32, fy.floor() as i32)
}

pub fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cam: Res<GameCam>,
    map: Res<crate::map::Map>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut customers: Query<(Entity, &mut Customer)>,
    mut shelf_cycle: Local<usize>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    let pressed = keys.just_pressed(KeyCode::KeyE) || mouse.just_pressed(MouseButton::Left);
    if !pressed || !matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        return;
    }
    let (tx, ty) = front_tile(&cam);
    let tile = map.get(tx, ty);
    let rng = &mut game_rng.0;

    match tile {
        Tile::Croutons => {
            if inv.take_croutons_from_stock() {
                crate::audio::play_sfx(&mut commands, &bank, "croutons", 0.6);
                state.notify("ГОРСТЬ РЖАНЫХ СУХАРЕЙ ВЗЯТА".into());
            } else {
                state.notify("СУХАРИ КОНЧИЛИСЬ — КУПИТЕ (1) НА СКЛАДЕ".into());
            }
        }
        Tile::Sugar => {
            if inv.take_sugar_from_stock() {
                crate::audio::play_sfx(&mut commands, &bank, "croutons", 0.5);
                state.notify("САХАР С ИЗЮМОМ ВЗЯТ".into());
            } else {
                state.notify("САХАР КОНЧИЛСЯ — КУПИТЕ (2)".into());
            }
        }
        Tile::Water => {
            if inv.has_bucket() {
                state.notify("ВЕДРО УЖЕ ПОЛНОЕ".into());
            } else if inv.put(HandItem::Bucket) {
                crate::audio::play_sfx(&mut commands, &bank, "water", 0.6);
                state.notify("ВЕДРО ВОДЫ НАБРАНО".into());
            } else {
                state.notify("РУКИ ЗАНЯТЫ".into());
            }
        }
        Tile::Shelf => {
            let Some(v) = inv.take_empty_from_stock(&mut shelf_cycle) else {
                state.notify("ПОСУДА КОНЧИЛАСЬ — КУПИТЕ (3)".into());
                return;
            };
            if inv.put(HandItem::Empty(v, 1)) {
                state.notify(format!("ВЗЯТ ПУСТЫЙ {}", v.name()));
            } else {
                match v {
                    Vessel::Glass => inv.stock_glass += 1,
                    Vessel::Jar => inv.stock_jar += 1,
                    Vessel::Bottle => inv.stock_bottle += 1,
                }
                state.notify("РУКИ ЗАНЯТЫ".into());
            }
        }
        Tile::Barrel => {
            let Some(b) = barrels.mutable_at(tx, ty) else { return };
            match b.phase {
                BarrelPhase::Empty => match b.step {
                    0 => {
                        if inv.drain_croutons() {
                            b.step = 1;
                            crate::audio::play_sfx(&mut commands, &bank, "croutons", 0.7);
                            state.notify("СУХАРИ ВЫСЫПАНЫ В БОЧКУ".into());
                        } else {
                            state.notify("НУЖНЫ СУХАРИ — ПОЛКА СНИЗУ (E)".into());
                        }
                    }
                    1 => {
                        if inv.drain_bucket() {
                            b.step = 2;
                            crate::audio::play_sfx(&mut commands, &bank, "water", 0.7);
                            state.notify("ВОДА ЗАЛИТА".into());
                        } else {
                            state.notify("НУЖНО ВЕДРО ВОДЫ — КОЛОНКА (E)".into());
                        }
                    }
                    _ => {
                        if inv.drain_sugar() {
                            let t = crate::brewing::random_brew_time(rng);
                            b.phase = BarrelPhase::Ferment(t);
                            b.step = 0;
                            crate::audio::play_sfx(&mut commands, &bank, "pop", 0.7);
                            state.notify(format!("САХАР ДОБАВЛЕН. БРОДИТ ~{:.0} СЕК", t));
                        } else {
                            state.notify("НУЖЕН САХАР С ИЗЮМОМ — ПОЛКА (E)".into());
                        }
                    }
                },
                BarrelPhase::Ferment(_) => {
                    state.notify("БОЧКА БРОДИТ — ПОДОЖДИТЕ".into());
                }
                BarrelPhase::Ready(t) => {
                    if b.portions == 0 {
                        state.notify(format!("ПУСТО! СВАРИТЕ НОВЫЙ. {t:.0} СЕК ДО ПЕРЕБРОДА"));
                    } else {
                        state.notify(format!("ГОТОВА! ДЕРЖИТЕ E ({} ПОРЦИЙ)", b.portions));
                    }
                }
                BarrelPhase::Overripe(t) => {
                    state.notify(format!("ПЕРЕБРОД! РАЗЛИВАЙТЕ, ЧТО ЕСТЬ ({t:.0} СЕК)"));
                }
                BarrelPhase::Ruined => {
                    state.notify("ИСПОРЧЕНА. ДЕРЖИТЕ E — ПОМЫТЬ".into());
                }
            }
        }
        Tile::Counter => {
            // покупатель за прилавком?
            let fx = cam.x + cam.angle.cos() * 1.6;
            let fy = cam.y + cam.angle.sin() * 1.6;
            let target = Vec2::new(fx, fy);
            let mut best: Option<(Entity, f32)> = None;
            for (e, c) in customers.iter() {
                if c.state != CustState::Queued {
                    continue;
                }
                let d = c.pos.distance(target);
                if d < 1.7 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((e, d));
                }
            }
            let Some((entity, _)) = best else {
                state.notify("ЗДЕСЬ НИКОГО НЕТ".into());
                return;
            };
            let Ok((_, mut c)) = customers.get_mut(entity) else { return };
            let order = c.order;
            let kind = c.kind;
            match inv.take_full(order) {
                Some((v, overripe)) => {
                    let mut pay = v.price();
                    if overripe {
                        pay /= 2;
                    }
                    // чаевые: если ждал недолго
                    if c.patience > c.max_patience * 0.5 && !overripe {
                        pay += 1;
                    }
                    state.money += pay;
                    state.day_earned += pay;
                    crate::audio::play_sfx(&mut commands, &bank, "coin", 0.7);
                    let line = crate::dialogues::thanks_line(kind, overripe, rng);
                    c.say(line);
                    c.state = CustState::Leaving;
                    c.served = true;
                    c.path = std::collections::VecDeque::from([crate::customer::DOOR]);
                    state.notify(format!("{}: +{} К.", kind.name(), pay));
                }
                None => {
                    state.notify(format!("НЕТ ПОЛНОГО {} В РУКАХ", order.name()));
                }
            }
        }
        _ => {}
    }
}

/// Удержание E у бочки: розлив из крана и промывка испорченной.
pub fn fill_hold(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cam: Res<GameCam>,
    map: Res<crate::map::Map>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut fill: ResMut<FillState>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    let holding = keys.pressed(KeyCode::KeyE) || mouse.pressed(MouseButton::Left);
    if !matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        return;
    }
    let (tx, ty) = front_tile(&cam);
    if map.get(tx, ty) != Tile::Barrel {
        fill.progress = 0.0;
        return;
    }
    let Some(b) = barrels.mutable_at(tx, ty) else { return };

    // Промывка испорченной.
    if b.phase == BarrelPhase::Ruined {
        if holding {
            if fill.target != (tx, ty) {
                fill.target = (tx, ty);
                fill.progress = 0.0;
                crate::audio::play_sfx(&mut commands, &bank, "water", 0.6);
            }
            fill.progress += time.delta_secs() / 1.5;
            if fill.progress >= 1.0 {
                b.phase = BarrelPhase::Empty;
                b.step = 0;
                b.portions = 0;
                fill.progress = 0.0;
                state.notify("БОЧКА ВЫМЫТА. СВАРИТЕ НОВЫЙ КВАС".into());
            }
        } else {
            fill.progress = 0.0;
        }
        return;
    }

    // Розлив.
    if !b.can_fill() {
        fill.progress = 0.0;
        if state.phase == Phase::Tutorial && holding {
            if let BarrelPhase::Ferment(t) = b.phase {
                state.notify(format!("КВАС ЕЩЁ БРОДИТ, ~{:.0} СЕК", t));
            }
        }
        return;
    }
    // найти первую пустую посуду в руках
    let Some((slot_idx, vessel)) = inv.hands.iter().enumerate().find_map(|(i, h)| {
        if let Some(HandItem::Empty(v, n)) = h {
            Some((i, (*v, *n)))
        } else {
            None
        }
    }) else {
        fill.progress = 0.0;
        return;
    };
    if !holding {
        return; // прогресс держим
    }
    if fill.target != (tx, ty) {
        fill.target = (tx, ty);
        fill.progress = 0.0;
        crate::audio::play_sfx(&mut commands, &bank, "pour", 0.7);
    }
    let need = vessel.0.portions() as f32;
    fill.progress += time.delta_secs() * 2.0; // 2 порции/сек
    if fill.progress >= need {
        // залить
        let overripe = b.overripe_fill();
        b.portions = b.portions.saturating_sub(vessel.0.portions());
        // уменьшить пустую, добавить полную
        if let Some(HandItem::Empty(_, n)) = &mut inv.hands[slot_idx] {
            *n -= 1;
            if *n == 0 {
                inv.hands[slot_idx] = None;
            }
        }
        let _ = inv.put(HandItem::Full(vessel.0, 1, overripe));
        fill.progress = 0.0;
        if b.portions == 0 {
            state.notify("БОЧКА ПУСТА! СВАРИТЕ НОВУЮ".into());
        }
    }
}
