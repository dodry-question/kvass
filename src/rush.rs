//! Третий день: «Заводской гудок / Час пик».
//!
//! В 13:00 по таймеру смены (ровно середина дня 3) раздаётся протяжный
//! заводской гудок: смена на местном заводе закончилась, и к лавке
//! прибегает толпа работяг в спецовках. Все требуют только трёхлитровые
//! банки — самый дорогой чек, но и самый большой расход кваса.
//!
//! Правила сцены:
//! - толпа 4–5 человек, а в очередь влезает трое, поэтому лишние ждут
//!   за дверью и заходят по одному, как только место освобождается;
//! - пока толпа не обслужена (и пока последний работяга не ушёл),
//!   обычные покупатели не приходят — очередь не разбавляется;
//! - бандиты в такой толпе не суются: коридорная сцена на это время
//!   выключена (`ambush_system` смотрит `Rush::active`);
//! - терпение у кассы короткое: работяги хотят пить после горячего цеха.

use std::collections::VecDeque;

use bevy::prelude::*;
use rand::prelude::*;

use crate::customer::{CustState, Customer, QUEUE_SPOTS};
use crate::day_cycle::{GameRng, GameState, Phase};
use crate::sprites::{NpcKind, Vessel};

/// Сцена только на третьем дне (день 1 — обучение, день 2 — бандиты).
pub const RUSH_DAY: u32 = 3;
/// 13:00 при смене 10:00–16:00: день 3 длится 360 с, значит до конца 180 с.
/// Оставляет три минуты на толпу — успеть разлить и обслужить реально.
pub const RUSH_AT_TIME_LEFT: f32 = 180.0;
/// Размер толпы.
pub const CROWD_MIN: u32 = 4;
pub const CROWD_MAX: u32 = 5;
/// Терпение работяги у кассы: короткое, но не «десять секунд».
pub const WORKER_PATIENCE: f32 = 45.0;

/// Метка работяги из толпы «Час пик».
#[derive(Component)]
pub struct RushWorker;

#[derive(Resource, Default)]
pub struct Rush {
    /// День, к которому относится состояние (сброс при смене дня).
    pub day: u32,
    /// Толпа ещё в лавке (или ещё не вся зашла).
    pub active: bool,
    /// Сколько всего пришло.
    pub total: u32,
    /// Сколько ещё ждут за дверью.
    pub waiting: u32,
    /// Сколько уже получили квас.
    pub served: u32,
    /// Гудок в этот день уже был.
    pub fired: bool,
    /// Кого уже посчитали обслуженным (чтобы счётчик не скакал,
    /// когда работяга уходит из лавки и сущность исчезает).
    pub counted: Vec<Entity>,
}

impl Rush {
    /// Новый день (в том числе отладочный `day N`) — сцена снова доступна.
    pub fn reset(&mut self, day: u32) {
        *self = Self {
            day,
            ..Self::default()
        };
    }

    /// Обычные покупатели не заходят, пока толпа не разошлась.
    pub fn blocks_customers(&self) -> bool {
        self.active
    }
}

/// Заводской гудок в 13:00 и запуск толпы.
pub fn rush_start(
    mut state: ResMut<GameState>,
    mut rush: ResMut<Rush>,
    mut commands: Commands,
    mut game_rng: ResMut<GameRng>,
    bank: Res<crate::audio::SoundBank>,
) {
    // Заставка «ДЕНЬ N» — новый день, сцену сбрасываем (и после `day N`).
    if state.phase == Phase::Playing && state.day_intro > 0.0 {
        if rush.day != state.day || rush.fired || rush.active {
            rush.reset(state.day);
        }
        return;
    }
    if state.phase != Phase::Playing || state.day != RUSH_DAY || rush.fired {
        return;
    }
    if state.time_left > RUSH_AT_TIME_LEFT {
        return;
    }
    rush.fired = true;
    rush.active = true;
    rush.total = game_rng.0.random_range(CROWD_MIN..=CROWD_MAX);
    rush.waiting = rush.total;
    rush.served = 0;
    crate::audio::play_sfx(&mut commands, &bank, "horn", 0.85);
    state.notify(format!(
        "ЗАВОДСКОЙ ГУДОК! 13:00 - ЧАС ПИК: {} РАБОТЯГ ЗА ТРЁХЛИТРОВЫМИ БАНКАМИ",
        rush.total
    ));
    state.msg_t = 5.0;
    state.face_set(2, 3.0);
}

