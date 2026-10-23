//! «КВАСНАЯ ЛАВКА» — Soviet kvass shop Doom-style raycaster.
//! Vyatka_Owls Game Jam. Rust + Bevy.

// Релиз без консольного окна: диагностика в UTF-8 рисовалась бы в legacy-
// кодировке Windows-консоли («ударения» над буквами — артефакт CP852, а не
// шрифта). Для --selftest/--dump-sprites консоль возвращается в main().
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ambush;
mod audio;
mod brewing;
mod canvas;
mod customer;
mod day_cycle;
mod debug;
mod dialogues;
mod enemies;
mod font;
mod inventory;
mod interact;
mod map;
mod menu;
mod modals;
mod palette;
mod player;
mod raycaster;
mod rush;
mod dump;
mod selftest;
mod sprites;
mod story;
mod textures;
mod tutorial;
mod ui;

use bevy::prelude::*;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, WindowMode};
use rand::prelude::*;

use ambush::BanditAmbush;
use canvas::{CV_H, CV_W, Canvas};
use brewing::Barrels;
use customer::Customer;
use debug::DebugConsole;
use day_cycle::{GameState, GameRng, Phase};
use font::Font;
use inventory::Inventory;
use raycaster::{GameCam, Renderer, SCREEN_H, SCREEN_W};
use sprites::SpriteSheet;
use textures::Textures;

#[derive(Resource)]
struct World {
    textures: Textures,
    sprites: SpriteSheet,
    renderer: Renderer,
    font: Font,
    flicker: f32,
}

/// Handles текстур экрана: мир 640×400 и UI-канвас 1280×800 (один ресурс,
/// чтобы render_frame укладывался в лимит параметров Bevy).
#[derive(Resource)]
struct ScreenImages {
    world: Handle<Image>,
    ui: Handle<Image>,
}

/// Каталог assets: рядом с exe, в текущей папке или в исходниках (dev-сборка).
/// Настройка окна: игра ВСЕГДА открывается полноэкранной (borderless на
/// основном мониторе). Оконный режим — только явный флаг `--windowed`
/// (для автотестов/отладки, в т.ч. под Wine/Bottles, где borderless
/// может не открыться). Переменные окружения на режим окна не влияют.
fn window_setup() -> Window {
    let title = format!("КВАСНАЯ ЛАВКА [{BUILD_ID}]");
    if std::env::args().any(|a| a == "--windowed") {
        Window {
            title,
            mode: WindowMode::Windowed,
            resolution: (1280, 800).into(),
            ..default()
        }
    } else {
        Window {
            title,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            ..default()
        }
    }
}

/// Идентификатор сборки — выводится в HUD для проверки версии exe (ТЗ).
pub const BUILD_ID: &str = "BUILD 4: ЭКОНОМИКА, БАНДИТЫ И ЧАС ПИК";

pub fn assets_root() -> std::path::PathBuf {
    let mut bases: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            bases.push(d.to_path_buf());
            // dist/exe → корень проекта (assets лежит рядом с dist/)
            if let Some(p) = d.parent() {
                bases.push(p.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        bases.push(cwd);
    }
    for b in &bases {
        let a = b.join("assets");
        if a.join("sounds").exists() || a.join("fonts").exists() {
            return a;
        }
    }
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
}

/// Квадрат сцены: показывается ровно в область 4:3 по центру окна
/// (чёрные полоски по краям — см. `stage_43`).
#[derive(Component)]
struct ScreenQuad;

/// Геометрия сцены: фиксированная область 4:3, вписанная в окно и
/// центрированная в нём (возвращает x, y, w, h от левого верхнего угла
/// окна). Вокруг — чёрные полоски: на большом/широком экране кадр больше
/// не растягивается, а всегда строго 4:3 (пропорция ложится и на мир, и на
/// UI-канвас 1280×800 одинаково).
pub fn stage_43(win_w: f32, win_h: f32) -> (f32, f32, f32, f32) {
    let h = win_h.min(win_w * 3.0 / 4.0).floor().max(1.0);
    let w = (h * 4.0 / 3.0).floor().max(1.0);
    ((win_w - w) / 2.0, (win_h - h) / 2.0, w, h)
}

/// Оба кадра (мир и UI) — ровно в область 4:3 по центру окна.
fn fit_screen(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut quads: Query<&mut Sprite, With<ScreenQuad>>,
) {
    let Ok(w) = windows.single() else { return };
    let ww = w.resolution.width();
    let wh = w.resolution.height();
    if ww < 16.0 || wh < 16.0 {
        return;
    }
    let (_, _, sw, sh) = stage_43(ww, wh);
    let size = Vec2::new(sw, sh);
    for mut s in &mut quads {
        if s.custom_size != Some(size) {
            s.custom_size = Some(size);
        }
    }
}

/// Возврат консольного окна для диагностических режимов (релиз без консоли).
fn attach_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn AllocConsole() -> i32;
        }
        unsafe {
            AllocConsole();
        }
    }
}

