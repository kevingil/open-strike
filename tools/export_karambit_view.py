"""Author the karambit's right-hand reverse grip and ring-flip draw (Blender 4.2).

Uses the downloaded catalog mesh and DJMaesen's weighted reference arms. Outputs
only dedicated first-person variants; world geometry and source rigs are inputs.
"""

import math
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_assets import clean, export, import_rig  # noqa: E402
from export_reference_knife import compatible  # noqa: E402
from export_viewmodels import REFERENCE, style  # noqa: E402

OUT = ROOT / "assets/generated"
WRIST = "R_wrist_027"
# Authored from the catalog mesh's side profile, in its local metre coordinates.
RING_CENTER = Vector((0, -0.120, 0.020))
# Seat the near rim ahead of the descending finger, leaving the hole visible
# on its outside. These are native centimetres and a fraction of the phalanx.
RING_FINGER_FRACTION = 0.40
RING_OUTSIDE_OFFSET = 0.70
# Preserve the catalog proportions while fitting the complete guard outside the
# fist. The former 75% fit buried the guard and crowded the ring with the glove.
KNIFE_VIEW_SCALE = 95
CLIPS = {"idle_knife": 40, "draw_knife": 21, "slash_knife": 33}


def key_transform(target, frame):
    target.rotation_mode = "QUATERNION"
    for channel in ("location", "rotation_quaternion", "scale"):
        target.keyframe_insert(channel, frame=frame)


def smooth(t):
    return t * t * (3 - 2 * t)


def aim_joint(rig, name, child_name, direction):
    """Pose a phalanx using its real child joint, not the importer display tail."""
    bone = rig.pose.bones[name]
    child = rig.pose.bones[child_name]
    rotation = (child.head - bone.head).rotation_difference(Vector(direction))
    bone.matrix = Matrix.LocRotScale(
        bone.head, rotation @ bone.matrix.to_quaternion(), bone.matrix.to_scale()
    )
    bpy.context.view_layer.update()


def frame_arm(rig, upper_name, elbow_name, wrist_name, offset):
    """Two-bone reach to the authored screen position, preserving wrist rotation."""
    upper, elbow, wrist = (
        rig.pose.bones[n] for n in (upper_name, elbow_name, wrist_name)
    )
    shoulder = upper.head.copy()
    target = wrist.head + Vector(offset)
    wrist_rotation = wrist.matrix.to_quaternion()
    first_length = (elbow.head - shoulder).length
    second_length = (wrist.head - elbow.head).length
    axis = (target - shoulder).normalized()
    distance = (target - shoulder).length
    if not abs(first_length - second_length) < distance < first_length + second_length:
        raise ValueError("Karambit wrist target is outside the arm's reach")
    along = (first_length**2 - second_length**2 + distance**2) / (2 * distance)
    pole = elbow.head - shoulder
    pole = (pole - axis * pole.dot(axis)).normalized()
    bend = shoulder + axis * along + pole * math.sqrt(first_length**2 - along**2)
    aim_joint(rig, upper_name, elbow_name, bend - shoulder)
    aim_joint(rig, elbow_name, wrist_name, target - elbow.head)
    wrist.matrix = Matrix.LocRotScale(
        wrist.head, wrist_rotation, wrist.matrix.to_scale()
    )
    bpy.context.view_layer.update()


