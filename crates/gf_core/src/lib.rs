//! Shared vocabulary for every game crate: app states, system sets and loading.

mod loading;

use bevy::prelude::*;

pub use loading::LoadingQueue;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<PauseState>()
            .configure_sets(
                Update,
                (
                    GameplaySystems::Input,
                    GameplaySystems::Logic,
                    GameplaySystems::Presentation,
                )
                    .chain()
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(OnEnter(PauseState::Paused), pause_time)
            .add_systems(OnExit(PauseState::Paused), resume_time)
            .add_plugins(loading::plugin);
    }
}

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}

#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InGame)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

/// Frame-rate dependent gameplay work in `Update`, only while the game is running.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameplaySystems {
    Input,
    Logic,
    Presentation,
}

/// Marks a place where the player can appear. Levels add it to their spawn nodes.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PlayerSpawn;

fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn resume_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
