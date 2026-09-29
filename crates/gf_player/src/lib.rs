//! The player character: spawning, input to [`MoveIntent`] and animation selection.

use bevy::{gltf::Gltf, prelude::*, transform::TransformSystems};
use gf_animation::{AnimationRequest, Animator, AnimatorSystems, Transitions};
use gf_camera::{CameraTarget, OrbitCamera};
use gf_character::{CharacterController, Gait, Locomotion, MoveIntent, Stance};
use gf_core::{AppState, GameplaySystems, LoadingQueue, PlayerSpawn};
use gf_input::{Action, Actions, Crouch, Gameplay, Move, Sprint, gameplay_input};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, AnimatorSystems.after(GameplaySystems::Presentation))
            .add_systems(Startup, load_assets)
            .add_systems(
                PostUpdate,
                spawn_player
                    .after(TransformSystems::Propagate)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, read_input.in_set(GameplaySystems::Input))
            .add_systems(
                Update,
                (select_animation, update_camera_height).in_set(GameplaySystems::Presentation),
            );
    }
}

const MODEL_PATH: &str = "models/player/player.glb";
const STANDING_EYE_HEIGHT: f32 = 1.55;
const CROUCHING_EYE_HEIGHT: f32 = 1.05;

/// Clip names as authored in `player.blend`.
pub mod clips {
    pub const IDLE: &str = "Idle";
    pub const WALK: &str = "Walk";
    pub const RUN: &str = "Run";
    pub const CROUCH_IDLE: &str = "CrouchIdle";
    pub const CROUCH_WALK: &str = "CrouchWalk";
}

/// Ground speed (m/s) at which each cycle plays at 1x without foot sliding.
/// Measured from the stride length in the Blender clips.
const WALK_REFERENCE_SPEED: f32 = 1.7;
const RUN_REFERENCE_SPEED: f32 = 3.3;
const CROUCH_WALK_REFERENCE_SPEED: f32 = 0.8;
const MIN_PLAYBACK_SPEED: f32 = 0.6;
const MAX_PLAYBACK_SPEED: f32 = 1.6;

#[derive(Component, Debug, Default)]
pub struct Player;

#[derive(Resource)]
struct PlayerAssets {
    gltf: Handle<Gltf>,
}

fn transitions() -> Transitions {
    use clips::*;
    Transitions::new(0.25)
        .between(IDLE, WALK, 0.2)
        .between(WALK, RUN, 0.3)
        .between(IDLE, RUN, 0.3)
        .between(IDLE, CROUCH_IDLE, 0.3)
        .between(WALK, CROUCH_WALK, 0.3)
        .between(CROUCH_IDLE, CROUCH_WALK, 0.2)
        .sync_phase([WALK, RUN, CROUCH_WALK])
}

fn load_assets(mut commands: Commands, server: Res<AssetServer>, mut queue: ResMut<LoadingQueue>) {
    let gltf = server.load(MODEL_PATH);
    queue.track(gltf.clone());
    commands.insert_resource(PlayerAssets { gltf });
}

fn spawn_player(
    mut commands: Commands,
    assets: Res<PlayerAssets>,
    gltfs: Res<Assets<Gltf>>,
    spawns: Query<&GlobalTransform, With<PlayerSpawn>>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    let Some(spawn) = spawns.iter().next() else {
        return;
    };
    let Some(scene) = gltfs
        .get(&assets.gltf)
        .and_then(|gltf| gltf.default_scene.clone())
    else {
        error_once!("{MODEL_PATH} has no default scene");
        return;
    };

    commands.spawn((
        Name::new("Player"),
        Player,
        CharacterController::default(),
        Transform::from_translation(spawn.translation()),
        CameraTarget {
            height: STANDING_EYE_HEIGHT,
        },
        Animator {
            gltf: assets.gltf.clone(),
            transitions: transitions(),
        },
        AnimationRequest::new(clips::IDLE),
        gameplay_input(),
        DespawnOnExit(AppState::InGame),
        children![(Name::new("PlayerModel"), WorldAssetRoot(scene))],
    ));
}

fn read_input(
    cameras: Query<&OrbitCamera>,
    moves: Query<&Action<Move>>,
    sprints: Query<&Action<Sprint>>,
    crouches: Query<&Action<Crouch>>,
    mut players: Query<(&Actions<Gameplay>, &mut MoveIntent), With<Player>>,
) {
    let yaw = cameras
        .single()
        .map_or(Quat::IDENTITY, OrbitCamera::yaw_rotation);

    for (actions, mut intent) in &mut players {
        let input = moves
            .iter_many(actions)
            .next()
            .map_or(Vec2::ZERO, |action| **action);
        intent.direction = yaw * Vec3::new(input.x, 0.0, -input.y).clamp_length_max(1.0);
        intent.sprint = sprints.iter_many(actions).any(|action| **action);
        intent.crouch = crouches.iter_many(actions).any(|action| **action);
    }
}

fn select_animation(mut players: Query<(&Locomotion, &mut AnimationRequest), With<Player>>) {
    for (locomotion, mut request) in &mut players {
        let (clip, reference_speed) = match (locomotion.stance, locomotion.gait) {
            (Stance::Standing, Gait::Idle) => (clips::IDLE, None),
            (Stance::Standing, Gait::Walk) => (clips::WALK, Some(WALK_REFERENCE_SPEED)),
            (Stance::Standing, Gait::Run) => (clips::RUN, Some(RUN_REFERENCE_SPEED)),
            (Stance::Crouching, Gait::Idle) => (clips::CROUCH_IDLE, None),
            (Stance::Crouching, _) => (clips::CROUCH_WALK, Some(CROUCH_WALK_REFERENCE_SPEED)),
        };
        let speed = reference_speed.map_or(1.0, |reference| {
            (locomotion.speed / reference).clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED)
        });
        request.set_if_neq(AnimationRequest { clip, speed });
    }
}

fn update_camera_height(
    mut players: Query<(&Locomotion, &mut CameraTarget), (With<Player>, Changed<Locomotion>)>,
) {
    for (locomotion, mut target) in &mut players {
        target.height = match locomotion.stance {
            Stance::Standing => STANDING_EYE_HEIGHT,
            Stance::Crouching => CROUCHING_EYE_HEIGHT,
        };
    }
}
