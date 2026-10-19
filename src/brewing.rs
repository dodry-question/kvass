//! Варка кваса: фазы бочки (ПУСТАЯ → БРОДИТ → ГОТОВА → ПЕРЕБРОД → ИСПОРЧЕНА).

use std::collections::HashMap;

use bevy::prelude::*;
use rand::prelude::*;

use crate::map::Tile;

pub const BREW_TIME: f32 = 75.0; // 60–90 сек по ТЗ
pub const READY_WINDOW: f32 = 20.0; // окно, пока квас ещё хороший после готовности
pub const OVERRIPE_TIME: f32 = 30.0; // через сколько переброд окончательно пропадает
pub const BARREL_PORTIONS: u32 = 20;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BarrelPhase {
    /// Пустая, можно варить (step: 0 пусто, 1 засыпаны сухари, 2 залита вода).
    Empty,
    /// Бродит.
    Ferment(f32),
    /// Готова, кран открыт.
    Ready(f32),
    /// Переброд: ещё можно спасти, разливая.
    Overripe(f32),
    /// Испорчена, надо мыть.
    Ruined,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BarrelSignal {
    /// За 10 сек до готовности — «пшшш».
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
    /// Последний сигнал (прочитать и очистить).
    pub signal: Option<BarrelSignal>,
    pub flash: f32,
}

impl Barrel {
    pub fn new() -> Self {
        Self {
            phase: BarrelPhase::Empty,
            step: 0,
            portions: 0,
            signal: None,
            flash: 0.0,
        }
    }

    pub fn can_fill(&self) -> bool {
        matches!(self.phase, BarrelPhase::Ready(_) | BarrelPhase::Overripe(_))
            && self.portions > 0
    }

    pub fn overripe_fill(&self) -> bool {
        matches!(self.phase, BarrelPhase::Overripe(_))
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

/// Прогресс брожения/переброда.
pub fn barrel_update(
    time: Res<Time>,
    mut barrels: ResMut<Barrels>,
    mut dirty: ResMut<crate::day_cycle::Dirty>,
) {
    let dt = time.delta_secs();
    for b in barrels.map.values_mut() {
        b.flash = (b.flash - dt).max(0.0);
        match &mut b.phase {
            BarrelPhase::Ferment(t) => {
                *t -= dt;
                if *t <= 10.0 && *t + dt > 10.0 {
                    b.signal = Some(BarrelSignal::AlmostReady);
                }
                if *t <= 0.0 {
                    b.phase = BarrelPhase::Ready(READY_WINDOW);
                    b.portions = BARREL_PORTIONS;
                    b.signal = Some(BarrelSignal::Ready);
                }
            }
            BarrelPhase::Ready(t) => {
                *t -= dt;
                if *t <= 0.0 {
                    b.phase = BarrelPhase::Overripe(OVERRIPE_TIME);
                    b.signal = Some(BarrelSignal::OverripeStart);
                }
            }
            BarrelPhase::Overripe(t) => {
                *t -= dt;
                if *t <= 0.0 {
                    b.phase = BarrelPhase::Ruined;
                    b.signal = Some(BarrelSignal::Ruined);
                    dirty.0 += 1;
                }
            }
            _ => {}
        }
    }
}

/// Процент брожения для HUD (0..1), None если не бродит.
pub fn ferment_progress(b: &Barrel) -> Option<f32> {
    match b.phase {
        BarrelPhase::Ferment(t) => Some(1.0 - t / BREW_TIME),
        _ => None,
    }
}

pub fn random_brew_time(rng: &mut StdRng) -> f32 {
    rng.random_range(60.0..90.0)
}
