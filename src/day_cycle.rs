//! Игровой день: фазы, норма денег, закупка сырья, спавн покупателей.

use bevy::prelude::*;
use rand::prelude::*;

use crate::customer::{spawn_customer, Customer, QUEUE_SPOTS};
use crate::sprites::NpcKind;

#[derive(Resource)]
pub struct GameRng(pub StdRng);

/// Грязь: непромытые бочки/переброды — повод для штрафа.
#[derive(Resource, Default)]
pub struct Dirty(pub u32);

/// Точка появления игрока в начале каждого дня (и после обучения).
pub const SPAWN_X: f32 = 5.5;
pub const SPAWN_Y: f32 = 4.8;
pub const SPAWN_ANGLE: f32 = -std::f32::consts::FRAC_PI_2;

/// Сколько секунд идёт заставка «ДЕНЬ N» в начале дня: чёрный экран
/// с надписью, затем медленно проявляется лавка.
pub const DAY_INTRO_SECS: f32 = 3.2;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// Главное меню.
    Menu,
    /// Обучение: без времени, один гость, подсказки с маркером.
    Tutorial,
    /// Игровой день.
    Playing,
    /// Итоги дня.
    DayEnd,
    /// Финальная заставка.
    Final,
    /// Пауза (ESC): продолжить / в меню / выход.
    Paused,
}

/// Какой экран меню показывать (обои с кнопками на фоне).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum MenuView {
    /// Главное меню: кнопки ВХОД/ПРОДОЛЖИТЬ/ОБУЧЕНИЕ/НАСТРОЙКИ/ВЫХОД.
    #[default]
    Main,
    /// Экран настроек (громкость).
    Settings,
    /// Окно с настоятельной рекомендацией пройти обучение (перед ВХОДом).
    Advice,
}

#[derive(Resource)]
pub struct GameState {
    pub day: u32,
    pub money: i32,
    pub day_earned: i32,
    pub norm: i32,
    pub time_left: f32,
    pub phase: Phase,
    pub fine: i32,
    pub fine_reason: String,
    pub spawn_t: f32,
    pub spawn_interval: f32,
    pub msg: String,
    pub msg_t: f32,
    pub day_passed: bool,
    /// Шаг обучения (0..=9): 8 — помыть стакан, 9 — выбросить хлам.
    pub tut_step: u8,
    /// Обучение пройдено/пропущено.
    pub tut_done: bool,
    /// Затемнение-заставка на старте дня: >0 — экран ещё закрыт тьмой.
    /// Убывает в `day_system`; пока идёт, день не тикает и ввод заморожен.
    pub day_intro: f32,
    /// Откуда открыта пауза (PAUSE_FROM_*).
    pub pause_prev: u8,
    /// Выбранный пункт меню паузы.
    pub pause_sel: u8,
    /// Лик героя (Doom-style): 0 спокойно, 1 рад (купил), 2 мимо/боль.
    pub face_mood: u8,
    /// Секунд, сколько держится текущее выражение лица.
    pub face_t: f32,
    /// Прогресс промывки бочки удержанием E (0..1) — полоска в HUD (ТЗ №8).
    pub wash_bar: f32,
    /// Тревога (ТЗ №5): >0 — бандиты на подходе, экран пульсирует красным.
    /// Убывает в `day_system`, выставляется при спавне хулигана.
    pub alarm_t: f32,
    /// Экран меню (главное / настройки / рекомендация обучения).
    pub menu_view: MenuView,
    /// Выбранный пункт меню — индекс среди ВИДИМЫХ кнопок.
    pub menu_sel: u8,
    /// Громкость в настройках: 0..=10 (0 — тишина, 10 — 100%).
    pub menu_vol: u8,
    /// Есть прогресс (игра запускалась) — активен слот «ПРОДОЛЖИТЬ».
    pub has_progress: bool,
    /// Обучение запущено из меню — пропуск (T) запрещён.
    pub tut_locked: bool,
    /// Куда вести после обучения: в игровой цикл (`true`) или в меню.
    pub tut_after_game: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            day: 1,
            money: 30,
            day_earned: 0,
            norm: 100,
            time_left: 240.0,
            phase: Phase::Menu,
            fine: 0,
            fine_reason: String::new(),
            spawn_t: 10.0,
            spawn_interval: 22.0,
            msg: String::new(),
            msg_t: 0.0,
            day_passed: false,
            tut_step: 0,
            tut_done: false,
            day_intro: 0.0,
            pause_prev: PAUSE_FROM_PLAYING,
            pause_sel: 0,
            face_mood: 0,
            face_t: 0.0,
            wash_bar: 0.0,
            alarm_t: 0.0,
            menu_view: MenuView::Main,
            menu_sel: 0,
            menu_vol: 10,
            has_progress: false,
            tut_locked: false,
            tut_after_game: false,
        }
    }
}

