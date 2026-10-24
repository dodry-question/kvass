//! Сцена бандита в коридоре: возвращаешься со склада — он выбегает
//! из-за угла, мир замирает, музыка останавливается, начинается разговор.
//!
//! Три варианта: отдать 15 К., использовать предмет из рук/со склада,
//! соврать. Ложь — честные 50/50: поверит и уйдёт или ударит, после чего
//! половина экрана мутнеет и расплывается («подбитый глаз»).

use std::collections::VecDeque;

use bevy::prelude::*;
use rand::prelude::*;

use crate::customer::{CustState, Customer};
use crate::day_cycle::{GameRng, GameState, Phase};
use crate::inventory::{HandItem, Inventory};
use crate::raycaster::GameCam;
use crate::sprites::{NpcKind, Vessel};

/// Сколько бандит требует. В экономике игры «К.» — это тысячи,
/// поэтому 15 К. = те самые «15к» из задания.
pub const DEMAND: i32 = 15;
/// Сцена доступна с первого дня: она привязана к походу на склад,
/// а не к календарю. Раз в день.
pub const MIN_DAY: u32 = 1;
/// Сколько секунд держится «подбитый глаз».
pub const BLACK_EYE_SECS: f32 = 60.0;
/// Обычный шанс сцены при каждом возврате со склада. `band` его обходит.
pub const AMBUSH_CHANCE: f64 = 0.30;

pub const PHRASES: [&str; 4] = [
    "Стой, хозяин! Слышал, твой квас — лучший в городе. Делиться будем?",
    "Кошелёк или квас? Шутка. Давай кошелёк.",
    "А ну стоять! Ты со склада? Значит, выручка с собой.",
    "Тихо, буржуй. Отдай 15 К. — и разойдёмся по-хорошему.",
];

pub const LIE_LINE: &str = "Да я тут вообще уборщик! Касса пустая, и я не отсюда.";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AmbushStage {
    #[default]
    Inactive,
    /// Бандит выбегает из-за угла (короткая анимация, мир уже стоит).
    Rushing,
    /// Ждём выбора игрока.
    Choice,
    /// Показываем исход, ждём подтверждения.
    Result,
}

#[derive(Resource)]
pub struct BanditAmbush {
    pub stage: AmbushStage,
    pub phrase: usize,
    pub choice: usize,
    pub timer: f32,
    pub result: String,
    pub result_good: bool,
    pub black_eye_hit: bool,
    pub notice: String,
    pub bandit: Option<Entity>,
    pub day: u32,
    pub prev_x: f32,
    pub visited_warehouse: bool,
}

impl Default for BanditAmbush {
    fn default() -> Self {
        Self {
            stage: AmbushStage::Inactive,
            phrase: 0,
            choice: 0,
            timer: 0.0,
            result: String::new(),
            result_good: false,
            black_eye_hit: false,
            notice: String::new(),
            bandit: None,
            day: 0,
            prev_x: -1.0,
            visited_warehouse: false,
        }
    }
}

impl BanditAmbush {
    /// Сцена держит игровое время/музыку на паузе.
    pub fn freeze(&self) -> bool {
        self.stage != AmbushStage::Inactive
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn reset_day(&mut self, day: u32) {
        *self = Self::default();
        self.day = day;
        self.prev_x = -1.0;
    }

    pub fn phrase(&self) -> &'static str {
        PHRASES[self.phrase.min(PHRASES.len() - 1)]
    }
}

#[derive(Component)]
pub struct AmbushBandit;

/// Общее управление бандитами (отладочные `band` / `unband` и лимиты дня).
///
/// `disabled` выключает и старую очередь хулиганов, и сцену в коридоре.
/// `force_next` — `band`: следующий переход склад → касса гарантированно
/// запускает сцену, даже если лимит дня уже исчерпан.
#[derive(Resource)]
pub struct BanditControl {
    pub disabled: bool,
    pub force_next: bool,
    pub day: u32,
    /// Сколько раз бандиты уже появлялись в этот день (и очередь, и коридор).
    pub spawned_today: u32,
}

impl Default for BanditControl {
    fn default() -> Self {
        Self {
            disabled: false,
            force_next: false,
            day: 0,
            spawned_today: 0,
        }
    }
}

impl BanditControl {
    /// Балансный лимит появления бандитов за день.
    pub fn max_for_day(day: u32) -> u32 {
        match day {
            1 => 1,
            2 => 3,
            3 => 4,
            _ => 4,
        }
    }

