//! Варка кваса: фазы бочки (ПУСТАЯ → БРОДИТ → ГОТОВА → ПЕРЕБРОД → ИСПОРЧЕНА).
//!
//! Брожение считается в **процентах**, а не в секундах: так одна шкала
//! описывает и обыденное брожение, и ускорение игроком (мини-игра на
//! ПРОБЕЛЕ даёт +`QTE_BONUS`% за попадание). Отсюда же пороги ТЗ:
//! 100% — готова, 125% — переброд, 150% — испорчена.

use std::collections::HashMap;

use bevy::prelude::*;
use rand::prelude::*;

use crate::map::Tile;

/// Секунд на 0..100% в обычном (авто-)режиме.
/// Ускорено в 2.5 раза (было 55 с): квас должен успевать созреть,
/// пока покупатель стоит в очереди.
pub const BREW_TIME: f32 = 22.0;
/// Секунд на 100..125% при базовой скорости (окно, пока квас ещё хороший).
pub const READY_WINDOW: f32 = 24.0;
/// Секунд на 125..150% при базовой скорости.
pub const OVERRIPE_TIME: f32 = 34.0;
/// Порций кваса из одной варки (было 20 — уменьшено вдвое).
pub const BARREL_PORTIONS: u32 = 10;

/// Пороги в процентах.
pub const PCT_READY: f32 = 100.0;
pub const PCT_OVERRIPE: f32 = 125.0;
pub const PCT_RUINED: f32 = 150.0;
/// Прибавка за попадание в зону мини-игры ускорения.
pub const QTE_BONUS: f32 = 10.0;
/// ТЗ №5: 100..125% — окно идеального кваса, брожение ЗАМЕДЛЯЕТСЯ (×0.4):
/// спокойное время, чтобы разлить.
pub const IDEAL_RATE: f32 = 0.4;
/// ТЗ №5: 125..150% — ОПАСНАЯ ЗОНА ПЕРЕБРОДА: скорость снова УСКОРЯЕТСЯ (×1.8).
/// Квас стремительно скисает: либо срочно ведро с водой (охлаждение),
/// либо разливать, пока не стало150%.
pub const DANGER_RATE: f32 = 1.8;

