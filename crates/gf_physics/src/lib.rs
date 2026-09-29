//! Physics setup and the collision layers shared by every crate.

use avian3d::prelude::*;
use bevy::prelude::*;

pub use avian3d;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());

        #[cfg(feature = "debug")]
        app.add_plugins(PhysicsDebugPlugin)
            .add_systems(Startup, disable_debug_gizmos)
            .add_systems(
                Update,
                toggle_debug_gizmos.run_if(bevy::input::common_conditions::input_just_pressed(
                    KeyCode::F1,
                )),
            );
    }
}

#[derive(PhysicsLayer, Debug, Clone, Copy, Default)]
pub enum GameLayer {
    #[default]
    Default,
    /// Static level geometry.
    World,
    /// Player and NPC capsules.
    Character,
}

impl GameLayer {
    pub fn world() -> CollisionLayers {
        CollisionLayers::new(GameLayer::World, LayerMask::ALL)
    }

    pub fn character() -> CollisionLayers {
        CollisionLayers::new(
            GameLayer::Character,
            [GameLayer::Default, GameLayer::World, GameLayer::Character],
        )
    }
}

#[cfg(feature = "debug")]
fn disable_debug_gizmos(mut store: ResMut<GizmoConfigStore>) {
    store.config_mut::<PhysicsGizmos>().0.enabled = false;
}

#[cfg(feature = "debug")]
fn toggle_debug_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let config = store.config_mut::<PhysicsGizmos>().0;
    config.enabled = !config.enabled;
}