    pub fn reset_day(&mut self, day: u32) {
        self.day = day;
        self.spawned_today = 0;
    }

    pub fn can_spawn(&self, day: u32) -> bool {
        !self.disabled && self.spawned_today < Self::max_for_day(day)
    }

    pub fn note_spawn(&mut self) {
        self.spawned_today = self.spawned_today.saturating_add(1);
    }
}

pub fn item_label(inv: &Inventory) -> Option<String> {
    inv.first_item_name()
}

fn item_result(item: &HandItem) -> String {
    match item {
        HandItem::Bucket => {
            "ВЫ: Окатили бандита водой!\n\nБАНДИТ: Тьфу! Холодная... Я ещё вернусь!".into()
        }
        HandItem::Croutons(_) => {
            "ВЫ: Закидали бандита сухарями!\n\nБАНДИТ: Да что ты кидаешься? Ладно, живи.".into()
        }
        HandItem::Sugar(_) => {
            "ВЫ: Швырнули в бандита сахар с изюмом!\n\nБАНДИТ: Тьфу, сладко! Уйди с глаз.".into()
        }
        HandItem::Empty(v, _) => format!(
            "ВЫ: Замахнулись пустым {}!\n\nБАНДИТ: Псих! Ладно, проходи.\n\n({} УПАЛ НА ПОЛ И СТАЛ ГРЯЗНЫМ — ПОМОЙТЕ)",
            v.name(),
            v.name()
        ),
        HandItem::Full(v, _, _) => format!(
            "ВЫ: Плеснули квасом из {}!\n\nБАНДИТ: Фу, липко! Отстань.\n\n({} УПАЛ НА ПОЛ И СТАЛ ГРЯЗНЫМ — ПОМОЙТЕ)",
            v.name(),
            v.name()
        ),
    }
}

/// Детект перехода склад → зал, короткий выбег и фиксация позиции бандита.
pub fn ambush_system(
    time: Res<Time>,
    mut state: ResMut<GameState>,
    cam: Res<GameCam>,
    mut ambush: ResMut<BanditAmbush>,
    mut rng: ResMut<GameRng>,
    mut commands: Commands,
    mut bandits: Query<&mut Customer, With<AmbushBandit>>,
    overbrew: Res<crate::brewing::FirstOverbrew>,
    debug: Res<crate::debug::DebugConsole>,
    mut control: ResMut<BanditControl>,
    rush: Res<crate::rush::Rush>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    if ambush.day != state.day {
        ambush.reset_day(state.day);
    }
    if control.day != state.day {
        control.reset_day(state.day);
    }
    // `stop` в отладке замораживает и выбег бандита.
    if debug.time_stopped && ambush.stage == AmbushStage::Rushing {
        return;
    }

    if ambush.stage == AmbushStage::Inactive {
        // Пока время стоит/идёт первый переброд, просто запоминаем позицию,
        // чтобы после возврата не сработала «ложная» старая граница.
        if debug.time_stopped || overbrew.freeze() {
            ambush.prev_x = cam.x;
            return;
        }
        // «Час пик»: в такой толпе бандиты не суются.
        if rush.blocks_customers() {
            ambush.prev_x = cam.x;
            return;
        }
        // `band` обходит и выключение (`unband`), и дневной лимит.
        let force = control.force_next;
        if control.disabled && !force {
            ambush.prev_x = cam.x;
            return;
        }
        if state.day < MIN_DAY || state.day_intro > 0.0 {
            ambush.prev_x = cam.x;
            return;
        }
        if cam.x >= 14.0 {
            ambush.visited_warehouse = true;
        }
        if !force && !control.can_spawn(state.day) {
            ambush.prev_x = cam.x;
            return;
        }
        let returning = cam.x < ambush.prev_x && ambush.prev_x >= 9.25 && cam.x <= 9.25;
        let in_corridor = cam.y > 5.2 && cam.y < 7.2;
        let prev = ambush.prev_x;
        ambush.prev_x = cam.x;
        if returning
            && in_corridor
            && prev >= 9.25
            && ambush.visited_warehouse
            && (force || rng.0.random_bool(AMBUSH_CHANCE))
        {
            start_ambush(&cam, &mut state, &mut ambush, &mut rng, &mut commands);
            control.force_next = false;
            control.note_spawn();
        }
        return;
    }

    if ambush.stage == AmbushStage::Rushing {
        ambush.timer = (ambush.timer - time.delta_secs()).max(0.0);
        if let Some(e) = ambush.bandit {
            if let Ok(mut c) = bandits.get_mut(e) {
                let target = Vec2::new(
                    cam.x + cam.angle.cos() * 0.78,
                    cam.y + cam.angle.sin() * 0.78,
                );
                let d = target - c.pos;
                let len = d.length();
                if len > 0.02 {
                    let step = (len * 0.35).min(4.0 * time.delta_secs());
                    c.pos += d / len * step;
                }
            }
        }
        if ambush.timer <= 0.0 {
            ambush.stage = AmbushStage::Choice;
            ambush.choice = 0;
        }
    }
}