/// Первый переброд в игре — обучающий «стоп-кадр»: день и покупатели
/// замирают, а бочка доползает от 125% до 150% медленно (`FIRST_OVERBREW_RATE`
/// от обычной опасной скорости), чтобы игрок успел сбегать за ведром.
/// Это единственная поблажка; дальше переброд идёт с обычной `DANGER_RATE`.
pub const FIRST_OVERBREW_RATE: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BarrelPhase {
    /// Пустая, можно варить (step: 0 пусто, 1 засыпаны сухари, 2 залита вода).
    Empty,
    /// Бродит: проценты 0..100.
    Ferment(f32),
    /// Готова: проценты 100..125.
    Ready(f32),
    /// Переброд (125%): ещё можно спасти, разливая.
    Overripe(f32),
    /// Испорчена (150%), надо мыть.
    Ruined,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BarrelSignal {
    /// За ~10% до готовности — «пшшш».
    AlmostReady,
    /// Готова — «плюх/пип».
    Ready,
    /// Начало переброда — шипение.
    OverripeStart,
    /// Испорчена — «фьююю».
    Ruined,
}

#[derive(Clone, Debug)]
pub struct Barrel {
    pub phase: BarrelPhase,
    /// Шаг рецепта: 0 пусто, 1 сухари, 2 вода.
    pub step: u8,
    pub portions: u32,
    /// ТЗ №1: квас кончился раньше, чем бочка испортилась — слили всё.
    /// Такое состояние сразу помечается ИСПОРЧЕННОЙ («КВАС КОНЧИЛСЯ»),
    /// иначе игрок стоял бы у «ГОТОВОЙ», но пустой бочки без подсказки.
    pub ran_out: bool,
    /// Последний сигнал (прочитать и очистить).
    pub signal: Option<BarrelSignal>,
    pub flash: f32,
    /// Секунд этой варки на 0..100% (60–90 по ТЗ, рандом при закладке).
    pub brew_time: f32,
}

impl Barrel {
    pub fn new() -> Self {
        Self {
            phase: BarrelPhase::Empty,
            step: 0,
            portions: 0,
            ran_out: false,
            signal: None,
            flash: 0.0,
            brew_time: BREW_TIME,
        }
    }

    pub fn can_fill(&self) -> bool {
        matches!(self.phase, BarrelPhase::Ready(_) | BarrelPhase::Overripe(_))
            && self.portions > 0
    }

    pub fn overripe_fill(&self) -> bool {
        matches!(self.phase, BarrelPhase::Overripe(_))
    }

    /// Бродит ли (можно ускорять мини-игрой).
    ///
    /// Готовую бочку ускорять нельзя: иначе игрок в 118% бьёт ПРОБЕЛ,
    /// получает +10% и незаметно для себя уходит в переброд. После 100%
    /// остаётся только разливать (E) или ждать.
    pub fn boostable(&self) -> bool {
        matches!(self.phase, BarrelPhase::Ferment(_))
    }

    /// Прибавить QTE_BONUS% — если дошли до 100%, фаза сменится в
    /// `barrel_update` на следующем кадре (сигнал тоже там).
    pub fn boost(&mut self) {
        match &mut self.phase {
            BarrelPhase::Ferment(p) => *p += QTE_BONUS,
            _ => {}
        }
    }
}

#[derive(Resource)]
pub struct Barrels {
    pub map: HashMap<(i32, i32), Barrel>,
}

/// Одноразовые подсказки, которые показываются ровно в тот момент, когда
/// они нужны: при первом 100% и при подходе к переброду (120%).
/// HUD остаётся основным индикатором, а это — «обучение в нужный момент».
#[derive(Resource, Default)]
pub struct BrewHints {
    pub ready: bool,
    pub danger: bool,
}

/// Сценарий первого переброда (только день 1 и только в обычной игре).
/// Пока `stage` — один из активных, игровое время и покупатели стоят,
/// но игрок свободно ходит и взаимодействует. Ведём его к колонке (если
/// ведра нет), затем к бочке; после воды время снова идёт.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FirstOverbrewStage {
    #[default]
    Inactive,
    /// Ведра нет — цель колонка.
    NeedBucket,
    /// Ведро есть — цель бочка, E поливает.
    PourWater,
    /// Не успел: бочка испорчена, время ждёт, пока её не вымоют.
    FailedWash,
    /// Сценарий закончен, больше не запускается.
    Done,
}

#[derive(Resource, Default)]
pub struct FirstOverbrew {
    pub stage: FirstOverbrewStage,
    pub barrel: (i32, i32),
}

impl FirstOverbrew {
    /// Держит ли сценарий игровое время на паузе.
    pub fn freeze(&self) -> bool {
        matches!(
            self.stage,
            FirstOverbrewStage::NeedBucket
                | FirstOverbrewStage::PourWater
                | FirstOverbrewStage::FailedWash
        )
    }

