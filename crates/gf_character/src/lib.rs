//! Kinematic character controller shared by the player and any future NPC.
//!
//! Characters are driven exclusively through [`MoveIntent`], so the same controller works
//! for player input, AI, replays or networking. The resulting [`Locomotion`] is what
//! presentation layers (animation, audio, VFX) should read.

use bevy::prelude::*;
use gf_physics::{GameLayer, avian3d::prelude::*};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(attach_collider).add_systems(
            FixedUpdate,
            (
                update_stance,
                update_velocity,
                move_characters,
                probe_ground,
                face_movement,
                update_locomotion,
            )
                .chain()
                .in_set(CharacterSystems),
        );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterSystems;

const SKIN: f32 = 0.01;
const GROUND_TOLERANCE: f32 = 0.05;
const HEADROOM_MARGIN: f32 = 0.02;

/// Tuning for a character. The entity's origin is at its feet.
#[derive(Component, Debug, Clone)]
#[require(
    Transform,
    Visibility,
    RigidBody::Kinematic,
    CustomPositionIntegration,
    TransformInterpolation,
    MoveIntent,
    CharacterMotion,
    Locomotion
)]
pub struct CharacterController {
    pub radius: f32,
    pub standing_height: f32,
    pub crouching_height: f32,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub crouch_speed: f32,
    /// Exponential rate at which velocity approaches the target speed.
    pub acceleration: f32,
    /// Exponential rate at which the character turns toward its movement direction.
    pub turn_speed: f32,
    pub gravity: f32,
    pub max_fall_speed: f32,
    /// Steepest walkable slope, in radians.
    pub max_slope: f32,
    /// How far down the character sticks to the ground when walking off slopes or small steps.
    pub snap_distance: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            radius: 0.3,
            standing_height: 1.8,
            crouching_height: 1.2,
            walk_speed: 1.8,
            run_speed: 4.2,
            crouch_speed: 1.1,
            acceleration: 10.0,
            turn_speed: 12.0,
            gravity: 20.0,
            max_fall_speed: 40.0,
            max_slope: 45f32.to_radians(),
            snap_distance: 0.3,
        }
    }
}

impl CharacterController {
    pub fn height(&self, stance: Stance) -> f32 {
        match stance {
            Stance::Standing => self.standing_height,
            Stance::Crouching => self.crouching_height,
        }
    }

    pub fn max_speed(&self, stance: Stance, sprint: bool) -> f32 {
        match (stance, sprint) {
            (Stance::Crouching, _) => self.crouch_speed,
            (Stance::Standing, true) => self.run_speed,
            (Stance::Standing, false) => self.walk_speed,
        }
    }

    fn capsule(&self, stance: Stance) -> Collider {
        let height = self.height(stance);
        Collider::capsule(self.radius, (height - 2.0 * self.radius).max(0.0))
    }

    fn collider_offset(&self, stance: Stance) -> Vec3 {
        Vec3::Y * self.height(stance) * 0.5
    }
}

/// What the character wants to do this frame. Written by input or AI.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MoveIntent {
    /// World-space direction on the ground plane with a length in `0..=1`.
    pub direction: Vec3,
    pub sprint: bool,
    pub crouch: bool,
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CharacterMotion {
    pub velocity: Vec3,
    pub grounded: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stance {
    #[default]
    Standing,
    Crouching,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gait {
    #[default]
    Idle,
    Walk,
    Run,
}

/// High level movement state, meant for animation and other presentation.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct Locomotion {
    pub stance: Stance,
    pub gait: Gait,
    /// Horizontal speed in meters per second.
    pub speed: f32,
}

/// Points from a character to the child entity holding its collider.
#[derive(Component, Debug, Clone, Copy)]
pub struct CharacterCollider(pub Entity);

fn query_filter(collider: Entity) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(GameLayer::character().filters).with_excluded_entities([collider])
}

fn attach_collider(
    add: On<Add, CharacterController>,
    mut commands: Commands,
    controllers: Query<&CharacterController>,
) {
    let Ok(controller) = controllers.get(add.entity) else {
        return;
    };
    let collider = commands
        .spawn((
            Name::new("CharacterCollider"),
            controller.capsule(Stance::Standing),
            GameLayer::character(),
            Transform::from_translation(controller.collider_offset(Stance::Standing)),
            ChildOf(add.entity),
        ))
        .id();
    commands
        .entity(add.entity)
        .insert(CharacterCollider(collider));
}

fn update_stance(
    mut commands: Commands,
    spatial: SpatialQuery,
    mut characters: Query<(
        &CharacterController,
        &MoveIntent,
        &CharacterCollider,
        &Transform,
        &mut Locomotion,
    )>,
) {
    for (controller, intent, body, transform, mut locomotion) in &mut characters {
        let wanted = if intent.crouch {
            Stance::Crouching
        } else {
            Stance::Standing
        };
        if wanted == locomotion.stance {
            continue;
        }

        if wanted == Stance::Standing {
            let origin = transform.translation
                + controller.collider_offset(Stance::Standing)
                + Vec3::Y * HEADROOM_MARGIN;
            let blocked = !spatial
                .shape_intersections(
                    &controller.capsule(Stance::Standing),
                    origin,
                    Quat::IDENTITY,
                    &query_filter(body.0),
                )
                .is_empty();
            if blocked {
                continue;
            }
        }

        commands.entity(body.0).insert((
            controller.capsule(wanted),
            Transform::from_translation(controller.collider_offset(wanted)),
        ));
        locomotion.stance = wanted;
    }
}

