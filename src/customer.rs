//! Покупатели: приходят с улицы, ждут в очереди, заказывают текстом, уходят.

use std::collections::VecDeque;

use bevy::prelude::*;
use rand::prelude::*;

use crate::dialogues;
use crate::sprites::{NpcKind, Vessel};

pub const SPEED: f32 = 1.6;
const ENTER_TIME: f32 = 95.0;
const QUEUE_TIME: f32 = 78.0;
const BUBBLE_TIME: f32 = 5.0;

/// Точки очереди — колонна от прилавка к входу (x=5.5):
/// spot 0 — первым у прилавка (единственный, чья терпимость тикает).
pub const QUEUE_SPOTS: [(f32, f32); 3] = [(5.5, 3.5), (5.5, 2.5), (5.5, 1.5)];

/// Вход (северная стена, x=5..6) — появления и ухода, над колонной очереди.
pub const DOOR: Vec2 = Vec2::new(5.5, 0.5);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CustState {
    /// Идёт по маршруту к очереди.
    Entering,
    /// Стоит в очереди, ждёт заказ.
    Queued,
    /// Уходит (доволен или недоволен).
    Leaving,
    /// Особое поведение (враги) — движением рулит своя система.
    Custom,
}

#[derive(Component)]
pub struct Customer {
    pub kind: NpcKind,
    pub order: Vessel,
    pub state: CustState,
    pub pos: Vec2,
    pub path: VecDeque<Vec2>,
    pub patience: f32,
    pub max_patience: f32,
    pub bubble: Option<String>,
    pub bubble_t: f32,
    pub spot: usize,
    pub served: bool,
    /// Таймер анимации ножек.
    pub step_t: f32,
}

impl Customer {
    pub fn say(&mut self, text: String) {
        self.bubble = Some(text);
        self.bubble_t = BUBBLE_TIME;
    }
}

/// Маршрут от двери к точке очереди (все полы открыты — напрямую).
fn path_to_spot(spot: usize) -> VecDeque<Vec2> {
    let (sx, sy) = QUEUE_SPOTS[spot];
    VecDeque::from([Vec2::new(sx, sy)])
}

pub fn spawn_customer(
    commands: &mut Commands,
    kinds: &[NpcKind],
    taken_spots: &[bool],
    rng: &mut StdRng,
) -> Option<usize> {
    let spot = (0..QUEUE_SPOTS.len()).find(|&i| !taken_spots[i])?;
    let kind = *kinds.choose(rng).unwrap();
    let opts = [Vessel::Glass, Vessel::Jar, Vessel::Bottle];
    let order = *opts.choose(rng).unwrap();
    let c = Customer {
        kind,
        order,
        state: CustState::Entering,
        pos: DOOR,
        path: path_to_spot(spot),
        patience: ENTER_TIME,
        max_patience: ENTER_TIME,
        bubble: None,
        bubble_t: 0.0,
        spot,
        served: false,
        step_t: 0.0,
    };
    commands.spawn(c);
    Some(spot)
}

/// Движение, таймеры, реплики.
pub fn customer_update(
    time: Res<Time>,
    state: Res<crate::day_cycle::GameState>,
    mut q: Query<(Entity, &mut Customer)>,
    mut commands: Commands,
    mut game_rng: ResMut<crate::day_cycle::GameRng>,
) {
    let dt = time.delta_secs();
    // в обучении покупатели не торопятся уходить
    let tut = state.phase == crate::day_cycle::Phase::Tutorial;
    let mut rng = &mut game_rng.0;
    let mut despawn: Vec<Entity> = Vec::new();
    for (e, mut c) in &mut q {
        if c.bubble_t > 0.0 {
            c.bubble_t -= dt;
            if c.bubble_t <= 0.0 {
                c.bubble = None;
            }
        }
        match c.state {
            CustState::Entering => {
                if !tut {
                    c.patience -= dt;
                }
                if let Some(t) = c.path.front() {
                    let d = *t - c.pos;
                    let len = d.length();
                    if len < 0.15 {
                        c.path.pop_front();
                        if c.path.is_empty() {
                            c.state = CustState::Queued;
                            c.patience = QUEUE_TIME;
                            c.max_patience = QUEUE_TIME;
                            let line = dialogues::order_line(c.kind, c.order, &mut rng);
                            c.say(line);
                        }
                    } else {
                        let step = d / len * SPEED * dt;
                        c.pos += step;
                        c.step_t += dt;
                    }
                }
                if c.patience <= 0.0 {
                    let line = dialogues::angry_line(c.kind, &mut rng);
                    c.say(line);
                    c.state = CustState::Leaving;
                    c.path = VecDeque::from([DOOR]);
                }
            }
            CustState::Queued => {
                // ТЗ №6: терпимость тикает только у ПЕРВОГО у прилавка.
                if !tut && c.spot == 0 {
                    c.patience -= dt;
                    // нетерпеливые реплики на середине ожидания
                    if (c.patience - c.max_patience * 0.5).abs() < dt {
                        let line = dialogues::angry_line(c.kind, &mut rng);
                        c.say(line);
                    }
                    if c.patience <= 0.0 {
                        let line = dialogues::leaving_line(c.kind, &mut rng);
                        c.say(line.to_string());
                        c.state = CustState::Leaving;
                        c.path = VecDeque::from([DOOR]);
                    }
                }
            }
            CustState::Leaving => {
                if let Some(t) = c.path.front() {
                    let d = *t - c.pos;
                    let len = d.length();
                    if len < 0.15 {
                        c.path.pop_front();
                        if c.path.is_empty() {
                            despawn.push(e);
                        }
                    } else {
                        c.pos += d / len * SPEED * 1.4 * dt;
                        c.step_t += dt;
                    }
                } else {
                    despawn.push(e);
                }
            }
            CustState::Custom => {}
        }
    }
    for e in despawn {
        commands.entity(e).despawn();
    }
    // ТЗ №6: очередь уплотняется — освободилось место у прилавка,
    // все сзади шагают вперёд на ступень ближе.
    let mut shift: Vec<(Entity, usize)> = Vec::new();
    {
        let mut list: Vec<(Entity, usize)> = q
            .iter()
            .filter(|(_, c)| matches!(c.state, CustState::Queued | CustState::Entering))
            .map(|(e, c)| (e, c.spot))
            .collect();
        list.sort_by_key(|&(_, s)| s);
        for (i, (e, s)) in list.into_iter().enumerate() {
            if i < QUEUE_SPOTS.len() && i != s {
                shift.push((e, i));
            }
        }
    }
    for (e, nspot) in shift {
        if let Ok((_, mut c)) = q.get_mut(e) {
            c.spot = nspot;
            c.path = path_to_spot(nspot);
            // ТЗ №6: очередь реально ПОДХОДИТ вперёд. Если человек уже стоял
            // в очереди, переводим его в Entering — иначе менялся только
            // номер места, а сам он оставался на старом (очередь «баговалась»:
            // первый ушёл, а второй стоял на своём месте).
            if c.state == CustState::Queued {
                c.state = CustState::Entering;
            }
        }
    }
}

/// Идёт ли кто-то из покупателей на эту точку (для выбора свободного слота).
pub fn spot_taken(q: &Query<&Customer>, spot: usize) -> bool {
    q.iter().any(|c| c.spot == spot && c.state != CustState::Leaving)
}