fn start_ambush(
    cam: &GameCam,
    state: &mut GameState,
    ambush: &mut BanditAmbush,
    rng: &mut GameRng,
    commands: &mut Commands,
) {
    let phrase = rng.0.random_range(0..PHRASES.len());
    let forward = Vec2::new(cam.angle.cos(), cam.angle.sin());
    // Ставим бандита прямо перед игроком и не даём коридору шириной в
    // тайл задвинуть его в стену: лицо должно быть видно в диалоге.
    let pos = Vec2::new(cam.x, cam.y) + forward * 1.15;
    let pos = Vec2::new(pos.x, pos.y.clamp(6.15, 6.85));
    let entity = commands
        .spawn((
            Customer {
                kind: NpcKind::Hooligan,
                order: Vessel::Glass,
                state: CustState::Custom,
                pos,
                path: VecDeque::new(),
                patience: 9999.0,
                max_patience: 9999.0,
                queue_patience: 9999.0,
                bubble: None,
                bubble_t: 0.0,
                spot: 0,
                served: false,
                step_t: 0.0,
            },
            AmbushBandit,
        ))
        .id();
    ambush.stage = AmbushStage::Rushing;
    ambush.timer = 0.34;
    ambush.phrase = phrase;
    ambush.choice = 0;
    ambush.result.clear();
    ambush.notice.clear();
    ambush.result_good = false;
    ambush.black_eye_hit = false;
    ambush.bandit = Some(entity);
    ambush.visited_warehouse = false;
    state.face_set(2, 3.0);
}

/// Выбор варианта и подтверждение исхода.
pub fn ambush_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut ambush: ResMut<BanditAmbush>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut rng: ResMut<GameRng>,
    mut commands: Commands,
    bandits: Query<Entity, With<AmbushBandit>>,
) {
    match ambush.stage {
        AmbushStage::Choice => {
            let mut chosen = None;
            if keys.just_pressed(KeyCode::Digit1) {
                chosen = Some(0);
            } else if keys.just_pressed(KeyCode::Digit2) {
                chosen = Some(1);
            } else if keys.just_pressed(KeyCode::Digit3) {
                chosen = Some(2);
            }
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                ambush.choice = (ambush.choice + 2) % 3;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                ambush.choice = (ambush.choice + 1) % 3;
            }
            if keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE)
            {
                chosen = Some(ambush.choice);
            }
            if let Some(i) = chosen {
                choose(i, &mut ambush, &mut state, &mut inv, &mut rng);
                keys.clear();
            }
        }
        AmbushStage::Result => {
            if keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE)
                || keys.just_pressed(KeyCode::Escape)
            {
                for e in bandits.iter() {
                    commands.entity(e).despawn();
                }
                ambush.stage = AmbushStage::Inactive;
                ambush.bandit = None;
                ambush.notice.clear();
                ambush.black_eye_hit = false;
                keys.clear();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandit_daily_limits_match_balance() {
        assert_eq!(BanditControl::max_for_day(1), 1);
        assert_eq!(BanditControl::max_for_day(2), 3);
        assert_eq!(BanditControl::max_for_day(3), 4);
    }

    #[test]
    fn vessel_thrown_at_bandit_becomes_dirty_not_lost() {
        // Единственная бутылка, брошенная в бандита, не должна пропадать
        // из пула посуды — иначе день превращается в софтлок.
        let mut ambush = BanditAmbush::default();
        let mut state = GameState::default();
        let mut rng = GameRng(StdRng::seed_from_u64(1));

        let mut inv = Inventory::default();
        inv.put(HandItem::Empty(Vessel::Bottle, 1));
        choose(
            1,
            &mut ambush,
            &mut state,
            &mut inv,
            &mut rng,
        );
        assert_eq!(ambush.stage, AmbushStage::Result);
        assert!(inv.hands.iter().all(|h| h.is_none()));
        assert_eq!(inv.dirty_bottles, 1, "бутылка должна уйти в грязную");
        assert_eq!(
            inv.dirty_bottles + inv.stock_bottle,
            1,
            "пул бутылок не меняется"
        );

        // Полная банка — та же история: не исчезла, а стала грязной.
        let mut inv2 = Inventory::default();
        inv2.put(HandItem::Full(Vessel::Jar, 1, false));
        choose(1, &mut ambush, &mut state, &mut inv2, &mut rng);
        assert_eq!(inv2.dirty_jars, 1);
        assert_eq!(inv2.dirty_jars + inv2.stock_jar, 1);
    }

    #[test]
    fn unband_disables_and_day_resets_counter() {
        let mut c = BanditControl::default();
        c.reset_day(2);
        assert!(c.can_spawn(2));
        c.note_spawn();
        c.note_spawn();
        c.note_spawn();
        assert!(!c.can_spawn(2));
        c.disabled = true;
        c.reset_day(3);
        assert!(!c.can_spawn(3));
        c.disabled = false;
        assert!(c.can_spawn(3));
    }

    #[test]
    fn band_forces_ambush_on_warehouse_return() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameState {
            phase: Phase::Playing,
            day: 1,
            day_intro: 0.0,
            ..Default::default()
        });
        app.insert_resource(GameCam {
            x: 14.0,
            y: 6.0,
            angle: -std::f32::consts::FRAC_PI_2,
        });
        app.insert_resource(BanditAmbush::default());
        app.insert_resource(BanditControl {
            force_next: true,
            ..Default::default()
        });
        app.insert_resource(GameRng(StdRng::seed_from_u64(7)));
        app.insert_resource(crate::rush::Rush::default());
        app.insert_resource(crate::brewing::FirstOverbrew::default());
        app.insert_resource(crate::debug::DebugConsole::default());
        app.add_systems(Update, ambush_system);

        // Сначала побывали на складе.
        app.update();
        // Затем вернулись в коридор на запад — force обязан сработать.
        app.world_mut().resource_mut::<GameCam>().x = 9.1;
        app.update();

        let ambush = app.world().resource::<BanditAmbush>();
        assert_eq!(ambush.stage, AmbushStage::Rushing);
        assert!(ambush.bandit.is_some());
        assert!(!app.world().resource::<BanditControl>().force_next);
    }
}

