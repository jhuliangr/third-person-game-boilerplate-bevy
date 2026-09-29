//! Graphics quality presets applied to every 3D camera and directional light.
//!
//! [`GraphicsQuality::Low`] is the default so the game runs on weak integrated GPUs.
//! Change the resource at runtime and everything is reconfigured.

use bevy::{
    anti_alias::fxaa::Fxaa,
    light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphicsQuality>()
            .add_observer(on_camera_added)
            .add_observer(on_light_added)
            .add_systems(
                PostUpdate,
                apply_quality.run_if(resource_changed::<GraphicsQuality>),
            );
    }
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GraphicsQuality {
    #[default]
    Low,
    Medium,
    High,
}

impl GraphicsQuality {
    pub fn msaa(self) -> Msaa {
        match self {
            Self::Low | Self::Medium => Msaa::Off,
            Self::High => Msaa::Sample4,
        }
    }

    pub fn fxaa(self) -> bool {
        self == Self::Medium
    }

    pub fn shadow_map_size(self) -> usize {
        match self {
            Self::Low => 1024,
            Self::Medium => 2048,
            Self::High => 4096,
        }
    }

    pub fn shadow_cascades(self) -> CascadeShadowConfig {
        let (num_cascades, maximum_distance) = match self {
            Self::Low => (1, 25.0),
            Self::Medium => (2, 40.0),
            Self::High => (4, 80.0),
        };
        CascadeShadowConfigBuilder {
            num_cascades,
            maximum_distance,
            first_cascade_far_bound: maximum_distance / num_cascades as f32,
            ..default()
        }
        .build()
    }
}

fn configure_camera(commands: &mut Commands, camera: Entity, quality: GraphicsQuality) {
    let mut entity = commands.entity(camera);
    entity.insert(quality.msaa());
    if quality.fxaa() {
        entity.insert(Fxaa::default());
    } else {
        entity.remove::<Fxaa>();
    }
}

fn on_camera_added(add: On<Add, Camera3d>, mut commands: Commands, quality: Res<GraphicsQuality>) {
    configure_camera(&mut commands, add.entity, *quality);
}

fn on_light_added(
    add: On<Add, DirectionalLight>,
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
) {
    commands
        .entity(add.entity)
        .insert(quality.shadow_cascades());
}

fn apply_quality(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    cameras: Query<Entity, With<Camera3d>>,
    lights: Query<Entity, With<DirectionalLight>>,
) {
    commands.insert_resource(DirectionalLightShadowMap {
        size: quality.shadow_map_size(),
    });
    for camera in &cameras {
        configure_camera(&mut commands, camera, *quality);
    }
    for light in &lights {
        commands.entity(light).insert(quality.shadow_cascades());
    }
}
