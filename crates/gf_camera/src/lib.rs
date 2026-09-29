//! Third-person orbit camera that follows a [`CameraTarget`] and avoids clipping into walls.

use bevy::{prelude::*, transform::TransformSystems};
use gf_core::GameplaySystems;
use gf_input::{Action, InputSettings, Look};
use gf_physics::{GameLayer, avian3d::prelude::*};

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, apply_look.in_set(GameplaySystems::Input))
            .add_systems(
                PostUpdate,
                follow_target.before(TransformSystems::Propagate),
            );
    }
}

/// The entity the camera orbits around. Its origin is expected at the feet.
#[derive(Component, Debug, Clone)]
pub struct CameraTarget {
    /// Height of the orbit pivot above the target's origin.
    pub height: f32,
}

#[derive(Component, Debug, Clone)]
pub struct OrbitCamera {
    pub yaw: f32,
    /// Negative values look down.
    pub pitch: f32,
    pub min_pitch: f32,
    pub max_pitch: f32,
    pub distance: f32,
    /// Horizontal offset from the pivot, positive to the right.
    pub shoulder_offset: f32,
    /// How fast the pivot follows height changes such as crouching.
    pub height_sharpness: f32,
    /// How fast the camera moves back out after an obstacle is gone.
    pub recover_sharpness: f32,
    probe: Collider,
    pivot_height: Option<f32>,
    current_distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.25,
            min_pitch: -1.2,
            max_pitch: 0.7,
            distance: 4.0,
            shoulder_offset: 0.0,
            height_sharpness: 10.0,
            recover_sharpness: 6.0,
            probe: Collider::sphere(0.2),
            pivot_height: None,
            current_distance: 4.0,
        }
    }
}

impl OrbitCamera {
    /// Rotation around the vertical axis only, useful for camera-relative movement.
    pub fn yaw_rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
    }

    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("MainCamera"),
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 65f32.to_radians(),
            ..default()
        }),
        OrbitCamera::default(),
    ));
}

fn apply_look(
    settings: Res<InputSettings>,
    looks: Query<&Action<Look>>,
    mut cameras: Query<&mut OrbitCamera>,
) {
    let delta: Vec2 = looks.iter().map(|look| **look).sum();
    if delta == Vec2::ZERO {
        return;
    }
    let invert = if settings.invert_y { -1.0 } else { 1.0 };
    for mut camera in &mut cameras {
        camera.yaw -= delta.x * settings.look_sensitivity;
        camera.pitch = (camera.pitch - delta.y * settings.look_sensitivity * invert)
            .clamp(camera.min_pitch, camera.max_pitch);
    }
}

fn follow_target(
    time: Res<Time>,
    spatial: SpatialQuery,
    targets: Query<(&Transform, &CameraTarget), Without<OrbitCamera>>,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let Ok((target_transform, target)) = targets.single() else {
        return;
    };
    let dt = time.delta_secs();

    for (mut transform, mut camera) in &mut cameras {
        let height = match camera.pivot_height {
            Some(current) => {
                current.lerp(target.height, 1.0 - (-camera.height_sharpness * dt).exp())
            }
            None => target.height,
        };
        camera.pivot_height = Some(height);

        let rotation = camera.rotation();
        let pivot = target_transform.translation
            + Vec3::Y * height
            + rotation * Vec3::X * camera.shoulder_offset;
        let back = Dir3::new_unchecked((rotation * Vec3::Z).normalize());

        let free_distance = spatial
            .cast_shape(
                &camera.probe,
                pivot,
                Quat::IDENTITY,
                back,
                &ShapeCastConfig::from_max_distance(camera.distance),
                &SpatialQueryFilter::from_mask(GameLayer::World),
            )
            .map_or(camera.distance, |hit| hit.distance);

        camera.current_distance = if free_distance < camera.current_distance {
            free_distance
        } else {
            camera
                .current_distance
                .lerp(free_distance, 1.0 - (-camera.recover_sharpness * dt).exp())
        };

        transform.translation = pivot + back * camera.current_distance;
        transform.rotation = rotation;
    }
}