impl GameState {
    pub fn notify(&mut self, text: String) {
        self.msg = text;
        self.msg_t = 3.0;
    }
    /// Выражение лица героя на `secs` секунд (Doom-style лик в HUD).
    pub fn face_set(&mut self, mood: u8, secs: f32) {
        self.face_mood = mood;
        self.face_t = secs;
    }
}

/// Стартовый пакет дня 1 (он же выдаётся перед обучением и после него).
pub fn give_start_package(state: &mut GameState, inv: &mut crate::inventory::Inventory) {
    state.day = 1;
    state.money = 30;
    state.norm = norm_for(1);
    state.day_earned = 0;
    inv.stock_croutons = 2;
    inv.stock_sugar = 2;
    // ТЗ №8/№4: посуда многоразовая — стартовый пул, не покупается.
    // ТЗ №5: запас урезан (4/2/1) — мыть тару приходится уже в середине дня 1.
    inv.stock_glass = 4;
    inv.stock_jar = 2;
    inv.stock_bottle = 1;
    inv.dirty_glasses = 0;
    inv.dirty_jars = 0;
    inv.dirty_bottles = 0;
    inv.hands = [None; 4];
}

/// Начало игрового дня (ТЗ №3: фазы закупки больше нет — день стартует
/// сразу, сырьё покупают в «окне снабжения» посреди смены).
pub fn start_day(
    state: &mut GameState,
    barrels: &mut crate::brewing::Barrels,
    dirty: &mut Dirty,
    cam: &mut crate::raycaster::GameCam,
) {
    state.phase = Phase::Playing;
    state.time_left = day_len(state.day);
    state.spawn_t = 10.0;
    state.fine = 0;
    state.fine_reason.clear();
    // сброс мира: бочки и грязь (покупателей чистит вызывающий)
    for b in barrels.map.values_mut() {
        *b = crate::brewing::Barrel::new();
    }
    dirty.0 = 0;
    // игрок всегда начинает смену за прилавком, лицом к очереди
    cam.x = SPAWN_X;
    cam.y = SPAWN_Y;
    cam.angle = SPAWN_ANGLE;
    // слот «ПРОДОЛЖИТЬ» в главном меню появляется после первого запуска дня
    state.has_progress = true;
    let d = state.day;
    let n = state.norm;
    // заставка «ДЕНЬ N»: тьма медленно рассеивается, магазин проявляется
    state.day_intro = DAY_INTRO_SECS;
    // Тост не тикает под заставкой (day_system спит), поэтому покажется
    // сразу после проявления магазина и провисит обычные 3 секунды.
    state.notify(format!("ДЕНЬ {d} НАЧАЛСЯ! НОРМА {n} К. ОКНО СНАБЖЕНИЯ (E)"));
}

/// Дespawn всех покупателей (вызывается перед началом нового дня).
pub fn clear_customers(commands: &mut Commands, customers: &Query<Entity, With<Customer>>) {
    for e in customers.iter() {
        commands.entity(e).despawn();
    }
}

pub fn day_len(day: u32) -> f32 {
    210.0 + 30.0 * day as f32
}

pub fn norm_for(day: u32) -> i32 {
    let mut n = 100.0f32;
    for _ in 1..day {
        n *= 1.4;
    }
    n as i32
}

/// Кто может зайти в этот день.
pub fn customer_pool(day: u32) -> Vec<NpcKind> {
    use NpcKind::*;
    let mut v = vec![Grandma, Worker, Mama, Student];
    if day >= 2 {
        v.push(Worker);
        v.push(Mama);
        v.push(Student);
    }
    v
}

/// Фазы: меню, закупка, конец дня.