    /// Активная фаза «спасаем квас» (ещё не поздно).
    pub fn saving(&self) -> bool {
        matches!(
            self.stage,
            FirstOverbrewStage::NeedBucket | FirstOverbrewStage::PourWater
        )
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Barrels {
    pub fn from_map(map: &crate::map::Map) -> Self {
        let mut m = HashMap::new();
        for y in 0..crate::map::MAP_H {
            for x in 0..crate::map::MAP_W {
                if map.get(x as i32, y as i32) == Tile::Barrel {
                    m.insert((x as i32, y as i32), Barrel::new());
                }
            }
        }
        Self { map: m }
    }

    /// Ближайшая бочка к точке.
    pub fn nearest(&self, x: f32, y: f32) -> Option<((i32, i32), &Barrel)> {
        self.map
            .iter()
            .min_by(|(a, _), (b, _)| {
                let da = (a.0 as f32 + 0.5 - x).powi(2) + (a.1 as f32 + 0.5 - y).powi(2);
                let db = (b.0 as f32 + 0.5 - x).powi(2) + (b.1 as f32 + 0.5 - y).powi(2);
                da.partial_cmp(&db).unwrap()
            })
            .map(|(k, v)| (*k, v))
    }

    pub fn mutable_at(&mut self, tx: i32, ty: i32) -> Option<&mut Barrel> {
        self.map.get_mut(&(tx, ty))
    }
}

/// Мини-игра ускорения брожения: игрок подошёл к бочке и жмёт ПРОБЕЛ,
/// пока бегунок в выделенной зоне. Не подходил — брожение идёт как обычно.
#[derive(Resource)]
pub struct BrewQte {
    pub active: bool,
    /// Позиция бегунка по полоске, 0..1.
    pub pos: f32,
    pub dir: f32,
    /// Подсвеченная зона, 0..1.
    pub zone0: f32,
    pub zone1: f32,
    pub barrel: Option<(i32, i32)>,
    /// Защита от повторного срабатывания на том же нажатии.
    pub lock: f32,
    /// >0 — вспышка «попал», <0 — «мимо».
    pub flash: f32,
}

impl Default for BrewQte {
    fn default() -> Self {
        Self {
            active: false,
            pos: 0.0,
            dir: 1.0,
            zone0: 0.42,
            zone1: 0.54,
            barrel: None,
            lock: 0.0,
            flash: 0.0,
        }
    }
}

impl BrewQte {
    pub fn roll_zone(&mut self, rng: &mut StdRng) {
        // зона ~11–17% полоски, в пределах 0.06..0.94
        let len = rng.random_range(0.11..0.17);
        let s = rng.random_range(0.06..(0.94 - len));
        self.zone0 = s;
        self.zone1 = s + len;
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.barrel = None;
        self.flash = 0.0;
    }
}

/// Прогресс брожения/переброда (проценты, 0..150+).
pub fn barrel_update(
    time: Res<Time>,
    mut barrels: ResMut<Barrels>,
    mut dirty: ResMut<crate::day_cycle::Dirty>,
    overbrew: Res<FirstOverbrew>,
) {
    let dt = time.delta_secs();
    // Пока идёт первый переброд, весь мир (и другие бочки) замирает.
    // Медленно продолжает только проблемная бочка — чтобы игрок успел.
    let frozen = overbrew.freeze();
    let event_barrel = overbrew.barrel;
    for (pos, b) in barrels.map.iter_mut() {
        if frozen && *pos != event_barrel {
            continue;
        }
        b.flash = (b.flash - dt).max(0.0);
        match &mut b.phase {
            BarrelPhase::Ferment(p) => {
                let prev = *p;
                *p += dt * 100.0 / b.brew_time.max(1.0);
                if prev < 90.0 && *p >= 90.0 {
                    b.signal = Some(BarrelSignal::AlmostReady);
                }
                if *p >= PCT_READY {
                    let q = *p;
                    b.phase = BarrelPhase::Ready(q);
                    b.portions = BARREL_PORTIONS;
                    b.ran_out = false; // новая варка — флаг «квас кончился» снят
                    b.signal = Some(BarrelSignal::Ready);
                }
            }
            BarrelPhase::Ready(p) => {
                // ТЗ №5: окно идеального кваса — замедленно (×0.4).
                *p += dt * (PCT_OVERRIPE - PCT_READY) / READY_WINDOW * IDEAL_RATE;
                if *p >= PCT_OVERRIPE {
                    let q = *p;
                    b.phase = BarrelPhase::Overripe(q);
                    b.signal = Some(BarrelSignal::OverripeStart);
                }
            }
            BarrelPhase::Overripe(p) => {
                // ТЗ №5: опасная зона — брожение ускоряется (×1.8), квас
                // стремительно скисает: успей охладить или разлить!
                // Первый переброд дня 1 — обучающий, поэтому медленный.
                let rate = if frozen && *pos == event_barrel {
                    FIRST_OVERBREW_RATE
                } else {
                    DANGER_RATE
                };
                *p += dt * (PCT_RUINED - PCT_OVERRIPE) / OVERRIPE_TIME * rate;
                if *p >= PCT_RUINED {
                    b.phase = BarrelPhase::Ruined;
                    b.ran_out = false; // порча по времени, а не «слили всё»
                    b.signal = Some(BarrelSignal::Ruined);
                    dirty.0 += 1;
                }
            }
            _ => {}
        }
    }
}

/// Сценарий первого переброда. Запускается сам на дне 1, когда любая
/// бочка впервые пересекает 125%. На время сценария день и покупатели
/// стоят; бочка медленно портится. Игрок либо приносит ведро и поливает
/// (E), либо не успевает — тогда обязан вымыть испорченную бочку.
pub fn first_overbrew_event(
    mut state: ResMut<crate::day_cycle::GameState>,
    mut event: ResMut<FirstOverbrew>,
    barrels: Res<Barrels>,
    inv: Res<crate::inventory::Inventory>,
) {
    if state.phase != crate::day_cycle::Phase::Playing {
        return;
    }
    // Новый заход на день 1 (старт из меню/после обучения) — сценарий снова
    // доступен. Во время самого события day_intro уже 0, так что сброса нет.
    if state.day == 1
        && state.day_intro > 0.0
        && event.stage != FirstOverbrewStage::Inactive
    {
        event.reset();
    }

    // Старт: первая бочка, ушедшая в переброд на дне 1.
    if event.stage == FirstOverbrewStage::Inactive
        && state.day == 1
        && state.day_intro <= 0.0
    {
        let found = barrels
            .map
            .iter()
            .filter(|(_, b)| matches!(b.phase, BarrelPhase::Overripe(_)))
            .min_by_key(|(k, _)| **k)
            .map(|(k, _)| *k);
        if let Some(pos) = found {
            event.barrel = pos;
            event.stage = if inv.has_bucket() {
                FirstOverbrewStage::PourWater
            } else {
                FirstOverbrewStage::NeedBucket
            };
            let text = if inv.has_bucket() {
                "ПЕРВЫЙ ПЕРЕБРОД! ВЕДРО ЕСТЬ - БЕГИТЕ К БОЧКЕ И ПОЛЕЙТЕ (E)"
            } else {
                "ПЕРВЫЙ ПЕРЕБРОД! ВРЕМЯ ОСТАНОВЛЕНО. НАБЕРИТЕ ВЕДРО У КОЛОНКИ (E)"
            };
            state.notify(text.into());
            state.msg_t = 5.0;
            state.face_set(2, 1.5); // герой встревожен
        }
        return;
    }

    if !event.freeze() {
        return;
    }

    let Some(b) = barrels.map.get(&event.barrel) else {
        event.stage = FirstOverbrewStage::Done;
        return;
    };

    match event.stage {
        FirstOverbrewStage::NeedBucket => {
            if b.phase == BarrelPhase::Ruined {
                event.stage = FirstOverbrewStage::FailedWash;
                state.notify(
                    "ВЫ НЕ УСПЕЛИ ПОЛИТЬ КВАС - ОН ИСПОРТИЛСЯ. ПОМОЙТЕ БОЧКУ (ДЕРЖИТЕ E)"
                        .into(),
                );
                state.msg_t = 5.0;
                state.face_set(2, 2.0);
            } else if !matches!(b.phase, BarrelPhase::Overripe(_)) {
                // Кто-то успел спасти бочку другим способом.
                event.stage = FirstOverbrewStage::Done;
                state.notify("КВАС СПАСЁН - ВРЕМЯ СНОВА ИДЁТ".into());
                state.msg_t = 4.0;
            } else if inv.has_bucket() {
                event.stage = FirstOverbrewStage::PourWater;
                state.notify("ВЕДРО НАБРАНО - БЕГИТЕ К БОЧКЕ И НАЖМИТЕ E".into());
                state.msg_t = 4.0;
            }
        }
        FirstOverbrewStage::PourWater => {
            if b.phase == BarrelPhase::Ruined {
                event.stage = FirstOverbrewStage::FailedWash;
                state.notify(
                    "ВЫ НЕ УСПЕЛИ ПОЛИТЬ КВАС - ОН ИСПОРТИЛСЯ. ПОМОЙТЕ БОЧКУ (ДЕРЖИТЕ E)"
                        .into(),
                );
                state.msg_t = 5.0;
                state.face_set(2, 2.0);
            } else if !matches!(b.phase, BarrelPhase::Overripe(_)) {
                event.stage = FirstOverbrewStage::Done;
                state.notify("КВАС СПАСЁН - ВРЕМЯ СНОВА ИДЁТ".into());
                state.msg_t = 4.0;
            } else if !inv.has_bucket() {
                event.stage = FirstOverbrewStage::NeedBucket;
            }
        }
        FirstOverbrewStage::FailedWash => {
            // Успешная промывка: fill_hold превращает бочку в Empty.
            if b.phase == BarrelPhase::Empty {
                event.stage = FirstOverbrewStage::Done;
                state.notify("БОЧКА ВЫМЫТА - ВРЕМЯ СНОВА ИДЁТ".into());
                state.msg_t = 4.0;
                state.face_set(0, 0.0);
            }
        }
        FirstOverbrewStage::Inactive | FirstOverbrewStage::Done => {}
    }
}

/// Just-in-time обучение: первая готовая бочка и первое приближение
/// к переброду. Подсказки одноразовые, чтобы не спамить каждый день;
/// дальше игрок читает HUD (проценты и секунды до следующей фазы).
pub fn brew_hints(
    mut state: ResMut<crate::day_cycle::GameState>,
    barrels: Res<Barrels>,
    mut hints: ResMut<BrewHints>,
    overbrew: Res<FirstOverbrew>,
) {
    if !matches!(
        state.phase,
        crate::day_cycle::Phase::Playing | crate::day_cycle::Phase::Tutorial
    ) {
        return;
    }
    // Во время первого переброда говорит отдельный сценарий, не тосты.
    if overbrew.freeze() {
        return;
    }

    let mut ready: Option<&Barrel> = None;
    let mut danger: Option<&Barrel> = None;
    for b in barrels.map.values() {
        let BarrelPhase::Ready(p) = b.phase else {
            continue;
        };
        if ready.map_or(true, |r| match r.phase {
            BarrelPhase::Ready(q) => p > q,
            _ => true,
        }) {
            ready = Some(b);
        }
        if p >= 120.0
            && danger.map_or(true, |d| match d.phase {
                BarrelPhase::Ready(q) => p > q,
                _ => true,
            })
        {
            danger = Some(b);
        }
    }

    // Сначала сильное предупреждение: если игрок впервые видит бочку уже
    // на 120%+, объяснять «готово» поздно — важнее успеть разлить.
    if !hints.danger {
        if let Some(b) = danger {
            state.notify(format!(
                "ПЕРЕБРОД ЧЕРЕЗ {:.0} С! РАЗЛИВАЙ (E) ИЛИ ОХЛАДИ ВЕДРОМ",
                seconds_to_next(b)
            ));
            state.msg_t = 4.0;
            hints.danger = true;
            hints.ready = true;
            return;
        }
    }

    if !hints.ready && ready.is_some() {
        state.notify(
            "КВАС ГОТОВ! РАЗЛИВАЙ (E). ПОСЛЕ 125% - ПЕРЕБРОД, ЦЕНА -50%".into(),
        );
        hints.ready = true;
    }
}

/// Процент брожения для HUD (0..1.5), None если не бродит.
pub fn ferment_progress(b: &Barrel) -> Option<f32> {
    match b.phase {
        BarrelPhase::Ferment(p) => Some(p / 100.0),
        BarrelPhase::Ready(p) => Some(p / 100.0),
        BarrelPhase::Overripe(p) => Some(p / 100.0),
        _ => None,
    }
}

/// Секунд до переброда (для готовой) / до порчи (для перебродившей).
pub fn seconds_to_next(b: &Barrel) -> f32 {
    match b.phase {
        BarrelPhase::Ready(p) => ((PCT_OVERRIPE - p).max(0.0) / (PCT_OVERRIPE - PCT_READY))
            * READY_WINDOW
            / IDEAL_RATE,
        BarrelPhase::Overripe(p) => ((PCT_RUINED - p).max(0.0) / (PCT_RUINED - PCT_OVERRIPE))
            * OVERRIPE_TIME
            / DANGER_RATE,
        BarrelPhase::Ferment(p) => ((PCT_READY - p).max(0.0) / PCT_READY) * b.brew_time,
        _ => 0.0,
    }
}

/// Время брожения конкретной бочки: 18–28 с (было 45–70 с, ускорено
/// в 2.5 раза). Разброс оставлен — две бочки бродят по-разному.
pub fn random_brew_time(rng: &mut StdRng) -> f32 {
    rng.random_range(18.0..28.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::day_cycle::{day_system, Dirty, GameRng, GameState, Phase};
    use crate::inventory::{HandItem, Inventory};
    use crate::modals::Modals;

    fn test_world(phase: BarrelPhase) -> World {
        let mut world = World::default();
        let mut state = GameState::default();
        state.phase = Phase::Playing;
        state.day = 1;
        state.day_intro = 0.0;
        state.time_left = 100.0;
        world.insert_resource(state);
        world.insert_resource(FirstOverbrew::default());
        let mut barrel = Barrel::new();
        barrel.phase = phase;
        barrel.portions = if matches!(phase, BarrelPhase::Ready(_) | BarrelPhase::Overripe(_)) {
            BARREL_PORTIONS
        } else {
            0
        };
        world.insert_resource(Barrels {
            map: HashMap::from([((1, 1), barrel)]),
        });
        world.insert_resource(Inventory::default());
        world
    }

    fn run_event(world: &mut World) {
        world.run_system_once(first_overbrew_event).unwrap();
    }

    #[test]
    fn first_overbrew_goes_from_bucket_to_pour_to_done() {
        let mut world = test_world(BarrelPhase::Overripe(126.0));
        run_event(&mut world);
        let event = world.resource::<FirstOverbrew>();
        assert_eq!(event.stage, FirstOverbrewStage::NeedBucket);
        assert!(event.freeze());
        assert!(event.saving());

        world
            .resource_mut::<Inventory>()
            .put(HandItem::Bucket);
        run_event(&mut world);
        let event = world.resource::<FirstOverbrew>();
        assert_eq!(event.stage, FirstOverbrewStage::PourWater);
        assert!(event.freeze());

        // Игрок полил: interact забирает ведро и делает бочку готовой.
        world.resource_mut::<Inventory>().drain_bucket();
        world
            .resource_mut::<Barrels>()
            .map
            .get_mut(&(1, 1))
            .unwrap()
            .phase = BarrelPhase::Ready(101.0);
        run_event(&mut world);
        let event = world.resource::<FirstOverbrew>();
        assert_eq!(event.stage, FirstOverbrewStage::Done);
        assert!(!event.freeze());
    }

    #[test]
    fn first_overbrew_ruin_requires_wash_before_done() {
        let mut world = test_world(BarrelPhase::Overripe(149.0));
        run_event(&mut world);
        assert_eq!(
            world.resource::<FirstOverbrew>().stage,
            FirstOverbrewStage::NeedBucket
        );

        world
            .resource_mut::<Barrels>()
            .map
            .get_mut(&(1, 1))
            .unwrap()
            .phase = BarrelPhase::Ruined;
        run_event(&mut world);
        let event = world.resource::<FirstOverbrew>();
        assert_eq!(event.stage, FirstOverbrewStage::FailedWash);
        assert!(event.freeze());
        assert!(!event.saving());

        // fill_hold после мытья превращает бочку в Empty.
        world
            .resource_mut::<Barrels>()
            .map
            .get_mut(&(1, 1))
            .unwrap()
            .phase = BarrelPhase::Empty;
        run_event(&mut world);
        let event = world.resource::<FirstOverbrew>();
        assert_eq!(event.stage, FirstOverbrewStage::Done);
        assert!(!event.freeze());
    }

    #[test]
    fn first_overbrew_only_starts_on_day_one() {
        let mut world = test_world(BarrelPhase::Overripe(126.0));
        world.resource_mut::<GameState>().day = 2;
        run_event(&mut world);
        assert_eq!(
            world.resource::<FirstOverbrew>().stage,
            FirstOverbrewStage::Inactive
        );
    }

    #[test]
    fn day_system_stops_clock_until_overbrew_is_solved() {
        let mut world = World::default();
        let mut state = GameState::default();
        state.phase = Phase::Playing;
        state.day_intro = 0.0;
        state.time_left = 100.0;
        state.spawn_t = 100.0;
        world.insert_resource(state);
        world.insert_resource(FirstOverbrew {
            stage: FirstOverbrewStage::NeedBucket,
            barrel: (1, 1),
        });
        world.insert_resource(GameRng(StdRng::seed_from_u64(7)));
        world.insert_resource(Modals::default());
        world.insert_resource(crate::rush::Rush::default());
        world.insert_resource(Time::<()>::default());

        world
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs(1));
        world.run_system_once(day_system).unwrap();
        assert_eq!(world.resource::<GameState>().time_left, 100.0);

        world.resource_mut::<FirstOverbrew>().stage = FirstOverbrewStage::Done;
        world
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs(1));
        world.run_system_once(day_system).unwrap();
        assert!((world.resource::<GameState>().time_left - 99.0).abs() < 0.001);
    }

    #[test]
    fn first_overbrew_slows_target_and_freezes_other_barrels() {
        let mut world = World::default();
        let mut target = Barrel::new();
        target.phase = BarrelPhase::Overripe(PCT_OVERRIPE);
        let mut other = Barrel::new();
        other.phase = BarrelPhase::Overripe(PCT_OVERRIPE);
        world.insert_resource(Barrels {
            map: HashMap::from([((1, 1), target), ((2, 2), other)]),
        });
        world.insert_resource(FirstOverbrew {
            stage: FirstOverbrewStage::NeedBucket,
            barrel: (1, 1),
        });
        world.insert_resource(Dirty::default());
        world.insert_resource(Time::<()>::default());
        world
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs(1));

        world.run_system_once(barrel_update).unwrap();
        let barrels = world.resource::<Barrels>();
        let target_p = match barrels.map[&(1, 1)].phase {
            BarrelPhase::Overripe(p) => p,
            _ => panic!("target must stay overripe"),
        };
        let other_p = match barrels.map[&(2, 2)].phase {
            BarrelPhase::Overripe(p) => p,
            _ => panic!("other barrel must stay frozen"),
        };
        assert!((target_p - (PCT_OVERRIPE + 0.220588)).abs() < 0.01);
        assert_eq!(other_p, PCT_OVERRIPE);
    }
}
