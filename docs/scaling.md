# Scaling guide

Recipes for growing the foundation into a full game. Each one keeps the rules from the
[architecture](architecture.md): one plugin per crate, dependencies pointing downward,
gameplay driven by intents and presentation driven by state.

## Add a new feature crate

1. Create `crates/gf_<feature>/` with a `Cargo.toml` that uses the workspace fields:

   ```toml
   [package]
   name = "gf_inventory"
   version.workspace = true
   edition.workspace = true
   rust-version.workspace = true
   publish.workspace = true

   [dependencies]
   bevy.workspace = true
   gf_core.workspace = true

   [lints]
   workspace = true
   ```

2. Expose one plugin from `src/lib.rs`:

   ```rust
   pub struct InventoryPlugin;

   impl Plugin for InventoryPlugin {
       fn build(&self, app: &mut App) {
           app.add_systems(Update, update_inventory.in_set(GameplaySystems::Logic));
       }
   }
   ```

3. Add it to `[workspace.dependencies]` in the root `Cargo.toml`, to `gf_app/Cargo.toml`
   and to `GamePlugins` in `gf_app/src/main.rs`.

Put gameplay rules in `GameplaySystems::Logic` so they pause automatically. Physics-driven
logic goes in `FixedUpdate`.

## Add an input action

1. Define the action in `gf_input`:

   ```rust
   #[derive(InputAction)]
   #[action_output(bool)]
   pub struct Jump;
   ```

2. Bind it inside `gameplay_input()`:

   ```rust
   (Action::<Jump>::new(), bindings![KeyCode::Space, GamepadButton::South]),
   ```

3. React to it wherever it's needed, either with an observer
   (`fn jump(_: On<Start<Jump>>, ...)`) or by querying `Action<Jump>`.

For a new mode (driving, swimming, dialogue) create a new context component, register it
with `add_input_context` and switch it on and off with `ContextActivity` or `ActiveInStates`.

## Add a movement ability (jump, dash, climb...)

1. Add the request to `MoveIntent` (e.g. `jump: bool`) and its tuning to
   `CharacterController`.
2. Handle it in `gf_character`'s `update_velocity` (for a jump: set `velocity.y` when
   `motion.grounded`).
3. Expose any new state in `Locomotion` (e.g. `airborne`) so animation can react.
4. Write the intent from `gf_player::read_input`.

AI and networked characters get the ability for free.

## Add an animation state

1. Create the action in Blender (`player.blend`) with the new clip name, e.g. `Jump`, and
   re-export the `.glb` (see [assets](assets.md)).
2. Add the name to `gf_player::clips` and map the new `Locomotion` state to it in
   `select_animation`.
3. Add crossfade durations in `transitions()` with `.between("Jump", "Idle", 0.15)`.
4. For looping locomotion cycles, measure the ground speed at which feet don't slide and add
   a reference speed constant.

Upper-body-only actions (aiming, waving) should use Bevy animation masks with an additive
layer on the same `AnimationGraph`; `gf_animation` is the place to add that.

## Add an NPC

```rust
commands.spawn((
    Name::new("Guard"),
    CharacterController { walk_speed: 1.4, ..default() },
    Transform::from_xyz(5.0, 0.0, 5.0),
    Animator { gltf, transitions },
    AnimationRequest::new("Idle"),
    children![WorldAssetRoot(scene)],
));
```

Then write its `MoveIntent` from an AI crate (`gf_ai`). When there are several animated
character types, move `select_animation` and the reference speeds from `gf_player` into a
shared `LocomotionAnimations` component.

## Add or edit a level

1. Model the level in Blender. Every mesh becomes a static collider; keep meshes simple.
2. Place an empty named `PlayerSpawn` where the player should start.
3. Export with `tools/blender/export_glb.py`.
4. For multiple levels, replace `LEVEL_PATH` in `gf_world` with a `CurrentLevel` resource and
   reload by leaving and re-entering `AppState::InGame`, since every level entity uses
   `DespawnOnExit`.

Use Blender custom properties (exported as glTF extras) to mark triggers, pickups or
doors, and read them with Bevy's `GltfExtras` component.

## Add persistent settings

Derive `Serialize`, `Deserialize` and `Default` on a resource and register it from its
crate's plugin (after `SettingsPlugin` in `GamePlugins`):

```rust
#[derive(Resource, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: f32,
}

app.register_settings::<AudioSettings>("audio");
```

It is loaded from `%APPDATA%\game-foundation\audio.ron` at startup and saved whenever it
changes. `#[serde(default)]` keeps old files valid when fields are added. `InputSettings`
(sensitivity, invert Y) can be persisted the same way.

## Add a pause menu tab or option

All of it lives in `gf_ui/src/pause_menu.rs`:

1. **New tab:** add a variant to `PauseTab`, a tab button in `tab_bar()` and a panel built
   with `panel(PauseTab::YourTab)` inside `spawn_pause_menu`.
2. **New option:** add a variant to the options enum (like `GraphicsOption`) with its
   `label`, `value` and `step`, then add an `option_row(option, order)` to the panel. The
   arrows, gamepad navigation, value refresh and saving work automatically.
   Selectable rows carry a `Focusable` whose `order` must be contiguous within the tab.
3. **New button:** add a variant to `MenuAction` and handle it in `handle_actions`.

Reusable pieces (`button`, `label`, palette) are in `gf_ui/src/widgets.rs`.

## Add audio

Enable Bevy's `"audio"` feature in the workspace `Cargo.toml`, then create `gf_audio`. Drive
footsteps from `Locomotion` changes or from Bevy animation events placed on the clips.

## Add saving and loading

Keep persistent data in plain components and resources, derive `Reflect` and `serde`, and
save through a `gf_save` crate. Avoid saving render or physics internals; rebuild them from
gameplay state on load.

## Networking

Because characters are driven by `MoveIntent` and simulated in `FixedUpdate`, the controller
is ready for client-side prediction and server authority (e.g. with `lightyear`): replicate
intents from clients and `Transform` + `CharacterMotion` from the server.

## Stairs

The controller snaps down small drops but doesn't step up ledges. Either model stairs with an
invisible ramp collider (cheapest), or add a step-up pass in `gf_character` that casts up,
forward and down when a wall hit is lower than a step height.

## Upgrading Bevy

Upgrade Bevy, Avian and bevy_enhanced_input together in the workspace `Cargo.toml`, follow
Bevy's migration guide and commit as `[UPDATE] Bevy 0.x`.
