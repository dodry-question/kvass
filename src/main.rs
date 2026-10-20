//! «КВАСНАЯ ЛАВКА» — Soviet kvass shop Doom-style raycaster.
//! Vyatka_Owls Game Jam. Rust + Bevy.

// Релиз без консольного окна: диагностика в UTF-8 рисовалась бы в legacy-
// кодировке Windows-консоли («ударения» над буквами — артефакт CP852, а не
// шрифта). Для --selftest/--dump-sprites консоль возвращается в main().
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod brewing;
mod canvas;
mod customer;
mod day_cycle;
mod dialogues;
mod enemies;
mod font;
mod inventory;
mod interact;
mod map;
mod modals;
mod palette;
mod player;
mod raycaster;
mod dump;
mod selftest;
mod sprites;
mod textures;
mod tutorial;
mod ui;

use bevy::prelude::*;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, WindowMode};
use rand::prelude::*;

use canvas::{CV_H, CV_W, Canvas};
use brewing::Barrels;
use customer::Customer;
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
/// Настройка окна: полноэкранный borderless либо окно, если задана
/// переменная `KVASS_WINDOWED` (например `KVASS_WINDOWED=1280x800`) —
/// так удобно запускать под Wine/Bottles, где borderless может не открыться.
fn window_setup() -> Window {
    let title = format!("КВАСНАЯ ЛАВКА [{BUILD_ID}]");
    match std::env::var("KVASS_WINDOWED") {
        Ok(v) => {
            let (w, h) = v
                .split_once('x')
                .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                .unwrap_or((1280, 800));
            Window {
                title,
                mode: WindowMode::Windowed,
                resolution: (w, h).into(),
                ..default()
            }
        }
        Err(_) => Window {
            title,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            ..default()
        },
    }
}

/// Идентификатор сборки — выводится в HUD для проверки версии exe (ТЗ).
pub const BUILD_ID: &str = "BUILD 2026-10-20.5 FONT-DIAG";

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

/// Кадр растягивается ровно на всё окно (полноэкранный режим).
#[derive(Component)]
struct ScreenQuad;

fn fit_screen(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut quads: Query<&mut Sprite, With<ScreenQuad>>,
) {
    let Ok(w) = windows.single() else { return };
    let size = Vec2::new(w.resolution.width(), w.resolution.height());
    if size.x < 16.0 || size.y < 16.0 {
        return;
    }
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
        .insert_resource(ClearColor(Color::srgb(0.09, 0.07, 0.06)))
        .init_resource::<modals::Modals>()
        .init_resource::<interact::WashQte>()
        .init_resource::<interact::CoolQte>()
        .add_systems(Startup, (setup, audio::load_bank).chain())
        .add_systems(
            Update,
            (
                (
                    player::movement.run_if(in_world),
                    player::mouse_turn.run_if(in_world),
                    grab_cursor,
                    slot_select,
                    day_cycle::phase_system,
                    day_cycle::day_system,
                    tutorial::tutorial_system,
                    brewing::barrel_update.run_if(in_world),
                    customer::customer_update.run_if(in_world),
                    enemies::enemies_spawn,
                    enemies::officials_fine.run_if(in_play),
                    enemies::hooligan_spawn.run_if(in_play),
                    enemies::hooligan_behavior.run_if(in_play),
                    interact::interact,
                    interact::fill_hold,
                    interact::brew_qte,
                    interact::wash_qte,
                    interact::cool_qte,
                )
                    .chain(),
                (
                    audio::ambience,
                    audio::bubble_volume,
                    audio::footsteps,
                    audio::event_sounds,
                )
                    .chain(),
            )
                .chain()
                // ТЗ №3/№4: пока открыта модалка — мир заморожен.
                .run_if(modal_closed),
        )
        .add_systems(
            Update,
            (
                // модалки и доставка живут поверх замороженного мира
                modals::modal_input,
                modals::supply_tick,
                render_frame,
                fit_screen,
            ),
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
    // Квадрат экрана: растягивается ровно на всё окно (полноэкранный режим)
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

    // Старт игрока — за прилавком, лицом к прилавку и очереди
    commands.insert_resource(GameCam {
        x: 5.5,
        y: 4.8,
        angle: -std::f32::consts::FRAC_PI_2,
    });
    let map = map::Map::shop();
    commands.insert_resource(Barrels::from_map(&map));
    commands.insert_resource(map);
    commands.insert_resource(Inventory::default());
    commands.insert_resource(GameState::default());
    commands.insert_resource(day_cycle::Dirty::default());
    commands.insert_resource(interact::FillState::default());
    commands.insert_resource(brewing::BrewQte::default());
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
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
    state: Res<GameState>,
) {
    // захват курсора — только в игре/обучении (в паузе и меню мышь свободна)
    if buttons.just_pressed(MouseButton::Left)
        && matches!(state.phase, Phase::Playing | Phase::Tutorial)
    {
        for mut c in &mut cursors {
            c.grab_mode = CursorGrabMode::Locked;
            c.visible = false;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        for mut c in &mut cursors {
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
    wash: Res<interact::WashQte>,
    cool: Res<interact::CoolQte>,
    modal: Res<modals::Modals>,
    customers: Query<&Customer>,
    mut world: ResMut<World>,
    mut ui_canvas: ResMut<Canvas>,
    mut images: ResMut<Assets<Image>>,
    screen: Res<ScreenImages>,
) {
    // мерцание люминесцентных ламп
    let t = time.elapsed_secs();
    let base = 0.94 + 0.04 * (t * 6.3).sin() + 0.02 * (t * 13.7).sin();
    let drop = if ((t * 1.9).sin() * 13.0).fract() < 0.06 { -0.18 } else { 0.0 };
    world.flicker = (base + drop).clamp(0.6, 1.0);

    // мир
    let mut rendered = false;
    if matches!(state.phase, Phase::Playing | Phase::Tutorial) {
        let World {
            textures,
            sprites,
            renderer,
            flicker,
            ..
        } = &mut *world;
        renderer.render(&map, textures, &cam, *flicker);
        let list: Vec<_> = customers.iter().map(|c| (c.pos, c.kind)).collect();
        renderer.draw_sprites(sprites, &list, &cam, *flicker);
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
            && keys.any_pressed([
                KeyCode::KeyW,
                KeyCode::KeyA,
                KeyCode::KeyS,
                KeyCode::KeyD,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
            ]);
        ui::draw_frame(
            &mut ui_canvas,
            font,
            &state,
            &inv,
            &barrels,
            &qte,
            &wash,
            &cool,
            &custs,
            t,
            &cam,
            &map,
            moving,
            &modal,
        );
    }

    // публикация кадров
    if rendered {
        world.renderer.present();
        let rgba = world.renderer.rgba.clone();
        if let Some(mut img) = images.get_mut(&screen.world) {
            img.data = Some(rgba);
        }
    }
    if let Some(mut img) = images.get_mut(&screen.ui) {
        img.data = Some(ui_canvas.px.clone());
    }
}
