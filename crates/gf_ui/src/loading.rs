use bevy::prelude::*;
use gf_core::AppState;

use crate::widgets::{full_screen, label};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), spawn_loading_screen);
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        Name::new("LoadingScreen"),
        DespawnOnExit(AppState::Loading),
        full_screen(Color::BLACK),
        children![label("Loading...", 32.0)],
    ));
}