fn update_velocity(
    time: Res<Time>,
    mut characters: Query<(
        &CharacterController,
        &MoveIntent,
        &Locomotion,
        &mut CharacterMotion,
    )>,
) {
    let dt = time.delta_secs();
    for (controller, intent, locomotion, mut motion) in &mut characters {
        let direction = intent.direction.with_y(0.0).clamp_length_max(1.0);
        let target = direction * controller.max_speed(locomotion.stance, intent.sprint);
        let blend = 1.0 - (-controller.acceleration * dt).exp();
        let horizontal = motion.velocity.with_y(0.0).lerp(target, blend);

        let vertical = if motion.grounded {
            0.0
        } else {
            (motion.velocity.y - controller.gravity * dt).max(-controller.max_fall_speed)
        };
        motion.velocity = horizontal.with_y(vertical);
    }
}

fn move_characters(
    move_and_slide: MoveAndSlide,
    time: Res<Time>,
    mut characters: Query<(
        &CharacterController,
        &CharacterCollider,
        &mut Transform,
        &mut CharacterMotion,
    )>,
    colliders: Query<(&Collider, &Transform), Without<CharacterController>>,
) {
    for (controller, body, mut transform, mut motion) in &mut characters {
        let Ok((collider, offset)) = colliders.get(body.0) else {
            continue;
        };
        let min_ground_normal_y = controller.max_slope.cos();

        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation + offset.translation,
            Quat::IDENTITY,
            motion.velocity,
            time.delta(),
            &MoveAndSlideConfig::default(),
            &query_filter(body.0),
            |hit| {
                // Treat steep slopes as walls so they can't be climbed.
                let normal = **hit.normal;
                if normal.y > 0.0
                    && normal.y < min_ground_normal_y
                    && let Ok(wall) = Dir3::new(normal.with_y(0.0))
                {
                    *hit.normal = wall;
                }
                MoveAndSlideHitResponse::Accept
            },
        );

        transform.translation = output.position - offset.translation;
        motion.velocity = output.projected_velocity;
    }
}

fn probe_ground(
    spatial: SpatialQuery,
    mut characters: Query<(
        &CharacterController,
        &CharacterCollider,
        &mut Transform,
        &mut CharacterMotion,
    )>,
    colliders: Query<(&Collider, &Transform), Without<CharacterController>>,
) {
    for (controller, body, mut transform, mut motion) in &mut characters {
        let Ok((collider, offset)) = colliders.get(body.0) else {
            continue;
        };
        let was_grounded = motion.grounded;
        motion.grounded = false;
        if motion.velocity.y > 0.01 {
            continue;
        }

        let Some(hit) = spatial.cast_shape(
            collider,
            transform.translation + offset.translation,
            Quat::IDENTITY,
            Dir3::NEG_Y,
            &ShapeCastConfig::from_max_distance(controller.snap_distance),
            &query_filter(body.0),
        ) else {
            continue;
        };

        let walkable = hit.normal1.y >= controller.max_slope.cos();
        if walkable && (was_grounded || hit.distance <= GROUND_TOLERANCE) {
            transform.translation.y -= (hit.distance - SKIN).max(0.0);
            motion.velocity.y = 0.0;
            motion.grounded = true;
        }
    }
}

fn face_movement(
    time: Res<Time>,
    mut characters: Query<(&CharacterController, &MoveIntent, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (controller, intent, mut transform) in &mut characters {
        let direction = intent.direction.with_y(0.0);
        if direction.length_squared() < 0.01 {
            continue;
        }
        let yaw = f32::atan2(-direction.x, -direction.z);
        let target = Quat::from_rotation_y(yaw);
        let blend = 1.0 - (-controller.turn_speed * dt).exp();
        transform.rotation = transform.rotation.slerp(target, blend);
    }
}

fn update_locomotion(
    mut characters: Query<(
        &CharacterController,
        &CharacterMotion,
        &MoveIntent,
        &mut Locomotion,
    )>,
) {
    for (controller, motion, intent, mut locomotion) in &mut characters {
        let speed = motion.velocity.with_y(0.0).length();
        let wants_to_move = intent.direction.length_squared() > 0.01;

        locomotion.speed = speed;
        locomotion.gait = if speed < 0.1 || (!wants_to_move && speed < 0.3) {
            Gait::Idle
        } else if intent.sprint
            && locomotion.stance == Stance::Standing
            && speed > controller.walk_speed * 1.05
        {
            Gait::Run
        } else {
            Gait::Walk
        };
    }
}
