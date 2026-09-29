"""Exports the currently open .blend to a .glb with the project's standard settings.

Usage:
    blender assets/models/player/player.blend --background --python tools/blender/export_glb.py

The .glb is written next to the .blend with the same base name.
"""

from pathlib import Path

import bpy


def export_glb(path: Path) -> None:
    for obj in bpy.data.objects:
        if obj.animation_data:
            obj.animation_data.action = None

    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        export_extras=True,
        export_skins=True,
        export_def_bones=True,
        export_morph=False,
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_force_sampling=True,
        export_optimize_animation_size=True,
    )
    print(f"Exported {path}")


if __name__ == "__main__":
    blend = Path(bpy.data.filepath)
    if not blend.name:
        raise SystemExit("Open a saved .blend file before exporting.")
    export_glb(blend.with_suffix(".glb"))
