//! Враги/помехи: хулиганы (ломают бочку), милиция и санэпид (штраф, взятка банкой).

use std::collections::VecDeque;

use bevy::prelude::*;
use rand::prelude::*;

use crate::ambush::BanditControl;
use crate::brewing::{BarrelPhase, Barrels};
use crate::customer::{CustState, Customer, QUEUE_SPOTS};
use crate::day_cycle::{GameState, GameRng, Phase};
use crate::dialogues;
use crate::sprites::NpcKind;

/// Милиционер или санэпид: взятка банкой снимает штраф.
#[derive(Component)]
pub struct Official;

/// Хулиган: если не угостить — ломает бочку.
#[derive(Component)]
pub struct Hooligan {
    pub t: f32,
    pub going: bool,
    pub smashed: bool,
}

/// Спавн врагов по дню.
pub fn enemies_spawn(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    customers: Query<&Customer>,
    mut game_rng: ResMut<GameRng>,
    bank: Res<crate::audio::SoundBank>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let rng = &mut game_rng.0;
    let count = customers.iter().count();
    // милиционер раз в день со 2-го дня
    if state.day >= 2 && (state.time_left - 150.0).abs() < 0.6 && count < 4 {
        let _ = spawn_official(&mut commands, NpcKind::Militsioner, rng);
        crate::audio::play_sfx(&mut commands, &bank, "whistle", 0.6);
        state.notify("МИЛИЦИЯ ИДЁТ ПРОВЕРЯТЬ...".into());
    }
    // санэпидемнадзор с 3-го дня
    if state.day >= 3 && (state.time_left - 90.0).abs() < 0.6 && count < 4 {
        let _ = spawn_official(&mut commands, NpcKind::Sanepid, rng);
        state.notify("САНЭПИДЕМНАДЗОР ВНИМАЕТ...".into());
    }
}

fn spawn_official(commands: &mut Commands, kind: NpcKind, rng: &mut StdRng) -> Option<usize> {
    // последнее место очереди — ближе к двери
    let spot = QUEUE_SPOTS.len() - 1;
    let c = Customer {
        kind,
        order: crate::sprites::Vessel::Jar,
        state: CustState::Entering,
        pos: crate::customer::DOOR,
        path: VecDeque::from([Vec2::new(QUEUE_SPOTS[spot].0, QUEUE_SPOTS[spot].1)]),
        patience: 80.0,
        max_patience: 80.0,
        bubble: None,
        bubble_t: 0.0,
        spot,
        served: false,
        step_t: 0.0,
    };
    commands.spawn((c, Official));
    Some(spot)
}

/// Штраф, если проверяющий ушёл с пустыми руками.
pub fn officials_fine(
    mut q: Query<(&mut Customer, &Official)>,
    mut state: ResMut<GameState>,
    mut game_rng: ResMut<GameRng>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let rng = &mut game_rng.0;
    for (mut c, _) in &mut q {
        if c.state == CustState::Leaving && !c.served {
            let reason = if c.kind == NpcKind::Sanepid {
                dialogues::sanepid_line(rng)
            } else {
                dialogues::militsioner_line(rng)
            };
            state.money -= 30;
            state.fine += 30;
            state.fine_reason = reason.to_string();
            state.notify(format!("ШТРАФ 30 К. {}", reason));
            c.served = true; // чтобы не сработало дважды
        }
    }
}

/// Хулиганы: спавн со 2-го дня, угрожают, ломают бочку.
pub fn hooligan_spawn(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    customers: Query<&Customer>,
    mut game_rng: ResMut<GameRng>,
    mut cooldown: Local<f32>,
    time: Res<Time>,
    bank: Res<crate::audio::SoundBank>,
    mut control: ResMut<BanditControl>,
) {
    if state.phase != Phase::Playing || state.day < 2 {
        return;
    }
    if control.day != state.day {
        control.reset_day(state.day);
        *cooldown = 0.0;
    }
    // `unband` выключает поток хулиганов; лимит дня — 3 на второй день
    // и 4 на третий (общий счётчик с коридорной сценой).
    if !control.can_spawn(state.day) {
        return;
    }
    *cooldown -= time.delta_secs();
    if *cooldown > 0.0 {
        return;
    }
    let count = customers.iter().count();
    if count >= 4 {
        return;
    }
    let rng = &mut game_rng.0;
    if !rng.random_bool(0.02) {
        return;
    }
    // ТЗ №5: бандиты приходят реже — интервал между визитами ~55 с.
    *cooldown = 55.0;
    // ТЗ №6: не встаём на уже занятое место в очереди (иначе два человека
    // визуально стоят в одной точке). Бандит встаёт в свободный хвост.
    let mut taken = [false; QUEUE_SPOTS.len()];
    for c in customers.iter() {
        if c.spot < taken.len() {
            taken[c.spot] = true;
        }
    }
    let spot = (0..QUEUE_SPOTS.len())
        .find(|&i| !taken[i])
        .unwrap_or(QUEUE_SPOTS.len() - 1);
    // ТЗ №5: атмосферная тревога — красная пульсация экрана + низкий гул.
    state.alarm_t = 3.0;
    state.notify("БАНДИТЫ НА ПОДХОДЕ! БЕГИТЕ К КАССЕ (E) - ЗАПЛАТИТЕ ДАНЬ".into());
    crate::audio::play_sfx(&mut commands, &bank, "alarm", 0.8);
    let c = Customer {
        kind: NpcKind::Hooligan,
        order: crate::sprites::Vessel::Glass,
        state: CustState::Entering,
        pos: crate::customer::DOOR,
        path: VecDeque::from([Vec2::new(QUEUE_SPOTS[spot].0, QUEUE_SPOTS[spot].1)]),
        patience: 45.0,
        max_patience: 45.0,
        bubble: None,
        bubble_t: 0.0,
        spot,
        served: false,
        step_t: 0.0,
    };
    commands.spawn((
        c,
        Hooligan {
            t: 0.0,
            going: false,
            smashed: false,
        },
    ));
    control.note_spawn();
}