def fit_hand(rig):
    """Closed reverse grip: exposed index loop, thumb underneath, blade by pinky."""
    # Keep the complete blade inside the right edge and bring the support hand
    # inward, matching the reference's low, relaxed guard at the game camera FOV.
    frame_arm(rig, "R_arm_025", "R_elbow_026", WRIST, (8, -4, -4))
    frame_arm(rig, "L_arm_02", "L_elbow_03", "L_wrist_04", (-4, -4, 0))
    wrist = rig.pose.bones[WRIST]
    pivot = wrist.head.copy()
    axis = (pivot - rig.pose.bones["R_elbow_026"].head).normalized()
    wrist.matrix = (
        Matrix.Translation(pivot)
        @ Matrix.Rotation(math.radians(-15), 4, axis)
        @ Matrix.Translation(-pivot)
        @ wrist.matrix
    )
    bpy.context.view_layer.update()
    # Match the fist-to-blade proportion, then turn its back toward the camera.
    # The ring is fitted afterward in world space so it keeps its catalog size.
    wrist.matrix = wrist.matrix @ Matrix.Scale(0.88, 4)
    bpy.context.view_layer.update()
    # Tuck the proximal index toward the palm, exposing only the descending
    # segment behind the ring rather than a broad side face above the handle.
    index_root = rig.pose.bones["R_point1_032"]
    index_root.matrix = Matrix.Translation((0.5, 0, 0)) @ index_root.matrix
    bpy.context.view_layer.update()
    for name, child, direction in (
        ("R_point1_032", "R_point2_033", (0.05, -0.90, -0.43)),
        ("R_point2_033", "R_point3_034", (0, -0.28, -0.96)),
        ("R_point3_034", "Joint_3_7_035", (-0.25, 0.95, -0.1)),
        # Ease the thumb slightly outward below the grip, retaining the inward
        # fingertip curl and clearance beneath the ring's near inner edge.
        ("R_thumb1_028", "R_thumb2_029", (0.76, -0.50, -0.22)),
        ("R_thumb2_029", "R_thumb3_030", (0.20, -0.87, 0.25)),
        ("R_thumb3_030", "Joint_3_6_031", (-0.8, -0.35, 0.15)),
    ):
        aim_joint(rig, name, child, direction)

    index = rig.matrix_world @ rig.pose.bones["R_point2_033"].head
    tip = rig.matrix_world @ rig.pose.bones["R_point3_034"].head
    normal = (tip - index).normalized()
    blade_axis = Vector((-1, 0, 0))
    blade_axis = (blade_axis - normal * blade_axis.dot(normal)).normalized()
    face_up = normal.cross(blade_axis)
    basis = Matrix((normal, blade_axis, face_up)).transposed().to_4x4()
    # Roll the knife onto the forward side of the finger: its curved tip points
    # primarily into the scene (-Y), not up (+Z). The ring's near rim crosses in
    # front of the finger, with the distal curl below it.
    return (
        Matrix.Translation(
            index.lerp(tip, RING_FINGER_FRACTION) + Vector((RING_OUTSIDE_OFFSET, 0, 0))
        )
        @ basis
        @ Matrix.Rotation(math.radians(8), 4, "X")
    )


def open_fingers(rig, base):
    """A relaxed catch pose; index and thumb keep their ring contact."""
    for first, second, third, end in (
        ("R_middle1_036", "R_middle2_037", "R_middle3_038", "Joint_3_8_039"),
        ("R_ring1_041", "R_ring2_042", "R_ring3_043", "Joint_3_9_044"),
        ("R_pink1_045", "R_pink2_046", "R_pink3_047", "Joint_3_10_048"),
    ):
        direction = rig.pose.bones[second].head - rig.pose.bones[first].head
        aim_joint(rig, second, third, direction)
        aim_joint(rig, third, end, direction)
    opened = {b.name: b.matrix_basis.to_quaternion() for b in rig.pose.bones}
    for bone in rig.pose.bones:
        bone.matrix_basis = base[bone.name]
    bpy.context.view_layer.update()
    return opened


