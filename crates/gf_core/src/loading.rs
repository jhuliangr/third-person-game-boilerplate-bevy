use bevy::prelude::*;

use crate::AppState;

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<LoadingQueue>().add_systems(
        Update,
        advance_when_loaded.run_if(in_state(AppState::Loading)),
    );
}

/// Assets that must be fully loaded before leaving [`AppState::Loading`].
///
/// Plugins push their handles here during `Startup`.
#[derive(Resource, Default)]
pub struct LoadingQueue(Vec<UntypedHandle>);

impl LoadingQueue {
    pub fn track(&mut self, handle: impl Into<UntypedHandle>) {
        self.0.push(handle.into());
    }
}

fn advance_when_loaded(
    queue: Res<LoadingQueue>,
    server: Res<AssetServer>,
    mut next: ResMut<NextState<AppState>>,
) {
    let mut ready = true;
    for handle in &queue.0 {
        if server.load_state(handle.id()).is_failed() {
            error_once!("Failed to load {:?}", handle.path());
        }
        ready &= server.is_loaded_with_dependencies(handle.id());
    }
    if ready {
        next.set(AppState::InGame);
    }
}