fn main() {
    if std::env::args().any(|a| a == "--selftest") {
        attach_console();
        selftest::run();
        return;
    }
    if std::env::args().any(|a| a == "--dump-sprites") {
        attach_console();
        dump::run();
        return;
    }
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets_root().to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(window_setup()),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .init_resource::<modals::Modals>()
        .init_resource::<interact::CoolQte>()
        .init_resource::<rush::Rush>()
        .add_systems(Startup, (setup, audio::load_bank).chain())
        .add_systems(
            Update,
            (
                // --- Отладка: первым делом, чтобы ввод не протекал в игру ---
                (
                    debug::toggle_console,
                    debug::input_console,
                    debug::execute_console,
                )
                    .chain(),
                // --- Сцена бандита: детект перехода и варианты ответа ---
                (ambush::ambush_system, ambush::ambush_input)
                    .chain()
                    .run_if(debug_closed),
                (
                    // ТЗ №5 «время не стоит на месте»: в модалках (мойка,
                    // снабжение, выбор тары) мир продолжает жить — гейтится
                    // только игровой ввод. Полная пауза — ESC (Phase::Paused).
                    // Во время заставки «ДЕНЬ N» игрок ещё не управляет собой.
                    player::movement
                        .run_if(in_world)
                        .run_if(modal_closed)
                        .run_if(day_intro_done),
                    player::mouse_turn
                        .run_if(in_world)
                        .run_if(modal_closed)
                        .run_if(day_intro_done),
                    slot_select.run_if(modal_closed),
                    menu::menu_input,
                    day_cycle::phase_system.run_if(modal_closed),
                    day_cycle::day_system.run_if(debug_clock_running),
                    tutorial::tutorial_system
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                    brewing::barrel_update
                        .run_if(in_world)
                        .run_if(debug_clock_running),
                    brewing::brew_hints
                        .run_if(in_world)
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                    customer::customer_update
                        .run_if(in_world)
                        .run_if(no_overbrew_freeze)
                        .run_if(debug_clock_running),
                    enemies::enemies_spawn
                        .run_if(no_overbrew_freeze)
                        .run_if(debug_clock_running),
                    enemies::officials_fine
                        .run_if(in_play)
                        .run_if(no_overbrew_freeze)
                        .run_if(debug_clock_running),
                    // --- День 3: заводской гудок и толпа работяг ---
                    rush::rush_start
                        .run_if(no_overbrew_freeze)
                        .run_if(debug_clock_running),
                    rush::rush_flow
                        .run_if(no_overbrew_freeze)
                        .run_if(debug_clock_running),
                    interact::interact
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                    interact::fill_hold
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                    brewing::first_overbrew_event
                        .run_if(in_play)
                        .run_if(debug_clock_running),
                    interact::brew_qte
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                    interact::cool_qte
                        .run_if(modal_closed)
                        .run_if(debug_clock_running),
                )
                    .chain()
                    .run_if(debug_closed)
                    .run_if(ambush_inactive),
                // --- Подтверждение предыстории ---
                story::story_input
                    .run_if(debug_closed)
                    .run_if(ambush_inactive),
                (
                    // музыка: меню/игра, петли + приглушение под эффекты
                    audio::music,
                    audio::music_volume,
                    audio::ambience,
                    audio::bubble_volume,
                    audio::footsteps,
                    // сигналы бочки слышно и из-под модалки — квас бродит!
                    audio::event_sounds,
                )
                    .chain()
                    .run_if(debug_closed)
                    .run_if(ambush_inactive),
                // Музыка/SFX замирают на открытой отладке и сцене бандита.
                audio::pause_for_overlays,
                // модалки и доставка живут поверх замороженного мира
                modals::modal_input
                    .run_if(debug_closed)
                    .run_if(ambush_inactive),
                modals::supply_tick
                    .run_if(no_overbrew_freeze)
                    .run_if(debug_clock_running)
                    .run_if(ambush_inactive),
                grab_cursor,
                render_frame,
                fit_screen,
            )
                .chain(),
        )
        .run();
}

/// Игровой мир (движение, брожение, очередь) заморожен, пока открыта модалка.
fn modal_closed(modal: Res<modals::Modals>) -> bool {
    !modal.is_open()
}

fn in_play(state: Res<GameState>) -> bool {
    state.phase == Phase::Playing
}

/// Мир отрисовывается и в игровой день, и в обучении.
fn in_world(state: Res<GameState>) -> bool {
    matches!(state.phase, Phase::Playing | Phase::Tutorial)
}