def build():
    clean()
    rig, imported = import_rig(REFERENCE)
    rig.animation_data.action = rig.animation_data.nla_tracks[0].strips[0].action
    bpy.context.scene.frame_set(0)
    bpy.context.view_layer.update()
    arms = next(o for o in imported if o.name == "Object_7")
    world = rig.matrix_world.copy()
    rig.parent = None
    rig.matrix_world = world
    for obj in imported - {rig, arms}:
        bpy.data.objects.remove(obj, do_unlink=True)
    rig.animation_data_clear()
    arms.animation_data_clear()
    rig.name = "KarambitArms"
    ring_pose = fit_hand(rig)
    base = {b.name: b.matrix_basis.copy() for b in rig.pose.bones}
    opened = open_fingers(rig, base)

    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(OUT / "weapons/karambit.glb"))
    imported = set(bpy.data.objects) - before
    blade = next(o for o in imported if o.type == "MESH")
    blade.parent = None
    blade.matrix_world = Matrix.Identity(4)
    blade.name = "KarambitGeometry"
    for obj in imported - {blade}:
        bpy.data.objects.remove(obj, do_unlink=True)
    # Centre before scaling: the finger-ring pivot stays fixed while the full
    # guard and blade extend beyond the little-finger side of the hand.
    blade.data.transform(
        Matrix.Scale(KNIFE_VIEW_SCALE, 4) @ Matrix.Translation(-RING_CENTER)
    )
    grip = bpy.data.objects.new("KarambitRing", None)
    bpy.context.collection.objects.link(grip)
    grip.parent = rig
    grip.parent_type = "BONE"
    grip.parent_bone = WRIST
    bpy.context.view_layer.update()
    grip.matrix_world = ring_pose
    bpy.context.view_layer.update()
    blade.parent = grip
    blade.matrix_basis = Matrix.Identity(4)
    grip_base = grip.matrix_basis.copy()

    actions = {}
    for name, end in CLIPS.items():
        rig_action = bpy.data.actions.new(name + "_arms")
        ring_action = bpy.data.actions.new(name + "_ring")
        rig.animation_data_create().action = rig_action
        grip.animation_data_create().action = ring_action
        for frame in range(end + 1):
            bpy.context.scene.frame_set(frame)
            t = frame / end
            for bone in rig.pose.bones:
                bone.matrix_basis = base[bone.name]
            bpy.context.view_layer.update()
            # Author in native centimetres. Both hands enter from below; the
            # right wrist rolls through the reveal before settling into idle.
            draw = 1 - smooth(min(t / 0.38, 1)) if name == "draw_knife" else 0
            flip = (
                smooth(min(max((t - 0.24) / 0.58, 0), 1)) if name == "draw_knife" else 1
            )
            reveal = math.sin(math.pi * flip) if name == "draw_knife" else 0
            impact = 0.15 / 0.55
            stroke = (
                (
                    smooth(t / impact)
                    if t <= impact
                    else 1 - smooth((t - impact) / (1 - impact))
                )
                if name == "slash_knife"
                else 0
            )
            breath = math.sin(t * math.tau) * 0.25 if name == "idle_knife" else 0
            root = rig.pose.bones["root_01"]
            root.matrix = (
                Matrix.Translation((0, 0, breath - 24 * draw + 20 * reveal))
                @ root.matrix
            )
            bpy.context.view_layer.update()
            wrist = rig.pose.bones[WRIST]
            pose = wrist.matrix.copy()
            pivot = pose.translation.copy()
            roll = (
                math.radians(30) * math.sin(math.pi * t) if name == "draw_knife" else 0
            )
            wrist.matrix = (
                Matrix.Translation((10 * stroke, -6 * stroke, 3 * stroke))
                @ Matrix.Translation(pivot)
                @ Matrix.Rotation(roll + math.radians(25) * stroke, 4, "Y")
                @ Matrix.Translation(-pivot)
                @ pose
            )
            # Ring-centred full turn, completed before the final catch. The
            # index remains in the ring while the other fingers open then close.
            grip.matrix_basis = grip_base @ Matrix.Rotation(
                math.tau * (flip - 1), 4, "X"
            )
            if name == "draw_knife":
                opening = math.sin(math.pi * flip)
                for bone_name in (
                    "R_middle2_037",
                    "R_middle3_038",
                    "R_ring2_042",
                    "R_ring3_043",
                    "R_pink2_046",
                    "R_pink3_047",
                ):
                    bone = rig.pose.bones[bone_name]
                    bone.rotation_mode = "QUATERNION"
                    bone.rotation_quaternion = (
                        base[bone_name]
                        .to_quaternion()
                        .slerp(opened[bone_name], 0.8 * opening)
                    )
            for bone in rig.pose.bones:
                key_transform(bone, frame)
            key_transform(grip, frame)
        actions[name] = (rig_action, ring_action)
    for i, obj in enumerate((rig, grip)):
        obj.animation_data.action = None
        for name, pair in actions.items():
            action = pair[i]
            for curve in action.fcurves:
                for point in curve.keyframe_points:
                    point.interpolation = "LINEAR"
            track = obj.animation_data.nla_tracks.new()
            track.name = name
            track.strips.new(name, 0, action)
    bpy.context.scene.frame_set(0)
    return rig, arms, grip, blade


def export_views():
    objects = build()
    for character in ("soldier", "police"):
        style(objects[1], character)
        path = OUT / f"karambit_view_{character}.glb"
        export(path, objects)
        compatible(path)


if __name__ == "__main__":
    export_views()
