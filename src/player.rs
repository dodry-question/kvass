//! FPS-контроллер в 2D-плоскости (WASD + мышь/стрелки).

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;

use crate::map::Map;
use crate::raycaster::GameCam;

pub const SPEED: f32 = 3.0;
pub const TURN_SPEED: f32 = 2.5;
pub const MOUSE_SENS: f32 = 0.0025;
pub const RADIUS: f32 = 0.25;
/// ТЗ №7: отступ игрока от низкой мебели — не ближе ~0.6 юнита, иначе
/// игрок «заглядывает» в текстуру прилавка и видит сквозь неё.
/// Обводка фиксированная и одна для ВСЕХ блоков прилавка/полок: прилавок,
/// мойка, стеллаж посуды, бочка, сухари, сахар, вода, окно снабжения.
pub const FURNITURE_STANDOFF: f32 = 0.6;

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
    // ТЗ №7: не подходим ближе FURNITURE_STANDOFF к любой низкой мебели —
    // иначе камера «заглядывает» в неё и видит сквозь фасад. Обводка одна
    // и та же по всем четырём осям, независимо от того, что за тайлом.
    for (px, py) in [
        (x + FURNITURE_STANDOFF, y),
        (x - FURNITURE_STANDOFF, y),
        (x, y + FURNITURE_STANDOFF),
        (x, y - FURNITURE_STANDOFF),
    ] {
        if map.get(px.floor() as i32, py.floor() as i32).furniture() {
            return true;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Точка проходима: внутри пола и не в обводке мебели.
    fn walkable(map: &Map, x: f32, y: f32) -> bool {
        !map.get(x.floor() as i32, y.floor() as i32).solid() && !blocked(map, x, y)
    }

    /// Обводка не должна отрезать единственный проход на склад и подходы
    /// к станциям: волна от точки появления должна дотянуться до всех.
    #[test]
    fn furniture_standoff_keeps_shop_navigable() {
        let map = Map::shop();
        let step = 0.05f32;
        let nx = (crate::map::MAP_W as f32 / step) as usize + 1;
        let ny = (crate::map::MAP_H as f32 / step) as usize + 1;
        let idx = |ix: usize, iy: usize| iy * nx + ix;

        let (sx, sy) = (crate::day_cycle::SPAWN_X, crate::day_cycle::SPAWN_Y);
        assert!(walkable(&map, sx, sy), "точка появления занята обводкой");

        let mut seen = vec![false; nx * ny];
        let mut queue = std::collections::VecDeque::new();
        let start = ((sx / step).round() as usize, (sy / step).round() as usize);
        seen[idx(start.0, start.1)] = true;
        queue.push_back(start);
        while let Some((ix, iy)) = queue.pop_front() {
            for dx in -1..=1i32 {
                for dy in -1..=1i32 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let jx = ix as i32 + dx;
                    let jy = iy as i32 + dy;
                    if jx < 0 || jy < 0 || jx as usize >= nx || jy as usize >= ny {
                        continue;
                    }
                    let (jx, jy) = (jx as usize, jy as usize);
                    if seen[idx(jx, jy)] {
                        continue;
                    }
                    let (x, y) = (jx as f32 * step, jy as f32 * step);
                    if !walkable(&map, x, y) {
                        continue;
                    }
                    seen[idx(jx, jy)] = true;
                    queue.push_back((jx, jy));
                }
            }
        }
        let reachable = |x: f32, y: f32| {
            seen[idx((x / step).round() as usize, (y / step).round() as usize)]
        };

        for (name, x, y) in [
            ("прилавок", 6.5, 5.7),
            ("мойка", 7.4, 5.7),
            ("бочка", 6.5, 7.3),
            ("полка посуды", 7.5, 7.3),
            ("окно снабжения", 2.7, 6.5),
            ("коридор", 11.5, 6.5),
            ("склад", 15.0, 6.5),
            ("сухари", 15.5, 5.7),
            ("сахар", 15.5, 7.3),
            ("вода", 15.0, 6.5),
        ] {
            assert!(reachable(x, y), "не дойти до точки «{name}» ({x}, {y})");
        }
    }
}
