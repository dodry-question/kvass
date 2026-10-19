//! «КВАСНАЯ ЛАВКА» — Soviet kvass shop Doom-style raycaster.
//! Vyatka_Owls Game Jam. Rust + Bevy.

mod audio;
mod brewing;
mod customer;
mod day_cycle;
mod dialogues;
mod enemies;
mod font;
mod inventory;
mod interact;
mod map;
mod palette;
mod player;
mod raycaster;
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

#[derive(Resource)]
struct ScreenImage(Handle<Image>);

/// Каталог assets: рядом с exe, в текущей папке или в исходниках (dev-сборка).
pub fn assets_root() -> std::path::PathBuf {
    let mut bases: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            bases.push(d.to_path_buf());
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
    let Ok(mut s) = quads.single_mut() else { return };
    if s.custom_size != Some(size) {
        s.custom_size = Some(size);
    }
}

fn main() {
    if std::env::args().any(|a| a == "--selftest") {
        selftest::run();
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
                    primary_window: Some(Window {
                        title: "КВАСНАЯ ЛАВКА".into(),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.09, 0.07, 0.06)))
        .add_systems(Startup, (setup, audio::load_bank).chain())
        .add_systems(
            Update,
            (
                (
                    player::movement.run_if(in_world),
                    player::mouse_turn,
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
                    audio::ambience,
                    audio::bubble_volume,
                    audio::footsteps,
                    audio::event_sounds,
                )
                    .chain(),
                render_frame,
                fit_screen,
            ),
        )
        .run();
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
    commands.insert_resource(ScreenImage(handle.clone()));

    commands.spawn((
        ScreenQuad,
        Sprite {
            image: handle,
            custom_size: Some(Vec2::new(SCREEN_W as f32 * 3.0, SCREEN_H as f32 * 3.0)),
            ..default()
        },
    ));
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
) {
    if buttons.just_pressed(MouseButton::Left) {
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
    map: Res<map::Map>,
    cam: Res<GameCam>,
    state: Res<GameState>,
    inv: Res<Inventory>,
    barrels: Res<Barrels>,
    qte: Res<brewing::BrewQte>,
    customers: Query<&Customer>,
    mut world: ResMut<World>,
    mut images: ResMut<Assets<Image>>,
    handle: Res<ScreenImage>,
) {
    // мерцание люминесцентных ламп
    let t = time.elapsed_secs();
    let base = 0.94 + 0.04 * (t * 6.3).sin() + 0.02 * (t * 13.7).sin();
    let drop = if ((t * 1.9).sin() * 13.0).fract() < 0.06 { -0.18 } else { 0.0 };
    world.flicker = (base + drop).clamp(0.6, 1.0);

    // мир
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
    }

    // HUD / экраны
    let renders: Vec<ui::CustomerRender> = customers
        .iter()
        .map(|c| ui::CustomerRender {
            kind: c.kind,
            pos: c.pos,
            bubble: c.bubble.clone(),
        })
        .collect();
    let view = ui::CustomerView { list: &renders };
    {
        let World {
            renderer, font, ..
        } = &mut *world;
        ui::draw_frame(renderer, font, &state, &inv, &barrels, &qte, &view, t, &cam, &map);
        renderer.present();
        let rgba = renderer.rgba.clone();
        if let Some(mut img) = images.get_mut(&handle.0) {
            img.data = Some(rgba);
        }
    }
}
