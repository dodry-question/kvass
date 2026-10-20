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
pub const BREW_TIME: f32 = 55.0;
/// Секунд на 100..125% при базовой скорости (окно, пока квас ещё хороший).
pub const READY_WINDOW: f32 = 24.0;
/// Секунд на 125..150% при базовой скорости.
pub const OVERRIPE_TIME: f32 = 34.0;
pub const BARREL_PORTIONS: u32 = 20;

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
    pub fn boostable(&self) -> bool {
        matches!(self.phase, BarrelPhase::Ferment(_) | BarrelPhase::Ready(_))
    }

    /// Прибавить QTE_BONUS% — если дошли до 125/150, фаза сменится в
    /// `barrel_update` на следующем кадре (сигнал тоже там).
    pub fn boost(&mut self) {
        match &mut self.phase {
            BarrelPhase::Ferment(p) => *p += QTE_BONUS,
            BarrelPhase::Ready(p) => *p += QTE_BONUS,
            _ => {}
        }
    }
}

#[derive(Resource)]
pub struct Barrels {
    pub map: HashMap<(i32, i32), Barrel>,
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
) {
    let dt = time.delta_secs();
    for b in barrels.map.values_mut() {
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
                *p += dt * (PCT_RUINED - PCT_OVERRIPE) / OVERRIPE_TIME * DANGER_RATE;
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

/// Время брожения конкретной бочки. Сокращено с 60–90 до 45–70: на первом
/// дне покупатель ждёт ~78 секунд, и квас обязан успевать созреть, пока
/// он в очереди. Разброс оставлен — две бочки бродят по-разному.
pub fn random_brew_time(rng: &mut StdRng) -> f32 {
    rng.random_range(45.0..70.0)
}