/// Толпа: завести работяг в свободные места очереди и закрыть сцену,
/// когда последний ушёл.
pub fn rush_flow(
    mut rush: ResMut<Rush>,
    mut commands: Commands,
    mut state: ResMut<GameState>,
    customers: Query<&Customer, Without<RushWorker>>,
    workers: Query<(Entity, &Customer), With<RushWorker>>,
) {
    if !rush.active {
        return;
    }
    if state.phase != Phase::Playing {
        return;
    }

    // Занятые места очереди (уходящие место ещё держат).
    let mut taken = [false; QUEUE_SPOTS.len()];
    for c in customers.iter().chain(workers.iter().map(|(_, c)| c)) {
        if c.spot < taken.len() && c.state != CustState::Leaving {
            taken[c.spot] = true;
        }
    }
    let mut spawned_now = false;
    if rush.waiting > 0 {
        if let Some(spot) = (0..QUEUE_SPOTS.len()).find(|&i| !taken[i]) {
            spawn_worker(&mut commands, spot);
            rush.waiting -= 1;
            spawned_now = true;
        }
    }

    let alive = workers.iter().count();
    // Считаем каждого работягу ровно один раз: сущность исчезает после
    // ухода, и пересчёт «живых» обнулил бы счётчик в конце сцены.
    for (e, c) in workers.iter() {
        if c.served && !rush.counted.contains(&e) {
            rush.counted.push(e);
            rush.served += 1;
        }
    }
    if rush.waiting == 0 && alive == 0 && !spawned_now {
        // Спавн применяется в конце кадра: только что заведённый работяга
        // ещё не виден в запросе — без `spawned_now` сцена закрылась бы
        // в тот же кадр, что и последний заход.
        rush.active = false;
        state.notify(format!(
            "ЧАС ПИК ПРОШЁЛ: ОБСЛУЖЕНО {}/{}",
            rush.served, rush.total
        ));
        state.msg_t = 4.0;
        if rush.served == rush.total {
            state.face_set(1, 2.5);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rush_starts_at_midday_and_leaves_time_to_serve() {
        let len = crate::day_cycle::day_len(RUSH_DAY);
        // 13:00 = середина смены 10:00–16:00.
        assert!((len - RUSH_AT_TIME_LEFT * 2.0).abs() < 1.0);
        // На разлив и обслуживание толпы должно оставаться не меньше
        // двух минут — иначе сцену физически не пройти.
        assert!(RUSH_AT_TIME_LEFT >= 120.0);
        assert!(CROWD_MIN >= 4 && CROWD_MAX >= CROWD_MIN);
        assert!(WORKER_PATIENCE >= 30.0);
    }

    #[test]
    fn crowd_fits_barrel_and_queue_overflows() {
        // Бочка — 10 порций, банка — 2: толпа из 5 выпивает бочку целиком.
        let max_portions = crate::brewing::BARREL_PORTIONS;
        let per_worker = Vessel::Jar.portions();
        assert!(CROWD_MAX * per_worker <= max_portions);
        // Лишние работяги ждут за дверью, а не ломают очередь.
        assert!(CROWD_MAX as usize > QUEUE_SPOTS.len());
    }

    #[test]
    fn crowd_waits_outside_the_door_and_finishes() {
        let mut app = App::new();
        app.insert_resource(GameState {
            day: RUSH_DAY,
            phase: Phase::Playing,
            time_left: RUSH_AT_TIME_LEFT - 30.0,
            ..Default::default()
        });
        app.insert_resource(Rush {
            day: RUSH_DAY,
            active: true,
            total: 5,
            waiting: 5,
            ..Default::default()
        });
        app.add_systems(Update, rush_flow);

        // Очередь — три места: трое заходят, двое ждут за дверью.
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().resource::<Rush>().waiting, 2);
        assert!(app.world().resource::<Rush>().active);

        // Все ушли (обслужены или нет) — сцена закрывается, счётчик не скачет.
        let workers: Vec<Entity> = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<RushWorker>>();
            q.iter(world).collect()
        };
        assert_eq!(workers.len(), 3);
        // Первого отмечаем обслуженным: счётчик должен пережить его уход.
        app.world_mut()
            .entity_mut(workers[0])
            .get_mut::<Customer>()
            .unwrap()
            .served = true;
        app.update();

        // Забираем вошедших (как будто они ушли) и ждём, пока толпа
        // не зайдёт целиком: после последнего захода сцена ещё идёт.
        for _ in 0..3 {
            let gone: Vec<Entity> = {
                let world = app.world_mut();
                let mut q = world.query_filtered::<Entity, With<RushWorker>>();
                q.iter(world).collect()
            };
            for e in gone {
                app.world_mut().entity_mut(e).despawn();
            }
            app.update();
            if app.world().resource::<Rush>().waiting == 0 {
                assert!(
                    app.world().resource::<Rush>().active,
                    "последний работяга только зашёл — сцена ещё не закрыта"
                );
                break;
            }
        }
        // Последний уходит — вот теперь сцена закрывается.
        let gone: Vec<Entity> = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<RushWorker>>();
            q.iter(world).collect()
        };
        for e in gone {
            app.world_mut().entity_mut(e).despawn();
        }
        app.update();

        let rush = app.world().resource::<Rush>();
        assert!(!rush.active, "толпа разошлась — сцена закрыта");
        assert_eq!(rush.waiting, 0);
        assert_eq!(rush.served, 1, "счётчик обслуженных не сбрасывается");
    }
}

fn spawn_worker(commands: &mut Commands, spot: usize) {
    let c = Customer {
        kind: NpcKind::Worker,
        order: Vessel::Jar,
        state: CustState::Entering,
        pos: crate::customer::DOOR,
        path: VecDeque::from([Vec2::new(QUEUE_SPOTS[spot].0, QUEUE_SPOTS[spot].1)]),
        patience: WORKER_PATIENCE,
        max_patience: WORKER_PATIENCE,
        queue_patience: WORKER_PATIENCE,
        bubble: None,
        bubble_t: 0.0,
        spot,
        served: false,
        step_t: 0.0,
    };
    commands.spawn((c, RushWorker));
}
