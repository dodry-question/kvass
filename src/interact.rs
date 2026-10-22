//! Взаимодействие (E / ЛКМ): станции, бочка, розлив, выдача покупателям.

use bevy::prelude::*;
use rand::prelude::*;

use crate::brewing::{BarrelPhase, Barrels};
use crate::customer::{CustState, Customer};
use crate::day_cycle::{GameState, GameRng, Phase};
use crate::inventory::{HandItem, Inventory};
use crate::map::Tile;
use crate::modals::{Modal, Modals};
use crate::raycaster::GameCam;
use crate::sprites::NpcKind;

/// Прогресс розлива/промывки бочки.
#[derive(Resource, Default)]
pub struct FillState {
    pub progress: f32,
    pub target: (i32, i32),
}

/// Мини-игра мытья посуды (ТЗ №4) живёт в `modals::Modal::Sponge`:
/// губка ходит по сетке грязи и стирает её, «дзынь» возвращает посуду
/// на склад. Из этой системы моется только бочка (удержание E).

/// Мини-игра охлаждения перебродившей бочки (ТЗ №9): ведро ледяной воды,
/// горизонтальная полоска с синей зоной. ПРОБЕЛ в зоне — температура
/// падает, переброд −25% (130→105), ведро опустевает (набрать у колонки).
#[derive(Resource, Default)]
pub struct CoolQte {
    pub active: bool,
    pub pos: f32,
    pub dir: f32,
    pub zone0: f32,
    pub zone1: f32,
    pub barrel: Option<(i32, i32)>,
    pub lock: f32,
    /// >0 — вспышка «охладил», <0 — «мимо».
    pub flash: f32,
}

