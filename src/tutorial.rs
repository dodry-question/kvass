//! Обучение (пробный уровень): без времени и спешки, один гость,
//! пошаговые инструкции сверху и мигающий маркер-стрелка на цель.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::brewing::{BarrelPhase, Barrels, BrewQte};
use crate::customer::{CustState, Customer, QUEUE_SPOTS};
use crate::day_cycle::{give_start_package, start_day, GameState, Phase};
use crate::inventory::{HandItem, Inventory};
use crate::sprites::{NpcKind, Vessel};

/// Шаг обучения: текст инструкции и мировая точка цели (центр тайла).
pub struct TutStep {
    pub text: &'static str,
    pub target: (f32, f32),
}

pub const TUT_STEPS: [TutStep; 8] = [
    TutStep {
        text: "ШАГ 1/8. ВОЗЬМИТЕ СУХАРИ У ПОЛКИ (E)",
        target: (3.5, 6.5),
    },
    TutStep {
        text: "ШАГ 2/8. НАБЕРИТЕ ВОДУ У КОЛОНКИ (E)",
        target: (6.5, 6.5),
    },
    TutStep {
        text: "ШАГ 3/8. ВОЗЬМИТЕ САХАР С ИЗЮМОМ (E)",
        target: (5.5, 6.5),
    },
    TutStep {
        text: "ШАГ 4/8. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        target: (1.5, 4.5),
    },
    // ТЗ №2: отдельный шаг на мини-игру ускорения брожения.
    TutStep {
        text: "ШАГ 5/8. НАЖИМАЙТЕ ПРОБЕЛ В ЗОНУ БЕГУНКА",
        target: (1.5, 4.5),
    },
    TutStep {
        text: "ШАГ 6/8. ВОЗЬМИТЕ СТАКАН СО СТЕЛЛАЖА (E)",
        target: (8.5, 6.5),
    },
    TutStep {
        text: "ШАГ 7/8. КВАС ГОТОВ: ДЕРЖИТЕ E У БОЧКИ",
        target: (1.5, 4.5),
    },
    TutStep {
        text: "ШАГ 8/8. ОТДАЙТЕ КВАС БАБУШКЕ (E)",
        target: (5.5, 2.7), // на шаге 8 цель — сама бабушка у прилавка
    },
];

/// Логика обучения: шаги, спавн гостя, быстрое брожение, переход на склад.
pub fn tutorial_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    qte: Res<BrewQte>,
    customers: Query<(Entity, &Customer)>,
    mut commands: Commands,
    mut dirty: ResMut<crate::day_cycle::Dirty>,
) {
    if state.phase != Phase::Tutorial {
        return;
    }
    let dt = time.delta_secs();
    if state.msg_t > 0.0 {
        state.msg_t -= dt;
    }

    // T: на шаге мини-игры (5) пропускает только этот шаг,
    // на остальных — пропускает обучение целиком.
    if keys.just_pressed(KeyCode::KeyT) {
        if state.tut_step == 4 {
            state.tut_step = 5;
            state.notify("ТАК ДЕРЖАТЬ!".into());
        } else {
            finish(&mut state, &mut inv, &mut barrels, &customers, &mut commands, &mut dirty);
        }
        return;
    }

    // в обучении квас бродит быстро (осталось 8 сек), чтобы не ждать минуту
    for b in barrels.map.values_mut() {
        if let BarrelPhase::Ferment(p) = &mut b.phase {
            let fast = (1.0 - 8.0 / b.brew_time.max(1.0)) * 100.0;
            if *p < fast {
                *p = fast;
            }
        }
    }

    // финальный шаг: появляется бабушка, пока её нет
    if state.tut_step == 7 && customers.iter().count() == 0 {
        let c = Customer {
            kind: NpcKind::Grandma,
            order: Vessel::Glass,
            state: CustState::Entering,
            pos: crate::customer::DOOR,
            path: VecDeque::from([Vec2::new(QUEUE_SPOTS[0].0, QUEUE_SPOTS[0].1)]),
            patience: 9999.0,
            max_patience: 9999.0,
            bubble: None,
            bubble_t: 0.0,
            spot: 0,
            served: false,
            step_t: 0.0,
        };
        commands.spawn(c);
    }

    // выполнение текущего шага
    let done = match state.tut_step {
        0 => inv.has_croutons(),
        1 => inv.has_bucket(),
        2 => inv.has_sugar(),
        3 => barrels
            .map
            .values()
            .any(|b| matches!(b.phase, BarrelPhase::Ferment(_))),
        // ТЗ №2: шаг закрывается после одного попадания в зону бегунка.
        4 => qte.flash > 0.0,
        5 => inv
            .hands
            .iter()
            .any(|h| matches!(h, Some(HandItem::Empty(_, _)))),
        6 => inv
            .hands
            .iter()
            .any(|h| matches!(h, Some(HandItem::Full(_, _, _)))),
        7 => customers.iter().any(|(_, c)| c.served),
        _ => false,
    };
    if done {
        if state.tut_step >= 7 {
            finish(&mut state, &mut inv, &mut barrels, &customers, &mut commands, &mut dirty);
        } else {
            state.tut_step += 1;
            state.notify("ТАК ДЕРЖАТЬ!".into());
        }
    }
}

fn finish(
    state: &mut GameState,
    inv: &mut Inventory,
    barrels: &mut Barrels,
    customers: &Query<(Entity, &Customer)>,
    commands: &mut Commands,
    dirty: &mut crate::day_cycle::Dirty,
) {
    state.tut_done = true;
    state.tut_step = 0;
    for (e, _) in customers.iter() {
        commands.entity(e).despawn();
    }
    give_start_package(state, inv);
    // ТЗ №3: день начинается сразу, без экрана закупки.
    start_day(state, barrels, dirty);
}
