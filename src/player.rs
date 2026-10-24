//! FPS-контроллер в 2D-плоскости (WASD + мышь/стрелки).

use bevy::prelude::*;
use bevy::ecs::message::MessageReader;

use crate::map::{Map, Tile};
use crate::raycaster::GameCam;

pub const SPEED: f32 = 3.0;
pub const TURN_SPEED: f32 = 2.5;
pub const MOUSE_SENS: f32 = 0.0025;
pub const RADIUS: f32 = 0.25;
/// ТЗ №7 (переработка): лёгкая обводка низкой мебели. Коробка игрока
/// (RADIUS + FURNITURE_STANDOFF) проверяется против тайлов мебели
/// целиком, а не четырьмя точками по осям: старая проба пропускала
/// диагональ — в углах прилавка камера «заходила» внутрь полублока.
/// Стенка тонкая (0.2 поверх радиуса), чтобы не перекрывать проходы.
pub const FURNITURE_STANDOFF: f32 = 0.2;

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

/// Пересекает ли коробка игрока [x±r, y±r] хотя бы один тайл с предикатом.
fn box_hits(map: &Map, x: f32, y: f32, r: f32, pred: impl Fn(Tile) -> bool) -> bool {
    let x0 = (x - r).floor() as i32;
    let x1 = (x + r).floor() as i32;
    let y0 = (y - r).floor() as i32;
    let y1 = (y + r).floor() as i32;
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if pred(map.get(tx, ty)) {
                return true;
            }
        }
    }
    false
}

fn blocked(map: &Map, x: f32, y: f32) -> bool {
    // ТЗ №7: коробка против тайлов — фасады и УГЛЫ мебели закрыты
    // одинаково (4-точечная проба по осям пропускала диагональ).
    box_hits(map, x, y, RADIUS, |t| t.solid())
        || box_hits(map, x, y, RADIUS + FURNITURE_STANDOFF, |t| t.furniture())
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

    /// Углы прилавка закрыты так же, как фасады: по диагонали к угловому
    /// тайлу вплотную зайти нельзя (старая 4-точечная проба это пропускала,
    /// и камера «проваливалась» в полублок — визуальный баг у стойки).
    #[test]
    fn counter_corners_stay_blocked() {
        let map = Map::shop();
        // Углы Г-образного прилавка: внешний (4,4)-(8,4) и внутренний (4,7).
        for (cx, cy) in [(4.0, 4.0), (8.0, 4.0), (4.0, 7.0)] {
            // диагональный подход: центр игрока на 0.4 по диагонали от угла
            let d = 0.4f32;
            let dx = cx + if cx >= 8.0 { d } else { -d };
            let dy = cy - d;
            assert!(
                blocked(&map, dx, dy),
                "угол прилавка ({cx},{cy}) пропускает диагональ ({dx:.2},{dy:.2})"
            );
        }
        // Но вплотную «по прямой» подход остаётся возможным с разумным зазором:
        assert!(!blocked(&map, 6.5, 5.7), "подход к прилавку стал толще, чем нужно");
        assert!(!blocked(&map, 15.5, 5.7), "подход к сухарям закрыт обводкой");
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
