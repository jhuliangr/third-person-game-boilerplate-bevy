//! Graphics settings applied to every 3D camera, directional light and the window.
//!
//! [`GraphicsPreset::Low`] is the default so the game runs on weak integrated GPUs.
//! Change the [`GraphicsSettings`] resource at runtime and everything is reconfigured;
//! the choice is persisted through `gf_settings`.

use bevy::{
    anti_alias::fxaa::Fxaa,
    light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    window::{PresentMode, PrimaryWindow},
};
use gf_settings::SettingsAppExt;
use serde::{Deserialize, Serialize};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.register_settings::<GraphicsSettings>("graphics")
            .add_observer(on_camera_added)
            .add_observer(on_light_added)
            .add_systems(
                PostUpdate,
                apply_settings.run_if(resource_changed::<GraphicsSettings>),
            );
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub shadows: ShadowQuality,
    pub anti_aliasing: AntiAliasing,
    pub vsync: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        GraphicsPreset::Low.settings()
    }
}

impl GraphicsSettings {
    /// The preset these settings match, or `None` when they were customized.
    pub fn preset(&self) -> Option<GraphicsPreset> {
        GraphicsPreset::ALL
            .into_iter()
            .find(|preset| preset.settings() == *self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphicsPreset {
    Low,
    Medium,
    High,
}

impl GraphicsPreset {
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    pub fn settings(self) -> GraphicsSettings {
        let (shadows, anti_aliasing) = match self {
            Self::Low => (ShadowQuality::Low, AntiAliasing::Off),
            Self::Medium => (ShadowQuality::Medium, AntiAliasing::Fxaa),
            Self::High => (ShadowQuality::High, AntiAliasing::Msaa4),
        };
        GraphicsSettings {
            shadows,
            anti_aliasing,
            vsync: true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShadowQuality {
    Off,
    Low,
    Medium,
    High,
}

impl ShadowQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }

    fn map_size(self) -> usize {
        match self {
            Self::Off | Self::Low => 1024,
            Self::Medium => 2048,
            Self::High => 4096,
        }
    }

    fn cascades(self) -> CascadeShadowConfig {
        let (num_cascades, maximum_distance) = match self {
            Self::Off | Self::Low => (1, 25.0),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AntiAliasing {
    Off,
    Fxaa,
    Msaa4,
}

impl AntiAliasing {
    pub const ALL: [Self; 3] = [Self::Off, Self::Fxaa, Self::Msaa4];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fxaa => "FXAA",
            Self::Msaa4 => "MSAA 4x",
        }
    }
}

fn configure_camera(commands: &mut Commands, camera: Entity, settings: &GraphicsSettings) {
    let mut entity = commands.entity(camera);
    match settings.anti_aliasing {
        AntiAliasing::Off => {
            entity.insert(Msaa::Off).remove::<Fxaa>();
        }
        AntiAliasing::Fxaa => {
            entity.insert((Msaa::Off, Fxaa::default()));
        }
        AntiAliasing::Msaa4 => {
            entity.insert(Msaa::Sample4).remove::<Fxaa>();
        }
    }
}

fn configure_light(
    light: &mut DirectionalLight,
    settings: &GraphicsSettings,
) -> CascadeShadowConfig {
    light.shadow_maps_enabled = settings.shadows != ShadowQuality::Off;
    settings.shadows.cascades()
}

fn on_camera_added(
    add: On<Add, Camera3d>,
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
) {
    configure_camera(&mut commands, add.entity, &settings);
}

fn on_light_added(
    add: On<Add, DirectionalLight>,
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    mut lights: Query<&mut DirectionalLight>,
) {
    if let Ok(mut light) = lights.get_mut(add.entity) {
        let cascades = configure_light(&mut light, &settings);
        commands.entity(add.entity).insert(cascades);
    }
}

fn apply_settings(
    mut commands: Commands,
    settings: Res<GraphicsSettings>,
    cameras: Query<Entity, With<Camera3d>>,
    mut lights: Query<(Entity, &mut DirectionalLight)>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    commands.insert_resource(DirectionalLightShadowMap {
        size: settings.shadows.map_size(),
    });
    for camera in &cameras {
        configure_camera(&mut commands, camera, &settings);
    }
    for (entity, mut light) in &mut lights {
        let cascades = configure_light(&mut light, &settings);
        commands.entity(entity).insert(cascades);
    }
    for mut window in &mut windows {
        window.present_mode = if settings.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
    }
}
