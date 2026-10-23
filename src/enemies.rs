//! Проверяющие: милиция (штраф, взятка банкой).
//!
//! Бандитов в очереди больше нет — они появляются только из-за угла
//! в коридоре (см. `ambush.rs`). Санэпидемнадзор из игры убран:
//! третий день занимает «час пик» (см. `rush.rs`).

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::customer::{CustState, Customer, QUEUE_SPOTS};
use crate::day_cycle::{GameState, GameRng, Phase};
use crate::dialogues;
use crate::sprites::NpcKind;

/// Проверяющий-милиционер: взятка банкой снимает штраф.
#[derive(Component)]
pub struct Official;

/// Спавн проверяющих по дню.
pub fn enemies_spawn(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    customers: Query<&Customer>,
    bank: Res<crate::audio::SoundBank>,
    rush: Res<crate::rush::Rush>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    // Во время «Часа пик» проверяющие не лезут в толпу работяг.
    if rush.blocks_customers() {
        return;
    }
    let count = customers.iter().count();
    // милиционер раз в день со 2-го дня
    if state.day >= 2 && (state.time_left - 150.0).abs() < 0.6 && count < 4 {
        let _ = spawn_official(&mut commands, NpcKind::Militsioner);
        crate::audio::play_sfx(&mut commands, &bank, "whistle", 0.6);
        state.notify("МИЛИЦИЯ ИДЁТ ПРОВЕРЯТЬ...".into());
    }
}

fn spawn_official(commands: &mut Commands, kind: NpcKind) -> Option<usize> {
    // последнее место очереди — ближе к двери
    let spot = QUEUE_SPOTS.len() - 1;
    let c = Customer {
        kind,
        order: crate::sprites::Vessel::Jar,
        state: CustState::Entering,
        pos: crate::customer::DOOR,
        path: VecDeque::from([Vec2::new(QUEUE_SPOTS[spot].0, QUEUE_SPOTS[spot].1)]),
        patience: 80.0,
        max_patience: 80.0,
        queue_patience: 78.0,
        bubble: None,
        bubble_t: 0.0,
        spot,
        served: false,
        step_t: 0.0,
    };
    commands.spawn((c, Official));
    Some(spot)
}

/// Штраф, если проверяющий ушёл с пустыми руками.
pub fn officials_fine(
    mut q: Query<(&mut Customer, &Official)>,
    mut state: ResMut<GameState>,
    mut game_rng: ResMut<GameRng>,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let rng = &mut game_rng.0;
    for (mut c, _) in &mut q {
        if c.state == CustState::Leaving && !c.served {
            let reason = dialogues::militsioner_line(rng);
            state.money -= 30;
            state.fine += 30;
            state.fine_reason = reason.to_string();
            state.notify(format!("ШТРАФ 30 К. {}", reason));
            c.served = true; // чтобы не сработало дважды
        }
    }
}