/// Отладочная консоль открыта — игровой ввод и симуляция не работают.
fn debug_closed(console: Res<DebugConsole>) -> bool {
    !console.open
}

/// `stop` в отладке: время, NPC, бочки и таймеры стоят, но игрок может ходить.
fn debug_clock_running(console: Res<DebugConsole>) -> bool {
    !console.time_stopped
}

/// Сцена бандита держит мир на паузе до подтверждения исхода.
fn ambush_inactive(ambush: Res<BanditAmbush>) -> bool {
    !ambush.freeze()
}

/// Пока идёт первый переброд, день и покупатели замирают; игрок при этом
/// свободно ходит, берёт ведро и поливает бочку.
fn no_overbrew_freeze(event: Res<brewing::FirstOverbrew>) -> bool {
    !event.freeze()
}

/// Заставка «ДЕНЬ N» закончилась — можно управлять игроком.
fn day_intro_done(state: Res<GameState>) -> bool {
    state.day_intro <= 0.0
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new(
        Extent3d {
            width: SCREEN_W as u32,
            height: SCREEN_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; SCREEN_W * SCREEN_H * 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    );
    // Квадрат экрана: каждая секунда подгоняется fit_screen под 4:3-сцену
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);

    commands.spawn((
        ScreenQuad,
        Sprite {
            image: handle.clone(),
            custom_size: Some(Vec2::new(SCREEN_W as f32 * 3.0, SCREEN_H as f32 * 3.0)),
            ..default()
        },
    ));

    // ТЗ №5: мусорка — статичный билборд-спрайт (а не тайл-стена), стоит
    // у южной стены зала, между «заказами» и стеллажом посуды.
    // Логика E — в interact::interact по близости.
    commands.spawn(sprites::WorldProp {
        kind: sprites::PropKind::TrashBin,
        pos: Vec2::new(5.5, 8.3),
    });

    // Канвас UI 1280×800 (2× буфер мира) — прозрачный слой поверх мира.
    let mut ui_image = Image::new(
        Extent3d {
            width: CV_W as u32,
            height: CV_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; CV_W as usize * CV_H as usize * 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::all(),
    );
    ui_image.sampler = ImageSampler::nearest();
    let ui_handle = images.add(ui_image);
    commands.insert_resource(ScreenImages {
        world: handle,
        ui: ui_handle.clone(),
    });
    commands.spawn((
        ScreenQuad,
        Sprite {
            image: ui_handle,
            custom_size: Some(Vec2::new(SCREEN_W as f32 * 3.0, SCREEN_H as f32 * 3.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
    commands.insert_resource(Canvas::new());
    commands.spawn(Camera2d);

    // Старт игрока — за Г-прилавком, лицом к прилавку и колонне очереди
    commands.insert_resource(GameCam {
        x: day_cycle::SPAWN_X,
        y: day_cycle::SPAWN_Y,
        angle: day_cycle::SPAWN_ANGLE,
    });
    let map = map::Map::shop();
    commands.insert_resource(Barrels::from_map(&map));
    commands.insert_resource(map);
    commands.insert_resource(Inventory::default());
    commands.insert_resource(GameState::default());
    commands.insert_resource(day_cycle::Dirty::default());
    commands.insert_resource(interact::FillState::default());
    commands.insert_resource(brewing::BrewQte::default());
    commands.insert_resource(brewing::BrewHints::default());
    commands.insert_resource(brewing::FirstOverbrew::default());
    commands.insert_resource(BanditAmbush::default());
    commands.insert_resource(ambush::BanditControl::default());
    commands.insert_resource(DebugConsole::default());
    commands.insert_resource(GameRng(StdRng::seed_from_u64(777)));
    commands.insert_resource(World {
        textures: Textures::generate(),
        sprites: SpriteSheet::generate(),
        renderer: Renderer::new(),
        font: Font::new(),
        flicker: 1.0,
    });
}

fn grab_cursor(
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    state: Res<GameState>,
    debug: Res<DebugConsole>,
    ambush: Res<BanditAmbush>,
) {
    // Мышь привязана к камере сразу при входе в игру/обучение — кликать
    // не нужно. В меню, паузе, отладке и сцене бандита курсор свободен.
    let want_grab = matches!(state.phase, Phase::Playing | Phase::Tutorial)
        && !debug.open
        && !ambush.freeze();
    for mut c in &mut cursors {
        if want_grab {
            if c.grab_mode != CursorGrabMode::Locked || c.visible {
                c.grab_mode = CursorGrabMode::Locked;
                c.visible = false;
            }
        } else if c.grab_mode != CursorGrabMode::None || !c.visible {
            c.grab_mode = CursorGrabMode::None;
            c.visible = true;
        }
    }
}

/// Выбор слота 1–4.
fn slot_select(keys: Res<ButtonInput<KeyCode>>, mut inv: ResMut<Inventory>) {
    for (i, k) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*k) {
            inv.selected = i;
        }
    }
}

fn render_frame(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    map: Res<map::Map>,
    cam: Res<GameCam>,
    state: Res<GameState>,
    inv: Res<Inventory>,
    barrels: Res<Barrels>,
    qte: Res<brewing::BrewQte>,
    cool: Res<interact::CoolQte>,
    // Пара объединена, чтобы render_frame не превысил лимит параметров Bevy.
    ui_extra: (
        Res<brewing::FirstOverbrew>,
        Res<modals::Modals>,
        Res<DebugConsole>,
        Res<BanditAmbush>,
        Res<ambush::BanditControl>,
    ),
    customers: Query<&Customer>,
    props: Query<&sprites::WorldProp>,
    mut world: ResMut<World>,
    mut ui_canvas: ResMut<Canvas>,
    mut images: ResMut<Assets<Image>>,
    screen: Res<ScreenImages>,
) {
    let overbrew = &*ui_extra.0;
    let modal = &*ui_extra.1;
    let debug = &*ui_extra.2;
    let ambush = &*ui_extra.3;
    let bandit_control = &*ui_extra.4;
    // мерцание люминесцентных ламп
    let t = time.elapsed_secs();
    let base = 0.94 + 0.04 * (t * 6.3).sin() + 0.02 * (t * 13.7).sin();
    let drop = if ((t * 1.9).sin() * 13.0).fract() < 0.06 { -0.18 } else { 0.0 };
    world.flicker = (base + drop).clamp(0.6, 1.0);

    // мир
    let prop_list: Vec<sprites::WorldProp> = props.iter().copied().collect();
    let mut rendered = false;
    if matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        let World {
            textures,
            sprites,
            renderer,
            flicker,
            ..
        } = &mut *world;
        renderer.render_with_barrels(&map, textures, &cam, *flicker, Some(&barrels));
        let list: Vec<_> = customers.iter().map(|c| (c.pos, c.kind)).collect();
        renderer.draw_sprites(sprites, &list, &cam, *flicker);
        // статичные предметы (ТЗ №5: урна-мусорка) — тот же billboard-пайплайн
        renderer.draw_props(sprites, &prop_list, &cam, *flicker);
        rendered = true;
    }

    // канвас UI (верхний слой)
    {
        let custs: Vec<ui::CustView> = customers
            .iter()
            .map(|c| ui::CustView {
                pos: c.pos,
                text: c.bubble.clone(),
                order: c.order,
                queued: c.state == customer::CustState::Queued,
            })
            .collect();
        let World { font, .. } = &*world;
        ui_canvas.clear();
        // движется ли игрок (для покачивания предмета в руках)
        let moving = matches!(state.phase, Phase::Playing | Phase::Tutorial)
            && !debug.open
            && !ambush.freeze()
            && keys.any_pressed([
                KeyCode::KeyW,
                KeyCode::KeyA,
                KeyCode::KeyS,
                KeyCode::KeyD,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
            ]);
        ui::draw_frame_with_overbrew(
            &mut ui_canvas,
            font,
            &state,
            &inv,
            &barrels,
            &qte,
            &cool,
            &custs,
            t,
            &cam,
            &map,
            moving,
            modal,
            &prop_list,
            Some(overbrew),
        );
        // Подбитый глаз — поверх HUD, но под окнами отладки/бандита.
        if state.black_eye > 0.0 && matches!(state.phase, Phase::Playing | Phase::Tutorial) {
            ui::draw_black_eye(&mut ui_canvas, t, state.black_eye);
        }
        if ambush.freeze() {
            ui::draw_ambush(&mut ui_canvas, font, &state, &inv, ambush);
        }
        if debug.open {
            ui::draw_debug_console(&mut ui_canvas, font, debug, bandit_control, state.day, t);
        } else if debug.time_stopped {
            ui::draw_debug_time_badge(&mut ui_canvas, font);
        }
    }

    // публикация кадров
    if rendered {
        world.renderer.present();
        // Сцена бандита/«подбитый глаз»: левая половина мира становится
        // крупнопиксельной и красной, как будто глаз заплыл.
        if state.black_eye > 0.0 && matches!(state.phase, Phase::Playing | Phase::Tutorial) {
            ui::black_eye_world(&mut world.renderer.rgba, state.black_eye);
        }
        let rgba = world.renderer.rgba.clone();
        if let Some(mut img) = images.get_mut(&screen.world) {
            img.data = Some(rgba);
        }
    }
    if let Some(mut img) = images.get_mut(&screen.ui) {
        img.data = Some(ui_canvas.px.clone());
    }
}
