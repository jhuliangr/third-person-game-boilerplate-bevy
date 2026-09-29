"""Generates the placeholder humanoid: rig, low-poly skinned mesh and locomotion clips.

Usage:
    blender --background --factory-startup --python tools/blender/generate_player.py

Writes assets/models/player/player.blend (the editable source) and player.glb.
Once the .blend exists, prefer editing it in Blender and re-exporting with
export_glb.py; re-running this script overwrites any manual changes.

Conventions:
    Blender units are meters, Z up, the character faces -Y (Blender "front" view).
    Positive local X rotation pitches any bone "forward"; `hips_z` moves the
    hips forward and `hips_y` moves them up.
    Pose values below are written for the left side; the right side is mirrored.
"""

import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.append(str(Path(__file__).parent))
from export_glb import export_glb  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = ROOT / "assets" / "models" / "player"
FPS = 30

FORWARD = Vector((0.0, -1.0, 0.0))
UP = Vector((0.0, 0.0, 1.0))

# name: (head, tail, parent, roll_axis)
BONES = {
    "hips": ((0, 0, 0.95), (0, 0, 1.05), None, FORWARD),
    "spine": ((0, 0, 1.05), (0, 0, 1.28), "hips", FORWARD),
    "chest": ((0, 0, 1.28), (0, 0, 1.50), "spine", FORWARD),
    "neck": ((0, 0, 1.50), (0, 0, 1.58), "chest", FORWARD),
    "head": ((0, 0, 1.58), (0, 0, 1.82), "neck", FORWARD),
    "shoulder.L": ((0.05, 0, 1.45), (0.19, 0, 1.45), "chest", FORWARD),
    "upper_arm.L": ((0.21, 0, 1.45), (0.21, 0, 1.17), "shoulder.L", FORWARD),
    "forearm.L": ((0.21, 0, 1.17), (0.21, 0, 0.93), "upper_arm.L", FORWARD),
    "hand.L": ((0.21, 0, 0.93), (0.21, 0, 0.83), "forearm.L", FORWARD),
    "thigh.L": ((0.10, 0, 0.95), (0.10, 0, 0.52), "hips", FORWARD),
    "shin.L": ((0.10, 0, 0.52), (0.10, 0, 0.10), "thigh.L", FORWARD),
    "foot.L": ((0.10, 0, 0.10), (0.10, -0.14, 0.03), "shin.L", UP),
}

SUIT = (0.16, 0.22, 0.32, 1.0)
TRIM = (0.85, 0.45, 0.12, 1.0)
SKIN = (0.80, 0.66, 0.55, 1.0)


def mirror_name(name: str) -> str:
    if name.endswith(".L"):
        return name[:-2] + ".R"
    if name.endswith(".R"):
        return name[:-2] + ".L"
    return name


def all_bones() -> dict:
    bones = {}
    for name, (head, tail, parent, roll) in BONES.items():
        bones[name] = (Vector(head), Vector(tail), parent, roll)
        if name.endswith(".L"):
            flip = lambda v: Vector((-v[0], v[1], v[2]))  # noqa: E731
            bones[mirror_name(name)] = (
                flip(head),
                flip(tail),
                mirror_name(parent) if parent else None,
                roll,
            )
    return bones


def reset_scene() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = FPS
    scene.unit_settings.system = "METRIC"


def build_armature() -> bpy.types.Object:
    data = bpy.data.armatures.new("PlayerRig")
    rig = bpy.data.objects.new("Player", data)
    bpy.context.scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)

    bpy.ops.object.mode_set(mode="EDIT")
    bones = all_bones()
    for name, (head, tail, _, roll) in bones.items():
        eb = data.edit_bones.new(name)
        eb.head, eb.tail = head, tail
        eb.align_roll(roll)
    for name, (_, _, parent, _) in bones.items():
        if parent:
            eb = data.edit_bones[name]
            eb.parent = data.edit_bones[parent]
            eb.use_connect = (eb.head - eb.parent.tail).length < 1e-4
    bpy.ops.object.mode_set(mode="OBJECT")

    data.display_type = "STICK"
    for pb in rig.pose.bones:
        pb.rotation_mode = "XYZ"
    return rig


