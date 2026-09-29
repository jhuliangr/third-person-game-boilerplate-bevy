//! Level loading: geometry exported from Blender, static colliders, spawn points and lighting.

use bevy::{prelude::*, world_serialization::WorldAsset};
use gf_core::{AppState, LoadingQueue, PlayerSpawn};
use gf_physics::{GameLayer, avian3d::prelude::*};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.53, 0.68, 0.85)))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.75, 0.82, 1.0),
                brightness: 200.0,
                ..default()
            })
            .add_systems(Startup, load_level)
            .add_systems(OnEnter(AppState::InGame), spawn_level);
    }
}

const LEVEL_PATH: &str = "levels/sandbox.glb";
/// Blender objects whose name starts with this become player spawn points.
const SPAWN_NODE_PREFIX: &str = "PlayerSpawn";

#[derive(Resource)]
struct LevelAssets {
    scene: Handle<WorldAsset>,
}

fn load_level(mut commands: Commands, server: Res<AssetServer>, mut queue: ResMut<LoadingQueue>) {
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(LEVEL_PATH));
    queue.track(scene.clone());
    commands.insert_resource(LevelAssets { scene });
}

fn spawn_level(mut commands: Commands, assets: Res<LevelAssets>) {
    commands
        .spawn((
            Name::new("Level"),
            WorldAssetRoot(assets.scene.clone()),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
                .with_default_layers(GameLayer::world()),
            DespawnOnExit(AppState::InGame),
        ))
        .observe(mark_spawn_points);

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(-0.4, -1.0, -0.6), Vec3::Y),
        DespawnOnExit(AppState::InGame),
    ));
}

fn mark_spawn_points(
    ready: On<ColliderConstructorHierarchyReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for entity in children.iter_descendants(ready.entity) {
        if names
            .get(entity)
            .is_ok_and(|name| name.as_str().starts_with(SPAWN_NODE_PREFIX))
        {
            commands.entity(entity).insert(PlayerSpawn);
        }
    }
}
