# Game Foundation

A third-person game boilerplate written in Rust with [Bevy](https://bevyengine.org).
It ships a humanoid player that walks, runs and crouches with blended animations, an orbit
camera, a pause menu and a sandbox level. Everything is split into small plugin crates so any
kind of game can be built on top.

## Requirements

- Rust stable (MSVC toolchain on Windows) — see `rust-version` in `Cargo.toml`
- Visual Studio Build Tools with the C++ workload (Windows only)
- Blender 5.2+ (only to edit or regenerate the 3D assets)

## Running

```sh
cargo run                    # debug build, dependencies optimized
cargo run --features dev     # fast iteration: dynamic linking, FPS overlay, F1 physics gizmos
cargo run --release          # optimized build for playtesting and distribution
```

The first build compiles Bevy and takes several minutes; later builds are incremental.

## Controls

| Action        | Keyboard / Mouse | Gamepad          |
|---------------|------------------|------------------|
| Move          | W A S D          | Left stick       |
| Look          | Mouse            | Right stick      |
| Run           | Left Shift       | Left stick press |
| Crouch (hold) | Left Ctrl        | East (B / ○)     |
| Pause menu    | Esc              | Start            |

## Project layout

```
assets/            Runtime assets (.glb) next to their editable Blender sources (.blend)
crates/            One crate per feature, each exposing a single Bevy plugin
docs/              Architecture, decisions, scaling guide, asset pipeline, performance
tools/blender/     Scripts that generate and export the 3D assets
```

## Documentation

- [Architecture](docs/architecture.md): how the crates fit together and how data flows each frame
- [Decisions](docs/decisions.md): every technical choice and why it was made
- [Scaling guide](docs/scaling.md): recipes for adding features, actions, animations, NPCs and levels
- [Asset pipeline](docs/assets.md): editing models and levels in Blender
- [Performance](docs/performance.md): low-end optimizations and quality presets
- [Contributing](docs/contributing.md): git workflow and code style