class MeshBuilder:
    """Accumulates rigidly skinned primitives into a single bmesh."""

    def __init__(self) -> None:
        self.bm = bmesh.new()
        self.deform = self.bm.verts.layers.deform.verify()
        self.groups: list[str] = []

    def _group(self, bone: str) -> int:
        if bone not in self.groups:
            self.groups.append(bone)
        return self.groups.index(bone)

    def _finish(self, geom_verts, bone: str, material: int, smooth: bool) -> None:
        index = self._group(bone)
        faces = set()
        for v in geom_verts:
            v[self.deform][index] = 1.0
            faces.update(v.link_faces)
        for f in faces:
            f.material_index = material
            f.smooth = smooth

    def segment(self, bone, a, b, r1, r2, material=0, segments=8, squash=1.0):
        a, b = Vector(a), Vector(b)
        axis = b - a
        rot = axis.to_track_quat("Z", "Y").to_matrix().to_4x4()
        mat = Matrix.Translation((a + b) / 2) @ rot @ Matrix.Diagonal((1, squash, 1, 1))
        res = bmesh.ops.create_cone(
            self.bm,
            cap_ends=True,
            segments=segments,
            radius1=r1,
            radius2=r2,
            depth=axis.length,
            matrix=mat,
        )
        self._finish(res["verts"], bone, material, smooth=True)

    def ball(self, bone, center, radius, material=1, scale=(1, 1, 1)):
        mat = Matrix.Translation(center) @ Matrix.Diagonal((*scale, 1))
        res = bmesh.ops.create_uvsphere(
            self.bm, u_segments=10, v_segments=6, radius=radius, matrix=mat
        )
        self._finish(res["verts"], bone, material, smooth=True)

    def box(self, bone, center, size, material=0):
        mat = Matrix.Translation(center) @ Matrix.Diagonal((*size, 1))
        res = bmesh.ops.create_cube(self.bm, size=1.0, matrix=mat)
        self._finish(res["verts"], bone, material, smooth=False)


def make_material(name: str, color, roughness=0.8) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = roughness
    mat.diffuse_color = color
    return mat


def build_body(rig: bpy.types.Object) -> bpy.types.Object:
    mb = MeshBuilder()

    mb.box("hips", (0, 0, 0.98), (0.32, 0.19, 0.16))
    mb.segment("spine", (0, 0, 1.04), (0, 0, 1.30), 0.13, 0.15, squash=0.72)
    mb.segment("chest", (0, 0, 1.27), (0, 0, 1.51), 0.17, 0.19, squash=0.66)
    mb.box("chest", (0, -0.1, 1.40), (0.16, 0.04, 0.08), material=1)
    mb.segment("neck", (0, 0, 1.49), (0, 0, 1.61), 0.05, 0.045, material=2)
    mb.ball("head", (0, 0, 1.70), 0.115, material=2, scale=(0.95, 1.0, 1.12))
    mb.box("head", (0, -0.095, 1.72), (0.17, 0.05, 0.05), material=1)

    for side in (1, -1):
        s = "L" if side == 1 else "R"
        x = 0.21 * side
        mb.ball(f"shoulder.{s}", (0.17 * side, 0, 1.45), 0.07)
        mb.segment(f"upper_arm.{s}", (x, 0, 1.44), (x, 0, 1.18), 0.055, 0.045)
        mb.ball(f"forearm.{s}", (x, 0, 1.17), 0.045)
        mb.segment(f"forearm.{s}", (x, 0, 1.16), (x, 0, 0.94), 0.045, 0.038)
        mb.ball(f"hand.{s}", (x, -0.005, 0.88), 0.045, material=2, scale=(0.7, 1.0, 1.3))

        lx = 0.10 * side
        mb.segment(f"thigh.{s}", (lx, 0, 0.93), (lx, 0, 0.53), 0.085, 0.065)
        mb.ball(f"shin.{s}", (lx, 0, 0.52), 0.06)
        mb.segment(f"shin.{s}", (lx, 0, 0.51), (lx, 0, 0.11), 0.06, 0.045)
        mb.box(f"foot.{s}", (lx, -0.04, 0.045), (0.095, 0.24, 0.09), material=1)

    mesh = bpy.data.meshes.new("PlayerBody")
    mb.bm.to_mesh(mesh)
    mb.bm.free()

    body = bpy.data.objects.new("PlayerBody", mesh)
    bpy.context.scene.collection.objects.link(body)
    for name in mb.groups:
        body.vertex_groups.new(name=name)
    for mat in (
        make_material("Suit", SUIT),
        make_material("Trim", TRIM, 0.5),
        make_material("Skin", SKIN, 0.9),
    ):
        mesh.materials.append(mat)

    body.parent = rig
    modifier = body.modifiers.new("Armature", "ARMATURE")
    modifier.object = rig
    return body