impl CoolQte {
    pub fn roll_zone(&mut self, rng: &mut StdRng) {
        let len = rng.random_range(0.13..0.20);
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

fn front_tile(cam: &GameCam) -> (i32, i32) {
    let fx = cam.x + cam.angle.cos() * 0.9;
    let fy = cam.y + cam.angle.sin() * 0.9;
    (fx.floor() as i32, fy.floor() as i32)
}

pub fn interact(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    cam: Res<GameCam>,
    map: Res<crate::map::Map>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut modal: ResMut<Modals>,
    mut cool: ResMut<CoolQte>,
    mut customers: Query<(Entity, &mut Customer)>,
    props: Query<&crate::sprites::WorldProp>,
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
            // ТЗ №4: паузим мир и открываем модалку выбора посуды.
            modal.modal = Modal::VesselPick;
            keys.clear(); // ТЗ №1: та же E не закроет модалку в этот же кадр
            state.notify("СТЕЛЛАЖ ПОСУДЫ: [1] СТАКАН [2] БАНКА [3] БУТЫЛКА".into());
        }
        Tile::Supply => {
            // ТЗ №3: окно снабжения — закупка сырья с доставкой через 3–5 сек.
            modal.modal = Modal::Supply;
            keys.clear(); // ТЗ №1
            state.notify("ОКНО СНАБЖЕНИЯ: [1] СУХАРИ [2] САХАР".into());
        }
        Tile::Sink => {
            // ТЗ №4: мойка — отдельная модалка с губкой (мир на паузе).
            if inv.total_dirty() == 0 {
                state.notify("ГРЯЗНОЙ ПОСУДЫ НЕТ — МОЙКА ЧИСТА".into());
            } else {
                modal.modal = Modal::Sponge;
                modal.sponge = crate::modals::Sponge::default(); // выбор посуды
                keys.clear(); // ТЗ №1: та же E не закроет модалку в этот же кадр
                state.notify(format!(
                    "МЫТЁ ПОСУДЫ: [1] СТАКАН {} [2] БАНКА {} [3] БУТЫЛКА {}",
                    inv.dirty_glasses, inv.dirty_jars, inv.dirty_bottles
                ));
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
                            b.brew_time = t;
                            b.phase = BarrelPhase::Ferment(0.0);
                            b.step = 0;
                            crate::audio::play_sfx(&mut commands, &bank, "pop", 0.7);
                            state.notify(format!(
                                "САХАР ДОБАВЛЕН. БРОДИТ ~{:.0} СЕК. ПРОБЕЛ — УСКОРИТЬ",
                                t
                            ));
                        } else {
                            state.notify("НУЖЕН САХАР С ИЗЮМОМ — ПОЛКА (E)".into());
                        }
                    }
                },
                BarrelPhase::Ferment(p) => {
                    state.notify(format!(
                        "БРОДИТ {}%. ПРОБЕЛ — ПОДНАЖАТЬ (+{}%)",
                        p as i32,
                        crate::brewing::QTE_BONUS as i32
                    ));
                }
                BarrelPhase::Ready(p) => {
                    if b.portions == 0 {
                        state.notify(format!(
                            "ПУСТО! СВАРИТЕ НОВЫЙ. {:.0} СЕК ДО ПЕРЕБРОДА",
                            crate::brewing::seconds_to_next(b)
                        ));
                    } else {
                        state.notify(format!(
                            "ГОТОВА {}%! ДЕРЖИТЕ E ({} ПОРЦИЙ)",
                            p as i32, b.portions
                        ));
                    }
                }
                BarrelPhase::Overripe(p) => {
                    // ТЗ №9: переброд можно спасти ведром ледяной воды.
                    if inv.has_bucket() {
                        if cool.active && cool.barrel == Some((tx, ty)) {
                            cool.reset();
                        } else {
                            cool.active = true;
                            cool.pos = if rng.random_bool(0.5) { 0.0 } else { 1.0 };
                            cool.dir = if cool.pos == 0.0 { 1.0 } else { -1.0 };
                            cool.barrel = Some((tx, ty));
                            cool.lock = 0.25;
                            cool.roll_zone(rng);
                            crate::audio::play_sfx(&mut commands, &bank, "water", 0.6);
                            state.notify(format!(
                                "ПЕРЕБРОД {}% — ПРОБЕЛ В СИНЮЮ ЗОНУ (ВЕДРО ЛЬЁТСЯ)",
                                p as i32
                            ));
                        }
                    } else {
                        state.notify(format!(
                            "ПЕРЕБРОД {}%! НУЖНО ВЕДРО ЛЕДЯНОЙ ВОДЫ — КОЛОНКА (E)",
                            p as i32
                        ));
                    }
                }
                BarrelPhase::Ruined => {
                    if b.ran_out {
                        state.notify("КВАС КОНЧИЛСЯ. ДЕРЖИТЕ E — ПОМЫТЬ БОЧКУ".into());
                    } else {
                        state.notify("ИСПОРЧЕНА. ДЕРЖИТЕ E — ПОМЫТЬ".into());
                    }
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
                // ТЗ №6: обслуживается только ПЕРВЫЙ в очереди (spot 0).
                if c.state != CustState::Queued || c.spot != 0 {
                    continue;
                }
                let d = c.pos.distance(target);
                // ТЗ №4: широкий триггер выдачи — клиент перед прилавком,
                // игрок за ним (геометрия даёт ~1.8–2.0, берём с запасом).
                if d < 2.4 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((e, d));
                }
            }
            let Some((entity, _)) = best else {
                state.notify("ЗДЕСЬ НИКОГО НЕТ".into());
                return;
            };
            let Ok((_, mut c)) = customers.get_mut(entity) else { return };
            // ТЗ №7 (бандиты): у кассы бандит требует дань — E платит 30 К.
            if c.kind == NpcKind::Hooligan {
                if state.money >= 30 {
                    state.money -= 30;
                    c.served = true;
                    crate::audio::play_sfx(&mut commands, &bank, "coin", 0.8);
                    state.notify("ДАНЬ ЗАПЛАЧЕНА: -30 К.".into());
                } else {
                    state.notify("НЕТ 30 К. НА ДАНЬ!".into());
                    state.face_set(2, 1.0);
                }
                return;
            }
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
                    // ТЗ №4/№5: любая посуда после гостя уходит в пул мойки
                    // (грязные стаканы, банки и бутылки ждут губки).
                    inv.add_dirty(order);
                    state.money += pay;
                    state.day_earned += pay;
                    crate::audio::play_sfx(&mut commands, &bank, "coin", 0.7);
                    let line = crate::dialogues::thanks_line(kind, overripe, rng);
                    c.say(line);
                    c.state = CustState::Leaving;
                    c.served = true;
                    c.path = std::collections::VecDeque::from([crate::customer::DOOR]);
                    state.notify(format!("{}: +{} К.", kind.name(), pay));
                    state.face_set(1, 2.5); // улыбка: покупатель доволен
                }
                None => {
                    state.notify(format!("НЕТ ПОЛНОГО {} В РУКАХ", order.name()));
                    state.face_set(2, 1.0); // хмурится: нет нужного
                }
            }
        }
        // Тайл-пол/стена. ТЗ №5: мусорка — спрайт (не тайл-стена): срабатывает
        // по близости. Тайловые станции в приоритете — стоя у мойки, E по-прежнему
        // моет посуду, а не выбрасывает.
        _ => {
            let Some(p) = props
                .iter()
                .find(|p| (Vec2::new(cam.x, cam.y) - p.pos).length() < crate::sprites::PROP_REACH)
            else {
                return;
            };
            if p.kind == crate::sprites::PropKind::TrashBin {
                // Выбросить первый занятый слот (или выбранный, если он занят).
                let slot = if inv.hands[inv.selected].is_some() {
                    Some(inv.selected)
                } else {
                    inv.hands.iter().position(|h| h.is_some())
                };
                let dumped = slot.and_then(|s| inv.hands[s].take());
                match dumped {
                    Some(item) => {
                        crate::audio::play_sfx(&mut commands, &bank, "fooff", 0.7);
                        // Тара многоразовая: выброшенная посуда (пустая или
                        // полная) возвращается на склад чистой. Иначе, выкинув
                        // всю посуду, игрок остался бы без тары — софтлок.
                        match item {
                            HandItem::Empty(v, n) | HandItem::Full(v, n, _) => {
                                inv.return_vessels(v, n);
                                state.notify(format!(
                                    "ВЫБРОШЕНО: {} — ТАРА ВЕРНУЛАСЬ НА СКЛАД (+{})",
                                    v.name(),
                                    n
                                ));
                            }
                            other => state.notify(format!("ВЫБРОШЕНО: {}", other.name())),
                        }
                    }
                    None => state.notify("РУКИ ПУСТЫ — ЧЕГО ВЫБРАСЫВАТЬ?".into()),
                }
            }
        }
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
    mut wash_snd: Local<f32>,
) {
    let holding = keys.pressed(KeyCode::KeyE) || mouse.pressed(MouseButton::Left);
    if !matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        return;
    }
    state.wash_bar = 0.0; // ТЗ №8: полоска промывки (заполнится, если моем)
    let (tx, ty) = front_tile(&cam);
    if map.get(tx, ty) != Tile::Barrel {
        fill.progress = 0.0;
        return;
    }
    let Some(b) = barrels.mutable_at(tx, ty) else { return };

    // Промывка испорченной (ТЗ №1: 2 секунды удержания E).
    if b.phase == BarrelPhase::Ruined {
        if holding {
            if fill.target != (tx, ty) {
                fill.target = (tx, ty);
                fill.progress = 0.0;
                crate::audio::play_sfx(&mut commands, &bank, "water", 0.6);
            }
            fill.progress += time.delta_secs() / 2.0;
            // ТЗ №8: полоска прогресса + звук воды при промывке.
            state.wash_bar = fill.progress.clamp(0.0, 1.0);
            *wash_snd += time.delta_secs();
            if *wash_snd > 0.55 {
                *wash_snd = 0.0;
                crate::audio::play_sfx(&mut commands, &bank, "water", 0.45);
            }
            if fill.progress >= 1.0 {
                let was_ran_out = b.ran_out;
                b.phase = BarrelPhase::Empty;
                b.step = 0;
                b.portions = 0;
                b.ran_out = false;
                fill.progress = 0.0;
                state.wash_bar = 0.0;
                if was_ran_out {
                    state.notify("БОЧКА ЧИСТА. ЗАСЫПЬТЕ СУХАРИ (E)".into());
                } else {
                    state.notify("БОЧКА ВЫМЫТА. СВАРИТЕ НОВЫЙ КВАС".into());
                }
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
            if let BarrelPhase::Ferment(p) = b.phase {
                state.notify(format!(
                    "КВАС ЕЩЁ БРОДИТ, {}%. ПОДОЖДИТЕ",
                    p as i32
                ));
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
            // ТЗ №1: кваса больше нет — сразу помечаем бочку пустой
            // (иначе она вечно висит «ГОТОВОЙ», но налить нечего).
            b.phase = BarrelPhase::Ruined;
            b.ran_out = true;
            state.notify("КВАС КОНЧИЛСЯ! ПОМОЙТЕ БОЧКУ (ДЕРЖИТЕ E)".into());
        }
    }
}

/// Мини-игра ускорения брожения: подошёл к бочке — снизу появляется полоска,
/// по ней бегает бегунок, зона подсвечена. ПРОБЕЛ в зоне = +`QTE_BONUS`%.
/// Если не подходить — брожение идёт обычной скоростью (авто-режим).
pub fn brew_qte(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cam: Res<GameCam>,
    map: Res<crate::map::Map>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut qte: ResMut<crate::brewing::BrewQte>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    let dt = time.delta_secs();
    qte.flash = if qte.flash > 0.0 {
        (qte.flash - dt).max(0.0)
    } else {
        (qte.flash + dt).min(0.0)
    };
    qte.lock = (qte.lock - dt).max(0.0);
    // ТЗ: в обучении полоска бегунка тоже обязана работать.
    if !matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        qte.reset();
        return;
    }
    // ТЗ: полоска живёт только на шаге QTE (5/8); после перехода шага её
    // быть не должно — иначе она висит под плашкой следующего шага.
    if state.phase == Phase::Tutorial && state.tut_step != 4 {
        qte.reset();
        return;
    }

    let (tx, ty) = front_tile(&cam);
    let at_barrel = map.get(tx, ty) == Tile::Barrel;
    let boostable = at_barrel
        && barrels
            .map
            .get(&(tx, ty))
            .map(|b| b.boostable())
            .unwrap_or(false);
    if !boostable {
        // отошёл от бочки или она сменила фазу — полоска прячется
        qte.reset();
        return;
    }

    let mut rng = &mut game_rng.0;
    // ТЗ: на шаге обучения полоска видна сразу у бочки — без первого
    // слепого нажатия в пустоту.
    if state.phase == Phase::Tutorial && state.tut_step == 4 && !qte.active {
        qte.active = true;
        qte.barrel = Some((tx, ty));
        qte.pos = 0.0;
        qte.dir = 1.0;
        qte.roll_zone(&mut rng);
        qte.lock = 0.25;
    }
    let pressed = keys.just_pressed(KeyCode::Space);

    if !qte.active {
        if pressed {
            qte.active = true;
            qte.barrel = Some((tx, ty));
            qte.pos = if rng.random_bool(0.5) { 0.0 } else { 1.0 };
            qte.dir = if qte.pos == 0.0 { 1.0 } else { -1.0 };
            qte.roll_zone(&mut rng);
            // защита: это же нажатие не должно считаться ударом
            qte.lock = 0.25;
            crate::audio::play_sfx(&mut commands, &bank, "pop", 0.5);
        }
        return;
    }

    // бегунок бежит по полоске (медленно: игрок должен успевать реагировать)
    let speed = 0.5;
    let prev_pos = qte.pos;
    qte.pos += qte.dir * speed * dt;
    if qte.pos >= 1.0 {
        qte.pos = 1.0;
        qte.dir = -1.0;
    } else if qte.pos <= 0.0 {
        qte.pos = 0.0;
        qte.dir = 1.0;
    }

    if pressed && qte.lock <= 0.0 {
        qte.lock = 0.2;
        // попадание, если бегунок БЫЛ в зоне или пересёк её на этом кадре
        // (сегмент хода [prev..pos] пересекает зону с небольшим запасом)
        let margin = 0.015;
        let (a, b) = (prev_pos.min(qte.pos), prev_pos.max(qte.pos));
        let hit = b >= qte.zone0 - margin && a <= qte.zone1 + margin;
        if hit {
            if let Some(b) = barrels.mutable_at(tx, ty) {
                b.boost();
            }
            qte.flash = 0.35;
            qte.roll_zone(&mut rng);
            crate::audio::play_sfx(&mut commands, &bank, "coin", 0.55);
            if let Some(b) = barrels.map.get(&(tx, ty)) {
                let pct = match b.phase {
                    BarrelPhase::Ferment(p) => p,
                    BarrelPhase::Ready(p) => p,
                    _ => 0.0,
                };
                state.notify(format!(
                    "+{}% — БРОДИТ {}%",
                    crate::brewing::QTE_BONUS as i32,
                    pct as i32
                ));
                state.face_set(1, 1.0); // подмигивает: попал
            }
        } else {
            qte.flash = -0.25;
            crate::audio::play_sfx(&mut commands, &bank, "water", 0.4);
            state.notify("МИМО! ПОДОЖДИТЕ БЕГУНКА В ЗОНЕ".into());
            state.face_set(2, 1.5); // морщится: мимо
        }
    }
}

