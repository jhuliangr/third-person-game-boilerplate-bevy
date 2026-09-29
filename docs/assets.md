# Asset pipeline

All 3D content lives in `assets/` as an editable Blender source (`.blend`) next to the
runtime file the game loads (`.glb`). Both are committed.

```
assets/
  models/player/player.blend   source: rig, mesh, animation actions
  models/player/player.glb     runtime
  levels/sandbox.blend         source: level geometry and spawn points
  levels/sandbox.glb           runtime
```

## Editing an asset

1. Open the `.blend` in Blender and edit it.
2. Save it, then export with the project settings:

   ```sh
   blender assets/models/player/player.blend --background --python tools/blender/export_glb.py
   ```

   The `.glb` is written next to the `.blend` with the same name. Exporting manually through
   *File → Export → glTF 2.0* also works if you use: format *glTF Binary*, *+Y Up*, *Apply
   Modifiers*, animation mode *Actions*, *Always Sample Animations*.

3. Run the game again to see the changes.

## Regenerating from scratch

The generators rebuild the placeholder assets and overwrite both files (manual edits in the
`.blend` are lost):

```sh
blender --background --factory-startup --python tools/blender/generate_player.py
blender --background --factory-startup --python tools/blender/generate_sandbox.py
```

## Conventions

- **Units:** meters. **Up:** +Z in Blender (converted to +Y).
- **Facing:** characters face Blender's front view, -Y. The game converts it to Bevy's
  forward, -Z.
- **Origin:** characters have their origin at the feet, between both feet.
- **Scale:** apply scale (`Ctrl+A`) before exporting.

## Characters

- One armature with the mesh parented to it (Armature modifier).
- Each animation is a separate **action**; its name is the clip name used in code
  (`Idle`, `Walk`, `Run`, `CrouchIdle`, `CrouchWalk`). Push each action into its own NLA
  track so the exporter picks it up.
- Clips are **in place** (no root motion). Locomotion cycles loop: the last frame equals the
  first.
- Locomotion cycles start on the left foot contact so the game can keep their phase when
  crossfading between Walk, Run and CrouchWalk.
- If a cycle's stride changes, update its reference speed in `gf_player`
  (`WALK_REFERENCE_SPEED`, ...): the ground speed at which feet don't slide at 1x playback.
  Measure it as stride length × 2 / cycle duration.

### Replacing the placeholder humanoid

Any rigged humanoid works (Mixamo, Quaternius, custom) as long as its action names match
the clip names in `gf_player::clips`, or you update those constants. The bone names don't
matter to the code.

The generated character's rig: `hips → spine → chest → neck → head`, arms
`shoulder → upper_arm → forearm → hand` and legs `thigh → shin → foot`, with `.L`/`.R`
suffixes. Its pose values in `generate_player.py` use the convention "positive local X
rotation pitches a bone forward".

## Levels

- Every mesh becomes a static **trimesh collider**. Keep collision geometry simple and closed.
- An **empty** whose name starts with `PlayerSpawn` marks where the player appears.
- Walkable slopes are up to 45°. Anything steeper blocks movement like a wall.
- The standing player is 1.8 m tall and 0.6 m wide; crouching it is 1.2 m tall. Openings
  meant to be crouched through should be between 1.25 m and 1.75 m high.
- Custom properties on objects are exported as glTF extras and can be read in code to mark
  triggers, pickups or other gameplay objects.

## Performance budget (low-end target)

- Characters: under 5k triangles, 1–3 materials, one skeleton under 60 bones.
- Level: prefer few materials; share them between objects so Bevy can batch draws.
- Textures: 1024² maximum for props, compress to KTX2 when textures are added.