def pose(**bones) -> dict:
    """Builds a pose from keyword args; `hips_y`/`hips_z` offset the hips (up/forward)."""
    base = {
        "upper_arm.L": (0, 0, 6),
        "upper_arm.R": (0, 0, 6),
        "forearm.L": (8, 0, 0),
        "forearm.R": (8, 0, 0),
    }
    for key, value in bones.items():
        base[key.replace("_L", ".L").replace("_R", ".R")] = value
    return base


def mirrored(p: dict) -> dict:
    out = {}
    for key, value in p.items():
        if isinstance(value, tuple) and mirror_name(key) == key:
            value = (value[0], -value[1], -value[2])
        out[mirror_name(key)] = value
    return out


def key_pose(rig: bpy.types.Object, frame: int, p: dict) -> None:
    for pb in rig.pose.bones:
        x, y, z = p.get(pb.name, (0, 0, 0))
        if pb.name.endswith(".R"):
            y, z = -y, -z
        pb.rotation_euler = (math.radians(x), math.radians(y), math.radians(z))
        pb.keyframe_insert("rotation_euler", frame=frame)

    hips = rig.pose.bones["hips"]
    hips.location = (0.0, p.get("hips_y", 0.0), p.get("hips_z", 0.0))
    hips.keyframe_insert("location", frame=frame)


def make_clip(rig: bpy.types.Object, name: str, keys: list) -> None:
    """keys: list of (frame, pose). The last frame should repeat the first for loops."""
    action = bpy.data.actions.new(name)
    action.use_fake_user = True
    rig.animation_data_create()
    rig.animation_data.action = action

    for frame, p in keys:
        key_pose(rig, frame, p)

    track = rig.animation_data.nla_tracks.new()
    track.name = name
    track.mute = True
    track.strips.new(name, int(keys[0][0]), action)
    rig.animation_data.action = None


