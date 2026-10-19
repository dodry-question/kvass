//! Обучение (пробный уровень): без времени и спешки, один гость,
//! пошаговые инструкции сверху и мигающий маркер-стрелка на цель.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::brewing::{BarrelPhase, Barrels};
use crate::customer::{CustState, Customer, QUEUE_SPOTS};
use crate::day_cycle::{give_start_package, GameState, Phase};
use crate::inventory::{HandItem, Inventory};
use crate::sprites::{NpcKind, Vessel};

/// Шаг обучения: текст инструкции и мировая точка цели (центр тайла).
pub struct TutStep {
    pub text: &'static str,
    pub target: (f32, f32),
}

pub const TUT_STEPS: [TutStep; 7] = [
    TutStep {
        text: "ШАГ 1/7. ВОЗЬМИТЕ СУХАРИ У ПОЛКИ (E)",
        target: (3.5, 6.5),
    },
    TutStep {
        text: "ШАГ 2/7. НАБЕРИТЕ ВОДУ У КОЛОНКИ (E)",
        target: (6.5, 6.5),
    },
    TutStep {
        text: "ШАГ 3/7. ВОЗЬМИТЕ САХАР С ИЗЮМОМ (E)",
        target: (5.5, 6.5),
    },
    TutStep {
        text: "ШАГ 4/7. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        target: (1.5, 4.5),
    },
    TutStep {
        text: "ШАГ 5/7. ВОЗЬМИТЕ СТАКАН СО СТЕЛЛАЖА (E)",
        target: (8.5, 6.5),
    },
    TutStep {
        text: "ШАГ 6/7. КВАС ГОТОВ: ДЕРЖИТЕ E У БОЧКИ",
        target: (1.5, 4.5),
    },
    TutStep {
        text: "ШАГ 7/7. ОТДАЙТЕ КВАС БАБУШКЕ (E)",
        target: (3.5, 2.7), // на шаге 7 цель — сама бабушка
    },
];

/// Логика обучения: шаги, спавн гостя, быстрое брожение, переход на склад.
pub fn tutorial_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    customers: Query<(Entity, &Customer)>,
    mut commands: Commands,
) {
    if state.phase != Phase::Tutorial {
        return;
    }
    let dt = time.delta_secs();
    if state.msg_t > 0.0 {
        state.msg_t -= dt;
    }

    // пропуск: T сразу на склад (ENTER в меню — старт, тут не мешает)
    if keys.just_pressed(KeyCode::KeyT) {
        finish(&mut state, &mut inv, &customers, &mut commands);
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

    // последний шаг: появляется бабушка, пока её нет
    if state.tut_step == 6 && customers.iter().count() == 0 {
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
        4 => inv
            .hands
            .iter()
            .any(|h| matches!(h, Some(HandItem::Empty(_, _)))),
        5 => inv
            .hands
            .iter()
            .any(|h| matches!(h, Some(HandItem::Full(_, _, _)))),
        6 => customers.iter().any(|(_, c)| c.served),
        _ => false,
    };
    if done {
        if state.tut_step >= 6 {
            finish(&mut state, &mut inv, &customers, &mut commands);
        } else {
            state.tut_step += 1;
            state.notify("ТАК ДЕРЖАТЬ!".into());
        }
    }
}

fn finish(
    state: &mut GameState,
    inv: &mut Inventory,
    customers: &Query<(Entity, &Customer)>,
    commands: &mut Commands,
) {
    state.tut_done = true;
    state.tut_step = 0;
    for (e, _) in customers.iter() {
        commands.entity(e).despawn();
    }
    give_start_package(state, inv);
    state.phase = Phase::Shopping;
    state.notify("ОБУЧЕНИЕ ГОТОВО! СЫРЬЁ: 1-3, ЗАТЕМ ENTER".into());
}
