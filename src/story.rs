//! Предыстория перед первым днём.
//!
//! Меню «ВХОД» (и финал обучения, если оно было входом в игру) переводит
//! состояние в `Phase::Story`. Игрок читает текст, подтверждает Enter/
//! Пробелом/E — и только тогда `start_day` включает чёрную заставку «ДЕНЬ 1».

use bevy::prelude::*;

use crate::customer::Customer;
use crate::day_cycle::{give_start_package, start_day, Dirty, GameState, MenuView, Phase};
use crate::inventory::Inventory;

/// Текст предыстории — ровно тот, что согласован в задании.
pub const STORY_TEXT: &str = "Ваш дедушка был известен своим квасом по всему союзу, однако его династия не продолжилась из-за вашего отца инженера.\n\nОднажды вы были в гостях у своей бабушки и решили залезть на чердак. Там было обнаружено немыслимое: рецепт того самого дедушкиного кваса. Когда вы увидели его, то внутри вас проскочила искра - надо открыть свою сеть квасных лавок.\n\nСможете ли вы оправдать имя вашего деда?";

pub fn story_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
    mut inv: ResMut<Inventory>,
    mut barrels: ResMut<crate::brewing::Barrels>,
    mut dirty: ResMut<Dirty>,
    mut cam: ResMut<crate::raycaster::GameCam>,
    customers: Query<Entity, With<Customer>>,
    mut commands: Commands,
) {
    if state.phase != Phase::Story {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.phase = Phase::Menu;
        state.menu_view = MenuView::Main;
        state.menu_sel = 0;
        keys.clear();
        return;
    }
    let go = keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::KeyE);
    if !go {
        return;
    }
    for e in customers.iter() {
        commands.entity(e).despawn();
    }
    state.msg_t = 0.0;
    state.face_mood = 0;
    state.face_t = 0.0;
    // Стартовый пакет — только после подтверждения: выход из предыстории
    // не должен обнулять уже начатую игру.
    give_start_package(&mut state, &mut inv);
    // После подтверждения — обычный старт дня: чёрный экран «ДЕНЬ 1»,
    // рассеивание тьмы и телепорт за прилавок.
    start_day(&mut state, &mut barrels, &mut dirty, &mut cam);
    keys.clear();
}
