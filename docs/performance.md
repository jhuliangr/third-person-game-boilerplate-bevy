# Performance

Target: run smoothly on low-end PCs, meaning any DX12/Vulkan-capable integrated GPU
(Intel HD 500 series and newer, AMD APUs) with 4 GB of RAM.

## What is already optimized

### Rendering

- The `Low` graphics preset is the default (`gf_render`). Players can change it, or each
  option individually, from the pause menu's Graphics tab:

  | Preset | Shadows                    | Anti-aliasing | VSync |
  |--------|----------------------------|---------------|-------|
  | Low    | Low (1024², 1 cascade)     | Off           | On    |
  | Medium | Medium (2048², 2 cascades) | FXAA          | On    |
  | High   | High (4096², 4 cascades)   | MSAA 4x       | On    |

  Shadows can also be turned off entirely for the weakest GPUs.

- No HDR, bloom, SSAO, SSR, volumetrics or other post-processing.
- VSync (`PresentMode::AutoVsync`) caps the frame rate to the display, saving power and heat.
- Low-poly assets: the player is ~1.3k triangles with 3 materials; the level uses 4 shared
  materials and no textures.
- Bevy's automatic batching, GPU preprocessing and frustum culling are active by default.

### Simulation

- Physics and character movement run at a fixed timestep (`FixedUpdate`, 64 Hz), and
  transform interpolation keeps rendering smooth at any frame rate.
- The character controller is kinematic (a few shape casts per step) instead of a
  simulated rigid body.
- The level uses static trimesh colliders, which cost nothing while idle.
- Animation clips are sampled on export and optimized to drop redundant keys.

### Build

- Release: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.
- Only the `3d` and `ui` Bevy feature collections are compiled.
- Dev builds optimize dependencies (`opt-level = 3`) so debug runs are playable.

## Measuring

```sh
cargo run --release --features bevy/trace_tracy   # profile with Tracy
cargo run --features dev                          # FPS overlay in the corner
```

Press F1 in `dev` builds to see the physics colliders.

## Going further when needed

- Lower the render resolution and upscale (render to a smaller texture, or use Bevy's
  upscaling when it fits the art style).
- Use `VisibilityRange` for distance-based LODs on large levels.
- Compress textures to KTX2 (Basis Universal) and keep them at 1024² or less.
- Replace trimesh colliders with convex hulls or primitives for dynamic objects.
- Cap the frame rate on laptops to save battery.
