"""Generates the sandbox test level.

Usage:
    blender --background --factory-startup --python tools/blender/generate_sandbox.py

Writes assets/levels/sandbox.blend and sandbox.glb. Every mesh becomes a static
collider in game. Empties named `PlayerSpawn` mark where the player appears.
"""

import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

sys.path.append(str(Path(__file__).parent))
from export_glb import export_glb  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = ROOT / "assets" / "levels"

GROUND_SIZE = 60
TILE = 2


def make_material(name: str, color) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = 0.9
    mat.diffuse_color = (*color, 1.0)
    return mat


def add_object(name: str, bm: bmesh.types.BMesh, location, materials) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    for mat in materials:
        mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    obj.location = location
    bpy.context.scene.collection.objects.link(obj)
    return obj


def box(name, center, size, material):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=Vector(size), verts=bm.verts)
    return add_object(name, bm, center, [material])


def ramp(name, origin, width, length, height, material):
    """Wedge rising along +Y from `origin` (low edge center at ground level)."""
    bm = bmesh.new()
    w = width / 2
    v = [
        bm.verts.new(p)
        for p in (
            (-w, 0, 0),
            (w, 0, 0),
            (w, length, 0),
            (-w, length, 0),
            (-w, length, height),
            (w, length, height),
        )
    ]
    for face in (
        (v[0], v[3], v[2], v[1]),
        (v[0], v[1], v[5], v[4]),
        (v[0], v[4], v[3]),
        (v[1], v[2], v[5]),
        (v[2], v[3], v[4], v[5]),
    ):
        bm.faces.new(face)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return add_object(name, bm, origin, [material])


def ground(light, dark):
    bm = bmesh.new()
    half = GROUND_SIZE / 2
    tiles = GROUND_SIZE // TILE
    for ix in range(tiles):
        for iy in range(tiles):
            x0, y0 = -half + ix * TILE, -half + iy * TILE
            verts = [
                bm.verts.new((x0, y0, 0)),
                bm.verts.new((x0 + TILE, y0, 0)),
                bm.verts.new((x0 + TILE, y0 + TILE, 0)),
                bm.verts.new((x0, y0 + TILE, 0)),
            ]
            face = bm.faces.new(verts)
            face.material_index = (ix + iy) % 2
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
    return add_object("Ground", bm, (0, 0, 0), [light, dark])


def main() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)

    light = make_material("GroundLight", (0.55, 0.57, 0.6))
    dark = make_material("GroundDark", (0.42, 0.44, 0.47))
    block = make_material("Block", (0.62, 0.6, 0.56))
    accent = make_material("Accent", (0.85, 0.45, 0.12))

    ground(light, dark)

    box("CrateSmall", (4, -4, 0.25), (0.5, 0.5, 0.5), accent)
    box("CrateMedium", (5.5, -4.5, 0.5), (1, 1, 1), block)
    box("CrateLarge", (7.5, -3, 1), (2, 2, 2), block)

    ramp("RampGentle", (-6, 2, 0), 3, 8, 1.4, block)
    box("RampGentleTop", (-6, 11.5, 0.7), (3, 3, 1.4), block)
    ramp("RampSteep", (-11, 2, 0), 3, 4, 2.0, accent)

    for i, (x, y) in enumerate(((10, 8), (14, 8), (10, 12), (14, 12))):
        box(f"Pillar{i}", (x, y, 2), (0.8, 0.8, 4), block)

    box("Wall", (0, 16, 1.5), (12, 0.5, 3), block)

    tunnel_x, tunnel_y = 0, -10
    box("TunnelLeft", (tunnel_x - 1.25, tunnel_y, 0.725), (0.5, 4, 1.45), block)
    box("TunnelRight", (tunnel_x + 1.25, tunnel_y, 0.725), (0.5, 4, 1.45), block)
    box("TunnelRoof", (tunnel_x, tunnel_y, 1.65), (3, 4, 0.4), accent)

    spawn = bpy.data.objects.new("PlayerSpawn", None)
    spawn.empty_display_type = "ARROWS"
    spawn.location = (0, 0, 0.1)
    bpy.context.scene.collection.objects.link(spawn)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT_DIR / "sandbox.blend"))
    export_glb(OUT_DIR / "sandbox.glb")


if __name__ == "__main__":
    main()