def cycle(half_a: dict, pass_a: dict, length: int) -> list:
    """Four-key locomotion loop: contact, passing, mirrored contact, mirrored passing."""
    q = length // 4
    return [
        (0, half_a),
        (q, pass_a),
        (length // 2, mirrored(half_a)),
        (length // 2 + q, mirrored(pass_a)),
        (length, half_a),
    ]


def build_clips(rig: bpy.types.Object) -> None:
    idle_a = pose(chest=(0, 0, 0), hips_y=0.0)
    idle_b = pose(
        chest=(2.5, 0, 0),
        neck=(-1.5, 0, 0),
        upper_arm_L=(1, 0, 8),
        upper_arm_R=(1, 0, 8),
        hips_y=-0.012,
    )
    make_clip(rig, "Idle", [(0, idle_a), (30, idle_b), (60, idle_a)])

    walk_contact = pose(
        hips=(0, 4, 0),
        spine=(3, -3, 0),
        chest=(0, -4, 0),
        thigh_L=(30, 0, 0),
        shin_L=(-5, 0, 0),
        foot_L=(-35, 0, 0),
        thigh_R=(-25, 0, 0),
        shin_R=(-8, 0, 0),
        foot_R=(45, 0, 0),
        upper_arm_L=(-22, 0, 6),
        forearm_L=(12, 0, 0),
        upper_arm_R=(24, 0, 6),
        forearm_R=(28, 0, 0),
        hips_y=-0.09,
    )
    walk_pass = pose(
        spine=(3, 0, 0),
        thigh_L=(0, 0, 0),
        shin_L=(-2, 0, 0),
        foot_L=(2, 0, 0),
        thigh_R=(22, 0, 0),
        shin_R=(-55, 0, 0),
        foot_R=(40, 0, 0),
        upper_arm_L=(0, 0, 6),
        upper_arm_R=(2, 0, 6),
        forearm_L=(12, 0, 0),
        forearm_R=(14, 0, 0),
        hips_y=0.0,
    )
    make_clip(rig, "Walk", cycle(walk_contact, walk_pass, 28))

    run_contact = pose(
        hips=(0, 8, 0),
        spine=(10, -5, 0),
        chest=(4, -8, 0),
        neck=(-6, 0, 0),
        head=(-6, 4, 0),
        thigh_L=(42, 0, 0),
        shin_L=(-18, 0, 0),
        foot_L=(-34, 0, 0),
        thigh_R=(-32, 0, 0),
        shin_R=(-45, 0, 0),
        foot_R=(90, 0, 0),
        upper_arm_L=(-40, 0, 10),
        forearm_L=(75, 0, 0),
        upper_arm_R=(45, 0, 10),
        forearm_R=(85, 0, 0),
        hips_y=-0.14,
    )
    run_pass = pose(
        spine=(10, 0, 0),
        chest=(4, 0, 0),
        neck=(-6, 0, 0),
        head=(-6, 0, 0),
        thigh_L=(-8, 0, 0),
        shin_L=(-20, 0, 0),
        foot_L=(20, 0, 0),
        thigh_R=(35, 0, 0),
        shin_R=(-105, 0, 0),
        foot_R=(60, 0, 0),
        upper_arm_L=(0, 0, 10),
        forearm_L=(80, 0, 0),
        upper_arm_R=(5, 0, 10),
        forearm_R=(80, 0, 0),
        hips_y=-0.02,
    )
    make_clip(rig, "Run", cycle(run_contact, run_pass, 20))

    crouch_base = dict(
        spine=(22, 0, 0),
        chest=(12, 0, 0),
        neck=(-12, 0, 0),
        head=(-10, 0, 0),
        upper_arm_L=(28, 0, 10),
        upper_arm_R=(28, 0, 10),
        forearm_L=(40, 0, 0),
        forearm_R=(40, 0, 0),
    )
    crouch_a = pose(
        **crouch_base,
        thigh_L=(78, 0, 4),
        shin_L=(-108, 0, 0),
        foot_L=(30, 0, 0),
        thigh_R=(76, 0, -2),
        shin_R=(-106, 0, 0),
        foot_R=(30, 0, 0),
        hips_y=-0.395,
        hips_z=-0.04,
    )
    crouch_b = pose(
        **{**crouch_base, "chest": (14, 0, 0), "spine": (23, 0, 0)},
        thigh_L=(79, 0, 4),
        shin_L=(-110, 0, 0),
        foot_L=(31, 0, 0),
        thigh_R=(77, 0, -2),
        shin_R=(-108, 0, 0),
        foot_R=(31, 0, 0),
        hips_y=-0.405,
        hips_z=-0.04,
    )
    make_clip(rig, "CrouchIdle", [(0, crouch_a), (40, crouch_b), (80, crouch_a)])

    crouch_contact = pose(
        **{**crouch_base, "spine": (22, -4, 0), "upper_arm_L": (18, 0, 10), "upper_arm_R": (38, 0, 10)},
        thigh_L=(84, 0, 0),
        shin_L=(-96, 0, 0),
        foot_L=(12, 0, 0),
        thigh_R=(30, 0, 0),
        shin_R=(-110, 0, 0),
        foot_R=(95, 0, 0),
        hips_y=-0.40,
        hips_z=-0.02,
    )
    crouch_pass = pose(
        **crouch_base,
        thigh_L=(70, 0, 0),
        shin_L=(-100, 0, 0),
        foot_L=(30, 0, 0),
        thigh_R=(82, 0, 0),
        shin_R=(-128, 0, 0),
        foot_R=(46, 0, 0),
        hips_y=-0.34,
        hips_z=-0.02,
    )
    make_clip(rig, "CrouchWalk", cycle(crouch_contact, crouch_pass, 36))


def main() -> None:
    reset_scene()
    rig = build_armature()
    build_body(rig)
    build_clips(rig)

    for pb in rig.pose.bones:
        pb.rotation_euler = (0, 0, 0)
        pb.location = (0, 0, 0)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT_DIR / "player.blend"))
    export_glb(OUT_DIR / "player.glb")


if __name__ == "__main__":
    main()
