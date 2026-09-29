# Architecture

## Principles

1. **One feature, one crate, one plugin.** Each crate in `crates/` exposes a single Bevy
   `Plugin`. The binary (`gf_app`) only configures the engine and registers the plugins.
2. **Dependencies point downward.** Low-level crates never know about higher-level ones.
   Communication upward happens through components, states and events.
3. **Gameplay reads intents, not devices.** Keys and sticks are translated into actions in
   `gf_input`; characters are driven by a `MoveIntent` component that player input, AI or
   networking can write.
4. **Presentation reads state, never drives it.** Animation, camera and UI observe gameplay
   state (`Locomotion`, `PauseState`, ...) and don't feed back into simulation.
5. **Assets are data.** Models, clips and levels come from Blender files in `assets/`;
   code references clips and spawn points by name.

## Crates

| Crate          | Responsibility                                                              |
|----------------|-----------------------------------------------------------------------------|
| `gf_app`       | Binary. Window and engine settings, `GamePlugins` group, `dev` feature.     |
| `gf_core`      | `AppState`, `PauseState`, `GameplaySystems` sets, `LoadingQueue`, `PlayerSpawn`. |
| `gf_input`     | Input contexts (`Gameplay`, `Menu`), actions and default bindings, cursor grab. |
| `gf_physics`   | Avian setup, `GameLayer` collision layers, debug gizmos.                    |
| `gf_character` | Kinematic character controller: `MoveIntent` → movement, crouch, `Locomotion`. |
| `gf_animation` | Clip playback by name with crossfades and phase sync (`Animator`).          |
| `gf_camera`    | Orbit camera with collision around a `CameraTarget`.                        |
| `gf_player`    | Spawns the player and bridges input → intent and locomotion → animation.   |
| `gf_world`     | Loads the level `.glb`, builds static colliders, marks spawn points, lighting. |
| `gf_render`    | `GraphicsQuality` presets applied to cameras and lights.                   |
| `gf_ui`        | Loading screen and pause menu.                                              |

### Dependency graph

```
gf_app ──► every crate below

gf_player ──► gf_character ──► gf_physics
    │    ──► gf_animation
    │    ──► gf_camera ──► gf_input ──► gf_core
    │                 ──► gf_physics
    └────► gf_input, gf_core
gf_world ──► gf_physics, gf_core
gf_ui    ──► gf_input, gf_core
gf_render      (bevy only)
gf_animation   (bevy only)
```

`gf_character` and `gf_animation` don't depend on input or on each other, so NPCs can reuse
them without pulling in anything player-specific.

## States

```
AppState::Loading ──(all LoadingQueue assets ready)──► AppState::InGame
                                                          │
                                         PauseState::Running ⇄ PauseState::Paused
```

- `PauseState` is a sub-state that only exists during `InGame`.
- Entering `Paused` pauses `Time<Virtual>`, which freezes `FixedUpdate` (physics, character
  movement) and animations. UI keeps working because it doesn't depend on virtual time.
- Input contexts follow the state automatically: `Gameplay` is active only while `Running`,
  `Menu` only while `Paused` (`ActiveInStates` from bevy_enhanced_input).
- Entities that belong to a state are spawned with `DespawnOnExit(...)`.

## Frame flow

```
PreUpdate      bevy_enhanced_input evaluates the active contexts → Action<T> values/events
FixedUpdate    CharacterSystems: stance → velocity → move-and-slide → ground probe → facing → Locomotion
               (Avian's physics step runs afterwards in FixedPostUpdate)
RunFixedMainLoop  transform interpolation smooths the character between fixed steps
Update         GameplaySystems::Input         read actions → MoveIntent, camera yaw/pitch
               GameplaySystems::Logic         (free for game rules)
               GameplaySystems::Presentation  Locomotion → AnimationRequest, camera height
               AnimatorSystems                AnimationRequest → crossfades on AnimationPlayer
PostUpdate     camera follows its target with collision, then transforms propagate
```

`GameplaySystems` only run while `PauseState::Running`.

## Player entity layout

```
Player  (CharacterController, MoveIntent, Locomotion, CameraTarget, Animator,
│        AnimationRequest, Gameplay input context, RigidBody::Kinematic)
├── CharacterCollider  (capsule Collider, resized when crouching)
└── PlayerModel        (WorldAssetRoot → player.glb scene with AnimationPlayer)
```

The player origin is at the feet. The collider is a child offset by half its height, so
crouching only changes the child and never teleports the character.

## Loading

Plugins request their assets in `Startup` and register the handles in `LoadingQueue`.
`gf_core` switches to `InGame` once every tracked asset and its dependencies are loaded,
so gameplay systems can assume assets exist.