/// Откуда открыли паузу (для возврата).
pub const PAUSE_FROM_PLAYING: u8 = 0;
pub const PAUSE_FROM_TUTORIAL: u8 = 1;
/// Число пунктов в меню паузы.
pub const PAUSE_ITEMS: u8 = 3;
pub fn phase_system(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut commands: Commands,
    mut barrels: ResMut<crate::brewing::Barrels>,
    customers: Query<Entity, With<Customer>>,
    mut dirty: ResMut<Dirty>,
    mut cam: ResMut<crate::raycaster::GameCam>,
) {
    match state.phase {
        Phase::Menu => {
            // Ввод меню (мышь + клавиатура) целиком в menu::menu_input:
            // ВХОД/ОБУЧЕНИЕ/НАСТРОЙКИ/ВЫХОД и окно рекомендации обучения.
        }
        Phase::DayEnd => {
            let go = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space);
            if go {
                if state.day >= 4 {
                    state.phase = Phase::Final;
                    return;
                }
                // следующий день — сразу в игру (ТЗ №3: закупки нет)
                state.day += 1;
                state.norm = norm_for(state.day);
                state.day_earned = 0;
                // Поток покупателей щадящий: в день 1 один человек за раз,
                // дальше чуть плотнее, но без очереди в три ряда.
                state.spawn_interval = (22.0 - state.day as f32 * 2.5).max(11.0);
                clear_customers(&mut commands, &customers);
                start_day(&mut state, &mut barrels, &mut dirty, &mut cam);
                keys.clear(); // ТЗ №1
            }
        }
        Phase::Final => {
            if keys.just_pressed(KeyCode::Escape) {
                state.phase = Phase::Menu;
                state.menu_view = MenuView::Main;
                state.menu_sel = 0;
                // игра пройдена — «продолжать» больше нечего
                state.has_progress = false;
                keys.clear(); // ТЗ №1
            }
        }
        Phase::Playing => {
            // во время заставки «ДЕНЬ N» пауза не открывается
            if state.day_intro <= 0.0 && keys.just_pressed(KeyCode::Escape) {
                state.pause_prev = PAUSE_FROM_PLAYING;
                state.pause_sel = 0;
                state.phase = Phase::Paused;
                keys.clear(); // ТЗ №1
            }
        }
        Phase::Tutorial => {
            if keys.just_pressed(KeyCode::Escape) {
                state.pause_prev = PAUSE_FROM_TUTORIAL;
                state.pause_sel = 0;
                state.phase = Phase::Paused;
                keys.clear(); // ТЗ №1
            }
        }
        Phase::Paused => {
            let n = PAUSE_ITEMS.max(1);
            if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
                state.pause_sel = (state.pause_sel + n - 1) % n;
            }
            if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
                state.pause_sel = (state.pause_sel + 1) % n;
            }
            let ok = keys.just_pressed(KeyCode::Enter)
                || keys.just_pressed(KeyCode::NumpadEnter)
                || keys.just_pressed(KeyCode::Space)
                || keys.just_pressed(KeyCode::KeyE);
            if keys.just_pressed(KeyCode::Escape) {
                // ESC = продолжить
                ok_pause_resume(&mut state);
                keys.clear(); // ТЗ №1: E/ESC не сработают в игре в этот же кадр
            } else if ok {
                match state.pause_sel.min(n - 1) {
                    0 => {
                        ok_pause_resume(&mut state);
                        keys.clear(); // ТЗ №1
                    }
                    1 => {
                        // в главное меню: сброс мира
                        for b in barrels.map.values_mut() {
                            *b = crate::brewing::Barrel::new();
                        }
                        for e in customers.iter() {
                            commands.entity(e).despawn();
                        }
                        dirty.0 = 0;
                        state.msg_t = 0.0;
                        state.phase = Phase::Menu;
                        state.menu_view = MenuView::Main;
                        state.menu_sel = 0;
                        keys.clear(); // ТЗ №1
                    }
                    _ => {
                        // выход из игры
                        std::process::exit(0);
                    }
                }
            }
        }
    }
}

/// Возврат из паузы к игре/обучению.
fn ok_pause_resume(state: &mut GameState) {
    state.phase = if state.pause_prev == PAUSE_FROM_TUTORIAL {
        Phase::Tutorial
    } else {
        Phase::Playing
    };
}

/// Идёт игровой день: таймер, спавн, конец смены.
pub fn day_system(
    time: Res<Time>,
    mut state: ResMut<GameState>,
    customers: Query<&Customer>,
    mut game_rng: ResMut<GameRng>,
    mut commands: Commands,
    mut modals: ResMut<crate::modals::Modals>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let dt = time.delta_secs();
    // Заставка «ДЕНЬ N»: пока тьма не рассеялась, смена ещё не тикает —
    // ни таймер, ни покупатели, ни тосты. Игрок «приходит в себя».
    if state.day_intro > 0.0 {
        state.day_intro = (state.day_intro - dt).max(0.0);
        return;
    }
    if state.msg_t > 0.0 {
        state.msg_t -= dt;
    }
    if state.face_t > 0.0 {
        state.face_t -= dt;
        if state.face_t <= 0.0 {
            state.face_mood = 0;
        }
    }
    // ТЗ №5: тревога (бандиты) пульсирует ~3 сек и гаснет.
    if state.alarm_t > 0.0 {
        state.alarm_t = (state.alarm_t - dt).max(0.0);
    }
    state.time_left -= dt;
    if state.time_left <= 0.0 {
        state.time_left = 0.0;
        state.day_passed = state.money >= state.norm;
        state.phase = Phase::DayEnd;
        // ТЗ №5: время идёт даже в модалках — смена может кончиться
        // прямо под окном снабжения/мойки: закрываем, чтобы не «зависло».
        modals.modal = crate::modals::Modal::None;
        modals.sponge = crate::modals::Sponge::default();
        return;
    }
    // спавн покупателей
    state.spawn_t -= dt;
    if state.spawn_t <= 0.0 {
        let mut taken = [false; QUEUE_SPOTS.len()];
        for c in customers.iter() {
            if c.spot < taken.len() {
                taken[c.spot] = true;
            }
        }
        let pool = customer_pool(state.day);
        let mut rng = &mut game_rng.0;
        let _ = spawn_customer(&mut commands, &pool, &taken, &mut rng);
        state.spawn_t = state.spawn_interval + rng.random_range(-3.0..3.0);
    }
}
