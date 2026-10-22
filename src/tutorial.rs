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

pub const TUT_STEPS: [TutStep; 10] = [
    TutStep {
        text: "ШАГ 1/10. ВОЗЬМИТЕ СУХАРИ У ПОЛКИ (E)",
        target: (3.5, 6.5),
    },
    TutStep {
        text: "ШАГ 2/10. НАБЕРИТЕ ВОДУ У КОЛОНКИ (E)",
        target: (9.5, 6.5),
    },
    TutStep {
        text: "ШАГ 3/10. ВОЗЬМИТЕ САХАР С ИЗЮМОМ (E)",
        target: (5.5, 6.5),
    },
    TutStep {
        text: "ШАГ 4/10. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        target: (1.5, 5.5),
    },
    // ТЗ №2: отдельный шаг на мини-игру ускорения брожения.
    TutStep {
        text: "ШАГ 5/10. НАЖИМАЙТЕ ПРОБЕЛ В ЗОНУ БЕГУНКА",
        target: (1.5, 5.5),
    },
    TutStep {
        text: "ШАГ 6/10. ВОЗЬМИТЕ СТАКАН СО СТЕЛЛАЖА (E)",
        target: (6.5, 6.5),
    },
    TutStep {
        text: "ШАГ 7/10. КВАС ГОТОВ: ДЕРЖИТЕ E У БОЧКИ",
        target: (1.5, 5.5),
    },
    TutStep {
        text: "ШАГ 8/10. ОТДАЙТЕ КВАС БАБУШКЕ (E)",
        target: (9.5, 2.7), // на шаге 8 цель — сама бабушка у прилавка
    },
    // После гостя появляется грязная посуда: ведём к мойке.
    TutStep {
        text: "ШАГ 9/10. ПОСЛЕ ГОСТЯ ГРЯЗНАЯ ПОСУДА — ПОМОЙТЕ СТАКАН (E)",
        target: (1.5, 4.5), // мойка (1,4)
    },
    // И сразу — мусорка: руки заполняются хламом, его надо выбросить.
    TutStep {
        text: "ШАГ 10/10. ВЕСЬ ХЛАМ — В МУСОРКУ: ВЫБРОСЬТЕ (E)",
        target: (2.5, 6.5), // урна у южной стены
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
    mut cam: ResMut<crate::raycaster::GameCam>,
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
    // ТЗ меню: обучение, запущенное кнопкой ОБУЧЕНИЕ (tut_locked), не
    // пропускается вообще — пройти придётся до конца.
    if keys.just_pressed(KeyCode::KeyT) && !state.tut_locked {
        if state.tut_step == 4 {
            state.tut_step = 5;
            state.notify("ТАК ДЕРЖАТЬ!".into());
        } else {
            finish(
                &mut state,
                &mut inv,
                &mut barrels,
                &customers,
                &mut commands,
                &mut dirty,
                &mut cam,
            );
        }
        return;
    }

    // в обучении квас бродит быстро (осталось 8 сек), чтобы не ждать минуту.
    // ТЗ (зависание обучения): на шаге QTE рост капнут на 80% — дальше только
    // удары по ПРОБЕЛУ (+10%), иначе бочка уходит в Ready и шаг зависает.
    for b in barrels.map.values_mut() {
        if state.tut_step == 4 {
            match &mut b.phase {
                BarrelPhase::Ferment(p) => *p = (*p).min(80.0),
                BarrelPhase::Ready(_) | BarrelPhase::Overripe(_) => {
                    b.phase = BarrelPhase::Ferment(80.0)
                }
                _ => {}
            }
        } else if let BarrelPhase::Ferment(p) = &mut b.phase {
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
        // шаг 9: грязный стакан после бабушки отмыт в мойке
        8 => inv.dirty_glasses == 0,
        // шаг 10: весь учебный хлам выброшен в мусорку
        9 => inv.is_empty(),
        _ => false,
    };
    if done {
        if state.tut_step >= 9 {
            finish(
                &mut state,
                &mut inv,
                &mut barrels,
                &customers,
                &mut commands,
                &mut dirty,
                &mut cam,
            );
        } else {
            state.tut_step += 1;
            match state.tut_step {
                8 => state.notify(
                    "ПОСЛЕ ГОСТЯ ОСТАЛАСЬ ГРЯЗНАЯ ПОСУДА — ОТНЕСИТЕ СТАКАН В МОЙКУ".into(),
                ),
                9 => {
                    fill_trash_practice(&mut inv);
                    state.notify(
                        "РУКИ ПОЛНЫ ХЛАМА. ВЫБРОСЬТЕ ВСЁ В МУСОРКУ У СТЕНЫ (E)".into(),
                    );
                }
                _ => state.notify("ТАК ДЕРЖАТЬ!".into()),
            }
        }
    }
}

/// Учебная практика с мусоркой: заполняем все четыре ячейки разным хламом,
/// чтобы игрок по одному выбросил его и запомнил, где урна.
fn fill_trash_practice(inv: &mut Inventory) {
    inv.hands = [
        Some(HandItem::Croutons(1)),
        Some(HandItem::Sugar(1)),
        Some(HandItem::Empty(Vessel::Jar, 1)),
        Some(HandItem::Empty(Vessel::Bottle, 1)),
    ];
    inv.selected = 0;
}

fn finish(
    state: &mut GameState,
    inv: &mut Inventory,
    barrels: &mut Barrels,
    customers: &Query<(Entity, &Customer)>,
    commands: &mut Commands,
    dirty: &mut crate::day_cycle::Dirty,
    cam: &mut crate::raycaster::GameCam,
) {
    state.tut_done = true;
    state.tut_step = 0;
    for (e, _) in customers.iter() {
        commands.entity(e).despawn();
    }
    if state.tut_after_game {
        // обучение — вход в основной цикл: день стартует сразу
        give_start_package(state, inv);
        // ТЗ №3: день начинается сразу, без экрана закупки.
        // start_day сам вернёт игрока на стартовую точку и включит
        // заставку «ДЕНЬ 1» с медленным проявлением магазина.
        start_day(state, barrels, dirty, cam);
    } else {
        // обучение из меню — отдельная секция: возвращаемся в главное меню
        state.phase = Phase::Menu;
        state.menu_view = crate::day_cycle::MenuView::Main;
        state.menu_sel = 0;
        state.tut_locked = false;
        state.notify("ОБУЧЕНИЕ ПРОЙДЕНО! ЖДЁМ ВАС ЗА ПРИЛАВКОМ".into());
    }
}
