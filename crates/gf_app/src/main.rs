//! Entry point: engine configuration and the list of game plugins.

use bevy::{
    app::PluginGroupBuilder,
    gltf::{GltfPlugin, convert_coordinates::GltfConvertCoordinates},
    prelude::*,
    window::WindowResolution,
};

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Game Foundation".into(),
                    resolution: WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            })
            .set(GltfPlugin {
                // Blender's front (-Y) becomes Bevy's forward (-Z).
                convert_coordinates: GltfConvertCoordinates {
                    rotate_scene_entity: true,
                    rotate_meshes: false,
                },
                ..default()
            }),
    )
    .add_plugins(GamePlugins);

    #[cfg(feature = "dev")]
    app.add_plugins(bevy::dev_tools::fps_overlay::FpsOverlayPlugin::default());

    app.run()
}

/// Every game feature, in dependency order. New features are added here.
pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(gf_core::CorePlugin)
            .add(gf_settings::SettingsPlugin {
                app_name: "game-foundation",
            })
            .add(gf_render::RenderPlugin)
            .add(gf_input::InputPlugin)
            .add(gf_physics::PhysicsPlugin)
            .add(gf_character::CharacterPlugin)
            .add(gf_animation::AnimatorPlugin)
            .add(gf_camera::OrbitCameraPlugin)
            .add(gf_world::WorldPlugin)
            .add(gf_player::PlayerPlugin)
            .add(gf_ui::UiPlugin)
    }
}