fn choose(
    idx: usize,
    ambush: &mut BanditAmbush,
    state: &mut GameState,
    inv: &mut Inventory,
    rng: &mut GameRng,
) {
    ambush.notice.clear();
    match idx {
        0 => {
            if state.money < DEMAND {
                ambush.notice = format!("НЕ ХВАТАЕТ {} К. (У ВАС {} К.)", DEMAND, state.money);
                return;
            }
            state.money -= DEMAND;
            ambush.result_good = true;
            ambush.result = "БАНДИТ: Вот это другой разговор. Живи пока.".into();
            state.face_set(1, 2.0);
            ambush.stage = AmbushStage::Result;
        }
        1 => {
            let Some(item) = inv.take_any_item() else {
                ambush.notice = "В РУКАХ И НА СКЛАДЕ НЕТ ПОДХОДЯЩЕГО ПРЕДМЕТА".into();
                return;
            };
            // ТЗ №4: посуда многоразовая — брошенная в бандита не исчезает
            // из пула (иначе единственная бутылка = софтлок), а уходит
            // в грязную: мойка вернёт её на склад. Ведро и сырьё расходуются.
            if let HandItem::Empty(v, _) | HandItem::Full(v, _, _) = &item {
                inv.add_dirty(*v);
            }
            ambush.result_good = true;
            ambush.result = item_result(&item);
            state.face_set(1, 2.0);
            ambush.stage = AmbushStage::Result;
        }
        _ => {
            let believe = rng.0.random_bool(0.5);
            if believe {
                ambush.result_good = true;
                ambush.result = format!(
                    "ВЫ: «{}»\n\nБАНДИТ: Уборщик? Ладно, работай. Но если соврал — найду.",
                    LIE_LINE
                );
                state.face_set(1, 2.0);
            } else {
                ambush.result_good = false;
                ambush.black_eye_hit = true;
                state.black_eye = BLACK_EYE_SECS;
                state.face_set(2, 5.0);
                ambush.result = format!(
                    "ВЫ: «{}»\n\nБАНДИТ: Заливаешь, уборщик?! Получай!",
                    LIE_LINE
                );
            }
            ambush.stage = AmbushStage::Result;
        }
    }
}