/// Мини-игра охлаждения перебродившей бочки (ТЗ №9): ПРОБЕЛ в синей зоне
/// льёт ледяную воду — переброд −25%, ведро опустевает.
pub fn cool_qte(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cam: Res<GameCam>,
    map: Res<crate::map::Map>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<Barrels>,
    mut state: ResMut<GameState>,
    mut cool: ResMut<CoolQte>,
    mut commands: Commands,
    bank: Res<crate::audio::SoundBank>,
) {
    let dt = time.delta_secs();
    cool.flash = if cool.flash > 0.0 {
        (cool.flash - dt).max(0.0)
    } else {
        (cool.flash + dt).min(0.0)
    };
    cool.lock = (cool.lock - dt).max(0.0);
    if !cool.active {
        return;
    }
    if state.phase != Phase::Playing {
        cool.reset();
        return;
    }
    let (tx, ty) = front_tile(&cam);
    let still = cool.barrel == Some((tx, ty))
        && map.get(tx, ty) == Tile::Barrel
        && inv.has_bucket()
        && barrels
            .map
            .get(&(tx, ty))
            .map(|b| matches!(b.phase, BarrelPhase::Overripe(_)))
            .unwrap_or(false);
    if !still {
        cool.reset();
        return;
    }

    let prev = cool.pos;
    cool.pos += cool.dir * 0.5 * dt;
    if cool.pos >= 1.0 {
        cool.pos = 1.0;
        cool.dir = -1.0;
    } else if cool.pos <= 0.0 {
        cool.pos = 0.0;
        cool.dir = 1.0;
    }

    if keys.just_pressed(KeyCode::Space) && cool.lock <= 0.0 {
        cool.lock = 0.2;
        let margin = 0.015;
        let (a, b) = (prev.min(cool.pos), prev.max(cool.pos));
        let hit = b >= cool.zone0 - margin && a <= cool.zone1 + margin;
        if hit {
            if let Some(b) = barrels.mutable_at(tx, ty) {
                if let BarrelPhase::Overripe(p) = b.phase {
                    let np = p - 25.0;
                    b.phase = if np < crate::brewing::PCT_OVERRIPE {
                        BarrelPhase::Ready(np)
                    } else {
                        BarrelPhase::Overripe(np)
                    };
                }
            }
            inv.drain_bucket();
            cool.reset();
            cool.flash = 0.5;
            crate::audio::play_sfx(&mut commands, &bank, "ding", 0.7);
            state.notify("БОЧКА ОХЛАЖДЕНА (−25%). ВЕДРО ПУСТОЕ — КОЛОНКА (E)".into());
            state.face_set(1, 1.2);
        } else {
            cool.flash = -0.25;
            crate::audio::play_sfx(&mut commands, &bank, "water", 0.4);
            state.notify("МИМО! ПРОБЕЛ, КОГДА БЕГУНК В СИНЕЙ ЗОНЕ".into());
            state.face_set(2, 1.5);
        }
    }
}
