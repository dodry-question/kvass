//! FPS-контроллер в 2D-плоскости (WASD + мышь/стрелки).

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;

use crate::map::Map;
use crate::raycaster::GameCam;

pub const SPEED: f32 = 3.0;
pub const TURN_SPEED: f32 = 2.5;
pub const MOUSE_SENS: f32 = 0.0025;
pub const RADIUS: f32 = 0.25;
/// ТЗ №7: отступ игрока от прилавка — не ближе ~0.55 юнита, иначе игрок
/// «заглядывает» в текстуру прилавка и видит сквозь неё.
pub const COUNTER_STANDOFF: f32 = 0.6;

pub fn movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cam: ResMut<GameCam>,
    map: Res<Map>,
) {
    let dt = time.delta_secs();
    let mut forward = 0.0f32;
    let mut strafe = 0.0f32;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        forward += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        forward -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        strafe -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        strafe += 1.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        cam.angle -= TURN_SPEED * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        cam.angle += TURN_SPEED * dt;
    }
    if forward == 0.0 && strafe == 0.0 {
        return;
    }
    let len = (forward * forward + strafe * strafe).sqrt();
    let forward = forward / len;
    let strafe = strafe / len;
    let (dx, dy) = (
        (cam.angle.cos() * forward - cam.angle.sin() * strafe) * SPEED * dt,
        (cam.angle.sin() * forward + cam.angle.cos() * strafe) * SPEED * dt,
    );
    // раздельная коллизия по осям
    if !blocked(&map, cam.x + dx, cam.y) {
        cam.x += dx;
    }
    if !blocked(&map, cam.x, cam.y + dy) {
        cam.y += dy;
    }
}

fn blocked(map: &Map, x: f32, y: f32) -> bool {
    // Касса (9,3): не подходим ближе 0.6 ни с юга, ни с запада (ТЗ №6/7)
    if (9.0..10.0).contains(&x) && y >= 4.0 && y < 4.0 + COUNTER_STANDOFF {
        return true;
    }
    if x >= 9.0 - COUNTER_STANDOFF && x < 9.0 && (3.0..4.0).contains(&y) {
        return true;
    }
    for (ox, oy) in [(-RADIUS, -RADIUS), (RADIUS, -RADIUS), (-RADIUS, RADIUS), (RADIUS, RADIUS)] {
        if map.get((x + ox).floor() as i32, (y + oy).floor() as i32).solid() {
            return true;
        }
    }
    false
}

/// Поворот мышью. Захват курсора включается автоматически при входе в игру
/// (см. `grab_cursor`), здесь только читаем накопленное смещение.
pub fn mouse_turn(
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut cam: ResMut<GameCam>,
) {
    let mut dx = 0.0;
    for ev in motion.read() {
        // При захвате/перецентровке курсора ОС может прислать один большой
        // скачок — такие события игнорируем, чтобы камеру не дёргало.
        if ev.delta.length_squared() > 300.0 * 300.0 {
            continue;
        }
        dx += ev.delta.x;
    }
    cam.angle += dx * MOUSE_SENS;
}