/// Поведение бандитов (ТЗ №7): требование дани -> E у кассы платит 30 К ->
/// или через 15 сек портят бочку и уходят.
pub fn hooligan_behavior(
    time: Res<Time>,
    mut q: Query<(Entity, &mut Customer, &mut Hooligan)>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut dirty: ResMut<crate::day_cycle::Dirty>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let dt = time.delta_secs();
    let rng = &mut game_rng.0;
    let mut to_despawn: Vec<Entity> = Vec::new();
    for (e, mut c, mut h) in &mut q {
        if c.served {
            // угостили стаканом — доволен, уходит
            if c.state == CustState::Queued {
                c.state = CustState::Leaving;
                c.path = VecDeque::from([crate::customer::DOOR]);
                c.say("Ладно, считай. Только бочку не трогай.".into());
            }
            continue;
        }
        if c.state != CustState::Queued || c.spot != 0 || h.going {
            continue;
        }
        h.t += dt;
        // ТЗ №7: бандит требует дань; игрок платит E у кассы (30 К).
        if (h.t - 0.5).abs() < dt || (h.t - 5.0).abs() < dt {
            c.say("ПЛАТИ ДАНЬ 30 К. ИЛИ ИСПОРТИМ КВАС!".into());
        }
        if (h.t - 10.0).abs() < dt {
            c.say(dialogues::hooligan_threat(rng).to_string());
        }
        if h.t > 15.0 {
            // идёт за бочку: обход прилавка с запада, затем вдоль южной стены
            h.going = true;
            c.state = CustState::Custom;
            c.path = VecDeque::from([
                Vec2::new(3.5, 3.5),
                Vec2::new(3.5, 7.5),
                Vec2::new(6.5, 7.4),
            ]);
            c.say("Всё, барин, сам виноват!".into());
        }
        let _ = &mut commands;
    }
    // движение «за бочку» и слом
    for (_, mut c, mut h) in &mut q {
        if !h.going || h.smashed {
            continue;
        }
        if let Some(t) = c.path.front() {
            let d = *t - c.pos;
            let len = d.length();
            if len < 0.2 {
                c.path.pop_front();
            } else {
                c.pos += d / len * 2.2 * dt;
                c.step_t += dt;
            }
        } else {
            // сломать ближайшую бочку
            let key = barrels.map.keys().copied().min_by(|a, b| {
                let da = (a.0 as f32 + 0.5 - c.pos.x).powi(2) + (a.1 as f32 + 0.5 - c.pos.y).powi(2);
                let db = (b.0 as f32 + 0.5 - c.pos.x).powi(2) + (b.1 as f32 + 0.5 - c.pos.y).powi(2);
                da.partial_cmp(&db).unwrap()
            });
            if let Some(k) = key {
                if let Some(b) = barrels.mutable_at(k.0, k.1) {
                    if b.phase != BarrelPhase::Ruined {
                        b.phase = BarrelPhase::Ruined;
                        b.portions = 0;
                        b.ran_out = false; // сломан, а не «слили всё»
                        dirty.0 += 1;
                        state.notify("БОЧКА СЛОМАНА! ПОДОЖДИТЕ 3 СЕК И МОЙТЕ (E)".into());
                    }
                }
            }
            h.smashed = true;
            crate::audio::play_sfx(&mut commands, &bank, "fooff", 0.9);
            c.say("ВОТ ТЕБЕ И КВАС!".into());
            // ТЗ №3: после слома хулиган уходит по маршруту, а не замирает
            // навсегда (Custom-состояние не двигается сам по себе).
            c.state = CustState::Leaving;
            c.path = VecDeque::from([
                Vec2::new(3.5, 7.5),
                Vec2::new(3.5, 1.5),
                Vec2::new(5.5, 1.5),
                crate::customer::DOOR,
            ]);
        }
    }
    let _ = &mut to_despawn;
}
