//! Clip playback with crossfades on top of Bevy's animation graph.
//!
//! Put an [`Animator`] on an entity whose descendants contain a spawned glTF scene and
//! write the clip you want into its [`AnimationRequest`]. Clips are looked up by the
//! action name they had in Blender. Gameplay decides *what* plays; this crate decides
//! *how* it blends.

use core::time::Duration;

use bevy::{gltf::Gltf, platform::collections::HashMap, prelude::*};

pub struct AnimatorPlugin;

impl Plugin for AnimatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (bind_animators, drive_animators)
                .chain()
                .in_set(AnimatorSystems),
        );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimatorSystems;

/// Name of an animation clip (the Blender action name).
pub type ClipName = &'static str;

/// Crossfade durations between clips.
#[derive(Debug, Clone)]
pub struct Transitions {
    default: Duration,
    overrides: HashMap<(ClipName, ClipName), Duration>,
    phase_synced: Vec<ClipName>,
}

impl Transitions {
    pub fn new(default_secs: f32) -> Self {
        Self {
            default: Duration::from_secs_f32(default_secs),
            overrides: HashMap::default(),
            phase_synced: Vec::new(),
        }
    }

    /// Sets the crossfade used in both directions between `a` and `b`.
    pub fn between(mut self, a: ClipName, b: ClipName, secs: f32) -> Self {
        let duration = Duration::from_secs_f32(secs);
        self.overrides.insert((a, b), duration);
        self.overrides.insert((b, a), duration);
        self
    }

    /// Clips whose cycles share the same timing (e.g. all start on the left foot).
    /// Switching between them keeps the normalized phase so feet don't pop.
    pub fn sync_phase(mut self, clips: impl IntoIterator<Item = ClipName>) -> Self {
        self.phase_synced.extend(clips);
        self
    }

    pub fn duration(&self, from: ClipName, to: ClipName) -> Duration {
        self.overrides
            .get(&(from, to))
            .copied()
            .unwrap_or(self.default)
    }

    fn syncs(&self, from: ClipName, to: ClipName) -> bool {
        self.phase_synced.contains(&from) && self.phase_synced.contains(&to)
    }
}

#[derive(Component, Debug, Clone)]
pub struct Animator {
    pub gltf: Handle<Gltf>,
    pub transitions: Transitions,
}

/// The clip that should be playing and its playback speed.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct AnimationRequest {
    pub clip: ClipName,
    pub speed: f32,
}

impl AnimationRequest {
    pub fn new(clip: ClipName) -> Self {
        Self { clip, speed: 1.0 }
    }
}

#[derive(Debug, Clone, Copy)]
struct BoundClip {
    node: AnimationNodeIndex,
    duration: f32,
}

#[derive(Component)]
struct AnimatorBinding {
    player: Entity,
    clips: HashMap<String, BoundClip>,
    current: Option<ClipName>,
}

fn bind_animators(
    mut commands: Commands,
    animators: Query<(Entity, &Animator), Without<AnimatorBinding>>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, animator) in &animators {
        let Some(player) = children
            .iter_descendants(entity)
            .find(|&child| players.contains(child))
        else {
            continue;
        };
        let Some(gltf) = gltfs.get(&animator.gltf) else {
            continue;
        };

        let mut graph = AnimationGraph::new();
        let mut bound = HashMap::default();
        for (name, handle) in &gltf.named_animations {
            let duration = clips.get(handle).map_or(1.0, AnimationClip::duration);
            let node = graph.add_clip(handle.clone(), 1.0, graph.root);
            bound.insert(name.to_string(), BoundClip { node, duration });
        }

        commands.entity(player).insert((
            AnimationGraphHandle(graphs.add(graph)),
            AnimationTransitions::new(),
        ));
        commands.entity(entity).insert(AnimatorBinding {
            player,
            clips: bound,
            current: None,
        });
    }
}

fn drive_animators(
    mut animators: Query<(&Animator, &mut AnimatorBinding, &AnimationRequest)>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (animator, mut binding, request) in &mut animators {
        let Some(&clip) = binding.clips.get(request.clip) else {
            warn_once!("Animation clip `{}` not found", request.clip);
            continue;
        };
        let Ok((mut player, mut transitions)) = players.get_mut(binding.player) else {
            continue;
        };

        if binding.current != Some(request.clip) {
            let previous = binding.current;
            let phase = previous
                .filter(|&from| animator.transitions.syncs(from, request.clip))
                .and_then(|from| {
                    let from = binding.clips.get(from)?;
                    let active = player.animation(from.node)?;
                    Some((active.seek_time() / from.duration).fract())
                });
            let fade = previous.map_or(Duration::ZERO, |from| {
                animator.transitions.duration(from, request.clip)
            });

            let active = transitions.play(&mut player, clip.node, fade).repeat();
            if let Some(phase) = phase {
                active.seek_to(phase * clip.duration);
            }
            binding.current = Some(request.clip);
        }

        if let Some(active) = player.animation_mut(clip.node) {
            active.set_speed(request.speed);
        }
    }
}
