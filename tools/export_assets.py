"""Bake explicit source actions into each skin's own rest pose. Run with Blender 4.2.

blender --background --factory-startup --python tools/export_assets.py -- characters
Original GLBs/blends are never overwritten. All outputs live in assets/generated.
"""

import json
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import] # Provided by Blender 4.2
from mathutils import Quaternion, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/generated"
# Chosen after inspecting source poses, not inferred from clip index/duration.
CLIPS = {
    "idle_rifle": "anim_003_Armature",
    "fire_rifle": "anim_0_Armature",
    "reload_rifle": "anim_002_Armature",
    "run_forward": "anim_006_Armature",
    "strafe_left": "anim_007_Armature",
    "strafe_right": "anim_008_Armature",
    "walk_forward": "anim_014_Armature",
    "walk_backward": "anim_015_Armature",
    "jump": "anim_004_Armature",
}


def clean():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30


def import_rig(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    objects = set(bpy.data.objects) - before
    rig = next(o for o in objects if o.type == "ARMATURE")
    for track in rig.animation_data.nla_tracks:
        track.mute = True
    return rig, objects


def export(path, objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_nla_strips=True,
        export_animation_mode="NLA_TRACKS",
        export_force_sampling=True,
        export_frame_range=False,
        export_optimize_animation_keep_anim_object=True,
        export_anim_slide_to_zero=True,
    )
    if path.name in {"attacker.glb", "defender.glb", "ak_view.glb"}:
        sys.path.insert(0, str(ROOT / "tools"))
        from inspect_assets import apply_character_materials

        apply_character_materials(path)


def align_support_hand(rig, action):
    """Bake an analytic two-bone reach to the declared rifle foregrip."""
    from mathutils import Matrix  # ty: ignore[unresolved-import]
    from math import sqrt

    rig.animation_data.action = action
    names = ["mixamorig:LeftArm", "mixamorig:LeftForeArm", "mixamorig:LeftHand"]
    samples = []
    for frame in range(round(action.frame_range[1]) + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        upper, lower, hand = [rig.pose.bones[name] for name in names]
        shoulder, elbow, wrist = upper.head.copy(), lower.head.copy(), hand.head.copy()
        hand_rotation = hand.matrix.to_quaternion()
        target_world = rig.matrix_world @ rig.pose.bones[
            "mixamorig:RightHand"
        ].head + Vector((0, -0.32, 0.02))
        target = rig.matrix_world.inverted() @ target_world
        length1, length2 = (elbow - shoulder).length, (wrist - elbow).length
        axis = (target - shoulder).normalized()
        distance = min(
            max((target - shoulder).length, abs(length1 - length2) + 0.001),
            length1 + length2 - 0.001,
        )
        along = (length1 * length1 - length2 * length2 + distance * distance) / (
            2 * distance
        )
        bend = elbow - shoulder - axis * (elbow - shoulder).dot(axis)
        if bend.length < 0.001:
            bend = axis.cross(Vector((0, 0, 1)))
        bend.normalize()
        goal_elbow = (
            shoulder
            + axis * along
            + bend * sqrt(max(0, length1 * length1 - along * along))
        )
        rotation = (elbow - shoulder).rotation_difference(
            goal_elbow - shoulder
        ) @ upper.matrix.to_quaternion()
        upper.matrix = Matrix.LocRotScale(shoulder, rotation, upper.matrix.to_scale())
        bpy.context.view_layer.update()
        elbow, wrist = lower.head.copy(), hand.head.copy()
        rotation = (wrist - elbow).rotation_difference(
            target - elbow
        ) @ lower.matrix.to_quaternion()
        lower.matrix = Matrix.LocRotScale(elbow, rotation, lower.matrix.to_scale())
        bpy.context.view_layer.update()
        hand.matrix = Matrix.LocRotScale(
            hand.head, hand_rotation, hand.matrix.to_scale()
        )
        bpy.context.view_layer.update()
        samples.append(
            {name: rig.pose.bones[name].rotation_quaternion.copy() for name in names}
        )
    for curve in list(action.fcurves):
        if any(
            curve.data_path == f'pose.bones["{name}"].rotation_quaternion'
            for name in names
        ):
            action.fcurves.remove(curve)
    for frame, pose in enumerate(samples):
        for name, rotation in pose.items():
            rig.pose.bones[name].rotation_quaternion = rotation
            rig.pose.bones[name].keyframe_insert("rotation_quaternion", frame=frame)


def ground_action(rig, objects, action):
    """Keep the evaluated soles on the floor while the pelvis bends or falls."""
    rig.animation_data.action = action
    hips = rig.pose.bones["mixamorig:Hips"]
    objects = [
        obj
        for obj in objects
        if obj.type == "MESH"
        and any(
            modifier.type == "ARMATURE" and modifier.object == rig
            for modifier in obj.modifiers
        )
    ]
    assert objects, "No skinned character meshes to ground"
    locations = []
    for frame in range(round(action.frame_range[1]) + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        minimum = float("inf")
        for obj in objects:
            if obj.type != "MESH":
                continue
            evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
            mesh = evaluated.to_mesh()
            minimum = min(
                minimum, min((evaluated.matrix_world @ v.co).z for v in mesh.vertices)
            )
            evaluated.to_mesh_clear()
        location = hips.location.copy()
        location.y += (0.01 - minimum) * 100.0
        locations.append(location)
    for curve in list(action.fcurves):
        if curve.data_path == 'pose.bones["mixamorig:Hips"].location':
            action.fcurves.remove(curve)
    for frame, location in enumerate(locations):
        hips.location = location
        hips.keyframe_insert("location", frame=frame)


def author_menu_hold(rig, idle):
    """Bake a planted, chest-high diagonal carry with both palms locked to the rifle.

    MenuWeaponSocket compensates for the world rifle's wrist-based origin.
    Contacts use the inspected AK geometry, in metres, before carry rotation.
    """
    from math import sin, tau, sqrt, pi
    from mathutils import Matrix  # ty: ignore[unresolved-import]
    from inspect_assets import read_glb

    rig.animation_data.action = idle
    bpy.context.scene.frame_set(0)
    bpy.context.view_layer.update()
    finger_basis = {
        b.name: b.matrix_basis.copy()
        for b in rig.pose.bones
        if any(part in b.name for part in ["Thumb", "Index", "Middle", "Ring", "Pinky"])
    }
    hands = {
        side: rig.matrix_world @ rig.pose.bones[f"mixamorig:{side}Hand"].matrix
        for side in ["Right", "Left"]
    }
    palm_offsets = {}
    for side, hand in hands.items():
        palm = (
            sum(
                (
                    rig.matrix_world
                    @ rig.pose.bones[f"mixamorig:{side}HandMiddle{i}"].head
                    for i in [1, 2, 3]
                ),
                Vector(),
            )
            / 3
        )
        palm_offsets[side] = hand.to_quaternion().inverted() @ (palm - hand.translation)

    # Use the same socket rotation as the runtime world-weapon scene.
    world, _ = read_glb(OUT / "ak_world.glb")
    socket = next(n for n in world["nodes"] if n.get("name") == "WeaponGrip")
    x, y, z, w = socket["rotation"]
    basis = Quaternion((1, 0, 0), pi / 2)
    # Bone axes are retained by the glTF importer; mesh object axes get the
    # Y-up/Z-up conversion. Only the mesh side needs that conversion here.
    socket_rotation = Quaternion((w, x, y, z)) @ basis.inverted()
    right_rotation = socket_rotation.inverted()
    # Turn the palm partly across the handguard to align it with the forearm.
    left_rotation = Quaternion((0, 0, 1), -0.65) @ hands["Left"].to_quaternion()
    # Seat the curled fingers around the upper grip, below the trigger guard.
    grip_center = Vector((0.0, 0.025, 0.006))
    # Keep the rifle's authored trajectory independent of wrist placement.
    carry_reference = Vector((0.0, 0.03866998, -0.03627182))
    support = Vector((-0.025, -0.265, 0.035))
    socket_offset = palm_offsets["Right"] - right_rotation.inverted() @ grip_center
    carry_offset = palm_offsets["Right"] - right_rotation.inverted() @ carry_reference

    for bone in rig.pose.bones:
        bone.matrix_basis = finger_basis.get(bone.name, Matrix.Identity(4))
    bpy.context.view_layer.update()
    feet = {
        side: rig.matrix_world @ rig.pose.bones[f"mixamorig:{side}Foot"].matrix
        for side in ["Right", "Left"]
    }
    action = bpy.data.actions.new("menu_hold_rifle")
    rig.animation_data.action = action

    def reach(names, target_world, rotation_world, pole_world):
        upper, lower, end = [rig.pose.bones[name] for name in names]
        shoulder, elbow, wrist = upper.head.copy(), lower.head.copy(), end.head.copy()
        target = rig.matrix_world.inverted() @ target_world
        a, b = (elbow - shoulder).length, (wrist - elbow).length
        axis = (target - shoulder).normalized()
        distance = min(
            max((target - shoulder).length, abs(a - b) + 0.001), a + b - 0.001
        )
        along = (a * a - b * b + distance * distance) / (2 * distance)
        pole = rig.matrix_world.inverted().to_3x3() @ pole_world
        bend = (pole - axis * pole.dot(axis)).normalized()
        goal = shoulder + axis * along + bend * sqrt(max(0, a * a - along * along))
        rotation = (elbow - shoulder).rotation_difference(
            goal - shoulder
        ) @ upper.matrix.to_quaternion()
        upper.matrix = Matrix.LocRotScale(shoulder, rotation, upper.matrix.to_scale())
        bpy.context.view_layer.update()
        rotation = (end.head - lower.head).rotation_difference(
            target - lower.head
        ) @ lower.matrix.to_quaternion()
        lower.matrix = Matrix.LocRotScale(lower.head, rotation, lower.matrix.to_scale())
        bpy.context.view_layer.update()
        rotation = rig.matrix_world.to_quaternion().inverted() @ rotation_world
        end.matrix = Matrix.LocRotScale(end.head, rotation, end.matrix.to_scale())
        bpy.context.view_layer.update()

    def turn(name, rotation):
        bone = rig.pose.bones["mixamorig:" + name]
        matrix = rig.matrix_world @ bone.matrix
        bone.matrix = rig.matrix_world.inverted() @ Matrix.LocRotScale(
            matrix.translation, rotation @ matrix.to_quaternion(), matrix.to_scale()
        )
        bpy.context.view_layer.update()

    def look(frame):
        # Deliberate glances with holds, followed by a smooth return to the start.
        keys = [
            (0, -0.10, 0.02),
            (40, -0.10, 0.02),
            (78, 0.32, -0.03),
            (128, 0.32, -0.03),
            (180, -0.24, 0.06),
            (228, -0.24, 0.06),
            (278, 0.06, -0.08),
            (308, 0.06, -0.08),
            (360, -0.10, 0.02),
        ]
        for (start, yaw_a, pitch_a), (end, yaw_b, pitch_b) in zip(keys, keys[1:]):
            if start <= frame <= end:
                t = (frame - start) / (end - start)
                t = t * t * (3 - 2 * t)
                return yaw_a + (yaw_b - yaw_a) * t, pitch_a + (pitch_b - pitch_a) * t
        return keys[-1][1:]

    for frame in range(361):
        bpy.context.scene.frame_set(frame)
        for bone in rig.pose.bones:
            bone.matrix_basis = finger_basis.get(bone.name, Matrix.Identity(4))
        bpy.context.view_layer.update()
        phase = tau * frame / 360
        breath = sin(phase * 3)
        shift = 0.018 * sin(phase)
        hips = rig.pose.bones["mixamorig:Hips"]
        pelvis = hips.matrix.copy()
        pelvis.translation += rig.matrix_world.inverted().to_3x3() @ Vector(
            (0.025 + shift, 0, -0.025)
        )
        hips.matrix = pelvis
        bpy.context.view_layer.update()
        # Feet stay at their authored marks as the pelvis settles over one leg.
        for side in ["Right", "Left"]:
            foot = feet[side]
            target = foot.translation + Vector(
                (
                    -0.060 if side == "Right" else 0.060,
                    0.065 if side == "Right" else -0.040,
                    0,
                )
            )
            reach(
                [f"mixamorig:{side}{part}" for part in ["UpLeg", "Leg", "Foot"]],
                target,
                foot.to_quaternion(),
                Vector((0, -1, 0)),
            )
        turn("Spine", Quaternion((0, 1, 0), -0.035 - shift * 0.4))
        turn("Spine1", Quaternion((1, 0, 0), 0.018 + breath * 0.007))
        yaw, pitch = look(frame)
        turn("Spine2", Quaternion((0, 0, 1), yaw * 0.12))
        turn("Neck", Quaternion((0, 0, 1), yaw * 0.3))
        turn("Head", Quaternion((0, 0, 1), yaw * 0.7) @ Quaternion((1, 0, 0), pitch))
        # Raise the muzzle across the chest while keeping both hand contacts
        # attached to the same rigid rifle transform throughout the idle loop.
        gun_rotation = Quaternion((0, 0, 1), 1.40 + 0.025 * sin(phase)) @ Quaternion(
            (1, 0, 0), -0.22 + 0.012 * breath
        )
        wrist_rotation = gun_rotation @ right_rotation
        origin = (
            Vector((-0.18 + shift * 0.4, -0.25, 1.14 + 0.006 * breath))
            + wrist_rotation @ carry_offset
        )
        right = origin - wrist_rotation @ socket_offset
        left = (
            origin
            + gun_rotation @ support
            - (gun_rotation @ left_rotation) @ palm_offsets["Left"]
        )
        for side, target, rotation in [
            ("Right", right, wrist_rotation),
            ("Left", left, gun_rotation @ left_rotation),
        ]:
            reach(
                [f"mixamorig:{side}{part}" for part in ["Arm", "ForeArm", "Hand"]],
                target,
                rotation,
                # Tuck the support elbow down and forward instead of forcing
                # it outside the wrist, against the direction of the palm.
                Vector((0, -0.7, -1)) if side == "Left" else Vector((-0.3, 0, -1)),
            )
        # Curl the index independently of the three fingers around the grip.
        # Its distal pad reaches the trigger inside the guard from the near side.
        distal = rig.pose.bones["mixamorig:RightHandIndex3"]
        # Use the exported fingertip joint: Blender synthesizes display tails
        # for imported glTF bones, and those are not the skinned finger lengths.
        distal_matrix = rig.matrix_world @ distal.matrix
        fingertip = rig.pose.bones["mixamorig:RightHandIndex4"]
        distal_axis = rig.matrix_world.to_3x3() @ (fingertip.head - distal.head)
        pad_direction = gun_rotation @ Vector((1, 0, 0))
        distal_rotation = (
            distal_axis.rotation_difference(pad_direction)
            @ distal_matrix.to_quaternion()
        )
        trigger = origin + gun_rotation @ Vector((-0.003, -0.022, 0.033))
        reach(
            [f"mixamorig:RightHandIndex{i}" for i in [1, 2, 3]],
            trigger - pad_direction * distal_axis.length,
            distal_rotation,
            gun_rotation @ Vector((-1, 0, 0)),
        )
        for bone in rig.pose.bones:
            bone.rotation_mode = "QUATERNION"
            bone.keyframe_insert("rotation_quaternion", frame=frame)
            bone.keyframe_insert("location", frame=frame)
    # Export a menu-only attachment in bone-local coordinates. The game keeps
    # its existing world socket and first-person scene.
    attachment = bpy.data.objects.new("MenuWeaponSocket", None)
    bpy.context.collection.objects.link(attachment)
    attachment.parent = rig
    attachment.parent_type = "BONE"
    attachment.parent_bone = "mixamorig:RightHand"
    hand = rig.matrix_world @ rig.pose.bones["mixamorig:RightHand"].matrix
    attachment.matrix_world = hand @ basis.inverted().to_matrix().to_4x4()
    attachment.matrix_world.translation += hand.to_quaternion() @ socket_offset
    bpy.context.view_layer.update()
    action.use_fake_user = True
    return action


def add_menu_to_generated():
    """Append the menu clip to existing exports; retain gameplay clip payloads."""
    sys.path.insert(0, str(ROOT / "tools"))
    from inspect_assets import read_glb, write_glb
    import copy
    import struct

    for filename in ["attacker.glb", "defender.glb"]:
        path = OUT / filename
        original, original_tail = read_glb(path)
        clean()
        rig, objects = import_rig(path)
        for obj in list(objects):
            if obj.name == "MenuWeaponSocket":
                objects.remove(obj)
                bpy.data.objects.remove(obj, do_unlink=True)
        idle = next(
            track.strips[0].action
            for track in rig.animation_data.nla_tracks
            if track.name == "idle_rifle"
        )
        action = author_menu_hold(rig, idle)
        ground_action(rig, objects, action)
        for track in list(rig.animation_data.nla_tracks):
            rig.animation_data.nla_tracks.remove(track)
        rig.animation_data.action = None
        track = rig.animation_data.nla_tracks.new()
        track.name = "menu_hold_rifle"
        track.strips.new("menu_hold_rifle", 0, action)
        temporary = Path("/private/tmp") / ("menu-" + filename)
        export(temporary, [*objects, bpy.data.objects["MenuWeaponSocket"]])
        donor, donor_tail = read_glb(temporary)
        animation = copy.deepcopy(
            next(a for a in donor["animations"] if a["name"] == "menu_hold_rifle")
        )
        node_ids = {n.get("name"): i for i, n in enumerate(original["nodes"])}
        for channel in animation["channels"]:
            channel["target"]["node"] = node_ids[
                donor["nodes"][channel["target"]["node"]]["name"]
            ]
        socket = copy.deepcopy(
            next(n for n in donor["nodes"] if n.get("name") == "MenuWeaponSocket")
        )
        if "MenuWeaponSocket" in node_ids:
            original["nodes"][node_ids["MenuWeaponSocket"]] = socket
        else:
            original["nodes"][node_ids["mixamorig:RightHand"]].setdefault(
                "children", []
            ).append(len(original["nodes"]))
            original["nodes"].append(socket)
        # Our appended menu channels live after the original asset payload.
        # Remove prior menu-only trailing accessors/views so reruns do not grow it.
        retained = [a for a in original["animations"] if a["name"] != "menu_hold_rifle"]
        used = {
            s[key]
            for a in retained
            for s in a["samplers"]
            for key in ["input", "output"]
        }
        for mesh in original["meshes"]:
            for primitive in mesh["primitives"]:
                used.update(primitive["attributes"].values())
                if "indices" in primitive:
                    used.add(primitive["indices"])
                for target in primitive.get("targets", []):
                    used.update(target.values())
        used.update(
            s["inverseBindMatrices"]
            for s in original.get("skins", [])
            if "inverseBindMatrices" in s
        )
        original["accessors"] = original["accessors"][: max(used) + 1]
        used_views = {
            a["bufferView"] for a in original["accessors"] if "bufferView" in a
        }
        used_views.update(
            i["bufferView"] for i in original.get("images", []) if "bufferView" in i
        )
        original["bufferViews"] = original["bufferViews"][: max(used_views) + 1]
        end = max(
            v.get("byteOffset", 0) + v["byteLength"] for v in original["bufferViews"]
        )
        binary = bytearray(original_tail[8 : 8 + end])

        def append_accessor(index):
            accessor = copy.deepcopy(donor["accessors"][index])
            view = copy.deepcopy(donor["bufferViews"][accessor["bufferView"]])
            offset = view.get("byteOffset", 0)
            binary.extend(bytes((-len(binary)) % 4))
            view["byteOffset"] = len(binary)
            binary.extend(donor_tail[8 + offset : 8 + offset + view["byteLength"]])
            accessor["bufferView"] = len(original["bufferViews"])
            original["bufferViews"].append(view)
            result = len(original["accessors"])
            original["accessors"].append(accessor)
            return result

        for sampler in animation["samplers"]:
            sampler["input"] = append_accessor(sampler["input"])
            sampler["output"] = append_accessor(sampler["output"])
        original["animations"] = [
            a for a in original["animations"] if a["name"] != "menu_hold_rifle"
        ] + [animation]
        binary.extend(bytes((-len(binary)) % 4))
        original["buffers"][0]["byteLength"] = len(binary)
        write_glb(path, original, struct.pack("<I4s", len(binary), b"BIN\x00") + binary)
        temporary.unlink()
        print("MENU_CLIP_APPENDED", filename)


def bake_skin(source_path, output):
    clean()
    source, source_objects = import_rig(ROOT / "assets/models/skins/shooter.glb")
    source_actions = {name: bpy.data.actions[action] for name, action in CLIPS.items()}
    source.name = "MotionSource"
    target, target_objects = import_rig(source_path)
    target.name = "Armature"
    target.animation_data.action = None
    for track in list(target.animation_data.nla_tracks):
        target.animation_data.nla_tracks.remove(track)
    # Preserve target's bind/rest transforms; never overwrite its bone lengths.
    bones = list(target.pose.bones)
    actions = []
    for name, source_action in source_actions.items():
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        target.animation_data.action = action
        source.animation_data.action = source_action
        start, end = source_action.frame_range
        frames = max(2, round(end - start))
        for frame in range(frames + 1):
            source_frame = start + (end - start) * frame / frames
            bpy.context.scene.frame_set(int(source_frame), subframe=source_frame % 1)
            for bone in bones:
                bone.rotation_mode = "QUATERNION"
                bone.location = (0, 0, 0)
                bone.scale = (1, 1, 1)
                donor = source.pose.bones.get(bone.name)
                if donor is None:
                    continue
                # World-orientation delta accounts for source-only Neck1 through
                # the donor's evaluated global pose, instead of guessing paths.
                donor_rest = donor.bone.matrix_local.to_quaternion()
                desired = (
                    donor.matrix.to_quaternion()
                    @ donor_rest.inverted()
                    @ bone.bone.matrix_local.to_quaternion()
                )
                parent_pose = (
                    bone.parent.matrix.to_quaternion() if bone.parent else Quaternion()
                )
                parent_rest = bone.parent.bone.matrix_local if bone.parent else None
                rest_local = (
                    (parent_rest.inverted() @ bone.bone.matrix_local)
                    if parent_rest
                    else bone.bone.matrix_local
                )
                bone.rotation_quaternion = (
                    rest_local.to_quaternion().inverted()
                    @ parent_pose.inverted()
                    @ desired
                )
                bpy.context.view_layer.update()
                bone.keyframe_insert(
                    "rotation_quaternion", frame=frame, group=bone.name
                )
                bone.keyframe_insert("location", frame=frame, group=bone.name)
        if name in ["idle_rifle", "fire_rifle"]:
            align_support_hand(target, action)
        actions.append(action)
    # Explicit authored stance/death variants based on the baked rifle pose.
    idle = actions[0]
    for name in ["crouch_idle", "crouch_walk", "death"]:
        action = idle.copy()
        action.name = name
        target.animation_data.action = action
        if name == "death":
            for fc in list(action.fcurves):
                if fc.data_path.startswith('pose.bones["mixamorig:Hips"]'):
                    action.fcurves.remove(fc)
            hips = target.pose.bones["mixamorig:Hips"]
            for frame, angle, down in [
                (0, 0, 0),
                (8, 0.2, -12),
                (18, 0.9, -45),
                (30, 1.5, -85),
            ]:
                hips.rotation_mode = "QUATERNION"
                hips.rotation_quaternion = Quaternion((1, 0, 0), angle)
                hips.location = (0, down, 0)
                hips.keyframe_insert("rotation_quaternion", frame=frame)
                hips.keyframe_insert("location", frame=frame)
        else:
            if name == "crouch_walk":
                bpy.data.actions.remove(action)
                action = next(a for a in actions if a.name == "walk_forward").copy()
                action.name = name
                target.animation_data.action = action
            # Bake a lowered pelvis and bent legs onto the actual target motion.
            # Crouch walk retains the donor's alternating gait at reduced amplitude.
            end = round(action.frame_range[1])
            samples = []
            for frame in range(end + 1):
                bpy.context.scene.frame_set(frame)
                pose = {}
                for side in ["Left", "Right"]:
                    for part, angle in [("UpLeg", -1.2), ("Leg", 2.1), ("Foot", -0.9)]:
                        bone = target.pose.bones["mixamorig:" + side + part]
                        motion = Quaternion().slerp(
                            bone.rotation_quaternion,
                            0.3 if name == "crouch_walk" else 0.0,
                        )
                        pose[bone.name] = Quaternion((1, 0, 0), angle) @ motion
                samples.append(pose)
            for fc in list(action.fcurves):
                if (
                    any(
                        fc.data_path.startswith(f'pose.bones["{bone}"]')
                        for bone in samples[0]
                    )
                    or fc.data_path == 'pose.bones["mixamorig:Hips"].location'
                ):
                    action.fcurves.remove(fc)
            for frame, pose in enumerate(samples):
                for bone_name, rotation in pose.items():
                    bone = target.pose.bones[bone_name]
                    bone.rotation_quaternion = rotation
                    bone.keyframe_insert("rotation_quaternion", frame=frame)
                hips = target.pose.bones["mixamorig:Hips"]
                hips.location = (0, -48, 0)
                hips.keyframe_insert("location", frame=frame)
        action.use_fake_user = True
        actions.append(action)
    actions.append(author_menu_hold(target, idle))
    for action in actions:
        if action.name in ["crouch_idle", "crouch_walk", "death", "menu_hold_rifle"]:
            ground_action(target, target_objects, action)
    target.animation_data.action = None
    for action in actions:
        track = target.animation_data.nla_tracks.new()
        track.name = action.name
        strip = track.strips.new(action.name, 0, action)
        strip.name = action.name
        track.mute = False
    for obj in source_objects:
        bpy.data.objects.remove(obj, do_unlink=True)
    for action in list(bpy.data.actions):
        if action not in actions:
            bpy.data.actions.remove(action)
    # Explicit texture export budget: every character map <= 2048 pixels.
    for image in bpy.data.images:
        if image.size[0] > 2048 or image.size[1] > 2048:
            factor = 2048 / max(image.size)
            image.scale(round(image.size[0] * factor), round(image.size[1] * factor))
    export(OUT / output, [*target_objects, bpy.data.objects["MenuWeaponSocket"]])
    print("EXPORTED", output, [a.name for a in actions])


def author_viewmodel_pose(rig, actions, grip_center, magazine_center):
    """Pose intact arms for the first-person camera, preserving bone lengths.

    Explicit shoulder anchors and elbow poles preserve arm proportions. Reload
    retains the original rifle movement and blends back to the exact idle grip.
    """
    from math import sqrt
    from mathutils import Matrix  # ty: ignore[unresolved-import]

    rig.animation_data.action = actions["idle_rifle"]
    bpy.context.scene.frame_set(0)
    bpy.context.view_layer.update()
    idle_basis = {b.name: b.matrix_basis.copy() for b in rig.pose.bones}
    right_idle = rig.matrix_world @ rig.pose.bones["mixamorig:RightHand"].matrix
    left_idle = rig.matrix_world @ rig.pose.bones["mixamorig:LeftHand"].matrix
    right_palm = (
        sum(
            (
                rig.matrix_world @ rig.pose.bones[f"mixamorig:RightHandMiddle{i}"].head
                for i in [1, 2, 3]
            ),
            Vector(),
        )
        / 3
    )
    left_palm = (
        sum(
            (
                rig.matrix_world @ rig.pose.bones[f"mixamorig:LeftHandMiddle{i}"].head
                for i in [1, 2, 3]
            ),
            Vector(),
        )
        / 3
    )
    gun_offset = right_palm - grip_center - right_idle.translation
    palm_offset = left_idle.to_quaternion().inverted() @ (
        left_palm - left_idle.translation
    )
    support = Vector((-0.04, -0.265, 0.035))
    magazine_rotation = (
        Quaternion((1, 0, 0), -1.57079632679) @ left_idle.to_quaternion()
    )
    handle_rotation = left_idle.to_quaternion()

    def smooth_keys(frame, keys):
        for (start, a), (end, b) in zip(keys, keys[1:]):
            if start <= frame <= end:
                t = (frame - start) / (end - start)
                t = t * t * (3 - 2 * t)
                return a.lerp(b, t) if isinstance(a, Vector) else a.slerp(b, t)
        return keys[-1][1]

    contact_keys = [
        (0, support),
        (8, support),
        (18, magazine_center),
        (32, magazine_center + Vector((0, 0, -0.22))),
        (46, magazine_center + Vector((0, 0, -0.22))),
        (65, magazine_center),
        (68, magazine_center + Vector((-0.10, 0.0, 0.0))),
        (73, Vector((-0.04, -0.07, 0.085))),
        (79, Vector((-0.04, -0.015, 0.085))),
        (84, Vector((-0.04, -0.07, 0.085))),
        (94, support),
        (99, support),
    ]
    rotation_keys = [
        (0, left_idle.to_quaternion()),
        (8, left_idle.to_quaternion()),
        (18, magazine_rotation),
        (65, magazine_rotation),
        (73, handle_rotation),
        (84, handle_rotation),
        (94, left_idle.to_quaternion()),
        (99, left_idle.to_quaternion()),
    ]
    for action_name, action in actions.items():
        rig.animation_data.action = action
        samples = []
        reloading = action_name == "reload_rifle"
        for frame in range(100 if reloading else round(action.frame_range[1]) + 1):
            bpy.context.scene.frame_set(frame)
            if reloading:
                # Retain the source reload performance. Only its entry and
                # recovery converge to our shared grip, preventing the final
                # hold-air pose and discontinuity at the gameplay timer edge.
                weight = max(0.0, 1.0 - frame / 6, (frame - 78) / 21)
                weight = min(weight, 1.0)
                weight = weight * weight * (3 - 2 * weight)
                for bone in rig.pose.bones:
                    location, rotation, scale = bone.matrix_basis.decompose()
                    idle_location, idle_rotation, idle_scale = idle_basis[
                        bone.name
                    ].decompose()
                    bone.matrix_basis = Matrix.LocRotScale(
                        location.lerp(idle_location, weight),
                        rotation.slerp(idle_rotation, weight),
                        scale.lerp(idle_scale, weight),
                    )
            bpy.context.view_layer.update()
            pose = {}
            for side in ["Right", "Left"]:
                upper, lower, hand = [
                    rig.pose.bones[f"mixamorig:{side}{part}"]
                    for part in ["Arm", "ForeArm", "Hand"]
                ]
                wrist_world = rig.matrix_world @ hand.head
                hand_rotation = hand.matrix.to_quaternion()
                if side == "Left":
                    right_world = (
                        rig.matrix_world @ rig.pose.bones["mixamorig:RightHand"].matrix
                    )
                    gun_rotation = (
                        right_world.to_quaternion()
                        @ right_idle.to_quaternion().inverted()
                    )
                    gun_origin = right_world.translation + gun_rotation @ gun_offset
                    contact = smooth_keys(frame, contact_keys) if reloading else support
                    rotation = (
                        smooth_keys(frame, rotation_keys)
                        if reloading
                        else left_idle.to_quaternion()
                    )
                    wrist_world = gun_origin + gun_rotation @ (
                        contact - rotation @ palm_offset
                    )
                    hand_rotation = (
                        rig.matrix_world.to_quaternion().inverted()
                        @ gun_rotation
                        @ rotation
                    )
                    # Keep the same curled finger pose while gripping the
                    # handguard, magazine and charging handle.
                    for bone in rig.pose.bones:
                        if bone.name.startswith("mixamorig:LeftHand") and bone != hand:
                            bone.matrix_basis = idle_basis[bone.name]
                    bpy.context.view_layer.update()
                target = rig.matrix_world.inverted() @ wrist_world
                length1 = (lower.head - upper.head).length
                length2 = (hand.head - lower.head).length
                # Camera-only detached arm anchors; no character/body changes.
                shoulder_world = Vector(
                    (
                        -0.22 if side == "Right" else -0.03,
                        0.02 if side == "Right" else -0.34,
                        1.12 if side == "Right" else 1.08,
                    )
                )
                shoulder = rig.matrix_world.inverted() @ shoulder_world
                axis = (target - shoulder).normalized()
                distance = (target - shoulder).length
                # Preserve the wrist trajectory if a reload reaches outward;
                # move the shoulder along with it rather than stretching bones.
                reach = length1 + length2 - 0.01
                if distance > reach:
                    shoulder += axis * (distance - reach)
                    distance = reach
                along = (length1**2 - length2**2 + distance**2) / (2 * distance)
                pole = rig.matrix_world.inverted().to_3x3() @ Vector(
                    (
                        -0.4 if side == "Right" else 0.4,
                        0.1,
                        -1,
                    )
                )
                bend = (pole - axis * pole.dot(axis)).normalized()
                elbow = (
                    shoulder + axis * along + bend * sqrt(max(0, length1**2 - along**2))
                )
                rotation = (lower.head - upper.head).rotation_difference(
                    elbow - shoulder
                ) @ upper.matrix.to_quaternion()
                upper.matrix = Matrix.LocRotScale(
                    shoulder, rotation, upper.matrix.to_scale()
                )
                bpy.context.view_layer.update()
                rotation = (hand.head - lower.head).rotation_difference(
                    target - lower.head
                ) @ lower.matrix.to_quaternion()
                lower.matrix = Matrix.LocRotScale(
                    lower.head, rotation, lower.matrix.to_scale()
                )
                bpy.context.view_layer.update()
                hand.matrix = Matrix.LocRotScale(
                    hand.head, hand_rotation, hand.matrix.to_scale()
                )
                bpy.context.view_layer.update()
                for bone in [upper, lower, hand]:
                    pose[bone.name] = (
                        bone.location.copy(),
                        bone.rotation_quaternion.copy(),
                    )
            pose = {
                bone.name: (bone.location.copy(), bone.rotation_quaternion.copy())
                for bone in rig.pose.bones
            }
            samples.append(pose)
        for curve in list(action.fcurves):
            if any(
                curve.data_path.startswith(f'pose.bones["{name}"]')
                for name in samples[0]
            ):
                action.fcurves.remove(curve)
        for frame, pose in enumerate(samples):
            for name, (location, rotation) in pose.items():
                bone = rig.pose.bones[name]
                bone.location = location
                bone.rotation_quaternion = rotation
                bone.keyframe_insert("location", frame=frame)
                bone.keyframe_insert("rotation_quaternion", frame=frame)


def bake_weapon(first_person_only=False):
    import bmesh  # ty: ignore[unresolved-import]
    from mathutils import Matrix  # ty: ignore[unresolved-import]

    clean()
    rig, arm_objects = import_rig(OUT / "attacker.glb")
    # Only explicit arm vertex groups contribute to the first-person mesh.
    keep_bones = {
        "mixamorig:" + side + part
        for side in ["Left", "Right"]
        for part in [
            "Arm",
            "ForeArm",
            "Hand",
            "HandThumb1",
            "HandThumb2",
            "HandThumb3",
            "HandIndex1",
            "HandIndex2",
            "HandIndex3",
            "HandMiddle1",
            "HandMiddle2",
            "HandMiddle3",
            "HandRing1",
            "HandRing2",
            "HandRing3",
            "HandPinky1",
            "HandPinky2",
            "HandPinky3",
        ]
    }
    for obj in arm_objects:
        if obj.type != "MESH":
            continue
        groups = {g.index for g in obj.vertex_groups if g.name in keep_bones}
        keep = {
            v.index
            for v in obj.data.vertices
            if sum(g.weight for g in v.groups if g.group in groups) > 0.6
        }
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.verts.ensure_lookup_table()
        bmesh.ops.delete(
            bm, geom=[v for v in bm.verts if v.index not in keep], context="VERTS"
        )
        bm.to_mesh(obj.data)
        bm.free()
    if first_person_only:
        # Fuller sleeve circumference in bind space, with unchanged bone
        # lengths, wrist joints, hand mesh, UVs and longitudinal proportions.
        sleeve_bones = {
            f"mixamorig:{side}{part}"
            for side in ["Left", "Right"]
            for part in ["Arm", "ForeArm"]
        }
        for obj in arm_objects:
            if obj.type != "MESH":
                continue
            groups = {group.index: group.name for group in obj.vertex_groups}
            to_rig = rig.matrix_world.inverted() @ obj.matrix_world
            to_mesh = to_rig.inverted().to_3x3()
            for vertex in obj.data.vertices:
                position = to_rig @ vertex.co
                delta = Vector()
                for group in vertex.groups:
                    name = groups[group.group]
                    if name not in sleeve_bones:
                        continue
                    bone = rig.data.bones[name]
                    axis = (bone.tail_local - bone.head_local).normalized()
                    offset = position - bone.head_local
                    radial = offset - axis * offset.dot(axis)
                    delta += radial * (0.20 * group.weight)
                vertex.co += to_mesh @ delta
    # Exported actions are named explicitly; inspect the actual imported suffix.
    actions = {
        name: bpy.data.actions[name + "_Armature"]
        for name in ["idle_rifle", "fire_rifle", "reload_rifle"]
    }
    rig.animation_data.action = actions["idle_rifle"]
    bpy.context.scene.frame_set(0)
    bpy.context.view_layer.update()
    # Preserve source sleeve proportions and skinning; frame the camera around
    # the intact arms instead of stretching vertices to hide their cut edges.
    hand_world = rig.matrix_world @ rig.pose.bones["mixamorig:RightHand"].matrix
    hand_rotation = hand_world.to_quaternion()
    palm_center = (
        sum(
            (
                rig.matrix_world @ rig.pose.bones[f"mixamorig:RightHandMiddle{i}"].head
                for i in [1, 2, 3]
            ),
            Vector(),
        )
        / 3
    )
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(ROOT / "assets/models/weapons/ak_47.glb"))
    gun_objects = set(bpy.data.objects) - before
    meshes = [o for o in gun_objects if o.type == "MESH" and o.name != "BulletSpawn"]
    grip = bpy.data.objects.new("WeaponGrip", None)
    bpy.context.collection.objects.link(grip)
    for obj in meshes:
        obj.parent = None
        obj.matrix_world = Matrix.Identity(4)
        # Source gun is 20 units long along X. Explicit conversion to a 0.8 m
        # gun, forward -Y in Blender, grip at source X=-3, vertical source -Y.
        for vertex in obj.data.vertices:
            x, y, z = vertex.co
            vertex.co = Vector((z * 0.04, -(x + 3) * 0.04, -y * 0.04))
        obj.parent = grip
    magazine = next(o for o in meshes if o.name == "Object_7")
    magazine.name = "Magazine"
    bolt = next(o for o in meshes if o.name == "Object_6")
    bolt.name = "Bolt"
    for obj in list(gun_objects):
        if obj not in meshes:
            bpy.data.objects.remove(obj, do_unlink=True)
    muzzle = bpy.data.objects.new("Muzzle", None)
    bpy.context.collection.objects.link(muzzle)
    muzzle.parent = grip
    muzzle.location = (0, -0.52, 0.065)
    # World gun is expressed in the right-hand socket's rotation space.
    grip.rotation_mode = "QUATERNION"
    grip.rotation_quaternion = hand_rotation.inverted()
    # First-person scene keeps the same skinning and all three authored motions.
    grip.parent = rig
    grip.parent_type = "BONE"
    grip.parent_bone = "mixamorig:RightHand"
    grip.matrix_parent_inverse = Matrix.Identity(4)
    bpy.context.view_layer.update()
    # Explicit pistol-grip surface selection in the inspected rifle model.
    # Seat its center inside the curled middle finger, not at the wrist joint.
    pistol_grip = next(obj for obj in meshes if obj.name == "Object_3")
    grip_vertices = [
        v.co for v in pistol_grip.data.vertices if v.co.y > 0.02 and v.co.z < -0.02
    ]
    assert len(grip_vertices) == 96
    grip_center = sum(grip_vertices, Vector()) / len(grip_vertices)
    if first_person_only:
        magazine_center = sum((v.co for v in magazine.data.vertices), Vector()) / len(
            magazine.data.vertices
        )
        author_viewmodel_pose(rig, actions, grip_center, magazine_center)
        rig.animation_data.action = actions["idle_rifle"]
        bpy.context.scene.frame_set(0)
        bpy.context.view_layer.update()
    grip.matrix_world = Matrix.Translation(
        palm_center - grip_center if first_person_only else hand_world.translation
    )
    bpy.context.view_layer.update()
    for track in list(rig.animation_data.nla_tracks):
        rig.animation_data.nla_tracks.remove(track)
    rig.animation_data.action = None
    for name, action in actions.items():
        track = rig.animation_data.nla_tracks.new()
        track.name = name
        track.strips.new(name, 0, action)
    # Magazine extraction/insertion and bolt action have their own authored
    # channels; NLA track names merge them with the corresponding arm action.
    for obj in [magazine, bolt]:
        obj.animation_data_create()
        for name in ["idle_rifle", "fire_rifle", "reload_rifle"]:
            action = bpy.data.actions.new("weapon_" + obj.name + "_" + name)
            obj.animation_data.action = action
            keys = (
                [(0, 0), (30, 0)]
                if name == "idle_rifle"
                else [(0, 0), (3, 0)]
                if name == "fire_rifle" and obj == magazine
                else [(0, 0), (1, 0.035), (3, 0)]
                if name == "fire_rifle" and obj == bolt
                else [
                    (0, 0),
                    (14, 0),
                    (28, 0.28),
                    (55, 0.28),
                    (73, 0),
                    (85, 0),
                    (90, 0),
                ]
                if obj == magazine
                else [(0, 0), (68, 0), (74, 0.07), (83, 0), (90, 0)]
            )
            if name == "reload_rifle":
                keys.append((99, 0))
                if first_person_only:
                    keys = (
                        [(0, 0), (18, 0), (32, 0.22), (46, 0.22), (65, 0), (99, 0)]
                        if obj == magazine
                        else [(0, 0), (73, 0), (79, 0.055), (84, 0), (99, 0)]
                    )
            if first_person_only and name == "reload_rifle":
                samples = []
                for frame in range(100):
                    for (start, a), (end, b) in zip(keys, keys[1:]):
                        if start <= frame <= end:
                            t = (frame - start) / (end - start)
                            samples.append((frame, a + (b - a) * t * t * (3 - 2 * t)))
                            break
                keys = samples
            for frame, offset in keys:
                obj.location = (
                    0,
                    offset if obj == bolt else 0,
                    -offset if obj == magazine else 0,
                )
                obj.keyframe_insert("location", frame=frame)
            obj.animation_data.action = None
            track = obj.animation_data.nla_tracks.new()
            track.name = name
            track.strips.new(name, 0, action)
    local_matrix = grip.matrix_basis.copy()
    grip.parent = None
    grip.parent_type = "OBJECT"
    grip.matrix_world = Matrix.Identity(4)
    grip.rotation_mode = "QUATERNION"
    grip.rotation_quaternion = hand_rotation.inverted()
    if not first_person_only:
        export(OUT / "ak_world.glb", [grip, *meshes, muzzle])
    grip.parent = rig
    grip.parent_type = "BONE"
    grip.parent_bone = "mixamorig:RightHand"
    grip.matrix_basis = local_matrix
    # Retain the complete stock and receiver in the first-person asset.
    export(OUT / "ak_view.glb", [rig, *arm_objects, grip, *meshes, muzzle])
    # Use the exact exported bone-local grip, including Blender/glTF basis
    # conversion. A world-space inverse rotation is not a bone-local socket.
    sys.path.insert(0, str(ROOT / "tools"))
    from inspect_assets import read_glb, write_glb

    if not first_person_only:
        view, _ = read_glb(OUT / "ak_view.glb")
        world, tail = read_glb(OUT / "ak_world.glb")
        socket = next(
            node for node in view["nodes"] if node.get("name") == "WeaponGrip"
        )
        world_socket = next(
            node for node in world["nodes"] if node.get("name") == "WeaponGrip"
        )
        world_socket["rotation"] = socket["rotation"]
        write_glb(OUT / "ak_world.glb", world, tail)
    if not first_person_only:
        bind_exported_magazine(["ak_view.glb", "ak_world.glb"])


def bind_exported_magazine(filenames):
    """Resolve palm contact in glTF node space, without Blender bone-tail offsets."""
    import bisect
    import struct
    from mathutils import Matrix  # ty: ignore[unresolved-import]
    from inspect_assets import read_glb, write_glb

    view, tail = read_glb(OUT / "ak_view.glb")
    binary = tail[8:]

    def values(document, data, index):
        accessor = document["accessors"][index]
        buffer = document["bufferViews"][accessor["bufferView"]]
        width = {"SCALAR": 1, "VEC3": 3, "VEC4": 4}[accessor["type"]]
        offset = buffer.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        stride = buffer.get("byteStride", width * 4)
        return [
            struct.unpack_from("<" + "f" * width, data, offset + i * stride)
            for i in range(accessor["count"])
        ]

    nodes = view["nodes"]
    magazine = next(i for i, n in enumerate(nodes) if n.get("name") == "Magazine")
    palm = next(
        i for i, n in enumerate(nodes) if n.get("name") == "mixamorig:LeftHandMiddle1"
    )
    parents = {
        child: parent
        for parent, node in enumerate(nodes)
        for child in node.get("children", [])
    }
    positions = values(
        view,
        binary,
        view["meshes"][nodes[magazine]["mesh"]]["primitives"][0]["attributes"][
            "POSITION"
        ],
    )
    center = sum((Vector(p) for p in positions), Vector()) / len(positions)
    animation = next(a for a in view["animations"] if a["name"] == "reload_rifle")
    channels = []
    for channel in animation["channels"]:
        sampler = animation["samplers"][channel["sampler"]]
        channels.append(
            (
                channel["target"],
                [v[0] for v in values(view, binary, sampler["input"])],
                values(view, binary, sampler["output"]),
            )
        )

    samples = []
    for frame in range(100):
        poses = [
            {
                key: node.get(key, default)
                for key, default in [
                    ("translation", (0, 0, 0)),
                    ("rotation", (0, 0, 0, 1)),
                    ("scale", (1, 1, 1)),
                ]
            }
            for node in nodes
        ]
        for target, times, output in channels:
            index = min(
                max(bisect.bisect_right(times, frame / 30) - 1, 0), len(times) - 1
            )
            end = min(index + 1, len(times) - 1)
            alpha = (
                0
                if times[end] == times[index]
                else (frame / 30 - times[index]) / (times[end] - times[index])
            )
            a, b = output[index], output[end]
            if target["path"] == "rotation":
                rotation = Quaternion((a[3], *a[:3])).slerp(
                    Quaternion((b[3], *b[:3])), alpha
                )
                value = (rotation.x, rotation.y, rotation.z, rotation.w)
            else:
                value = tuple(x + (y - x) * alpha for x, y in zip(a, b))
            poses[target["node"]][target["path"]] = value
        cache = {}

        def world(index):
            if index not in cache:
                pose = poses[index]
                q = pose["rotation"]
                local = Matrix.LocRotScale(
                    Vector(pose["translation"]),
                    Quaternion((q[3], *q[:3])),
                    Vector(pose["scale"]),
                )
                cache[index] = (
                    world(parents[index]) @ local if index in parents else local
                )
            return cache[index]

        contact = world(parents[magazine]).inverted() @ world(palm).translation - center
        weight = min(max((frame - 8) / 12, 0), max((73 - frame) / 13, 0), 1)
        weight = weight * weight * (3 - 2 * weight)
        samples.append(tuple(contact * weight))
    for filename in filenames:
        document, tail = read_glb(OUT / filename)
        data = bytearray(tail)
        action = next(a for a in document["animations"] if a["name"] == "reload_rifle")
        channel = next(
            c
            for c in action["channels"]
            if document["nodes"][c["target"]["node"]].get("name") == "Magazine"
            and c["target"]["path"] == "translation"
        )
        accessor = document["accessors"][
            action["samplers"][channel["sampler"]]["output"]
        ]
        assert accessor["count"] == len(samples)
        buffer = document["bufferViews"][accessor["bufferView"]]
        offset = 8 + buffer.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        for i, value in enumerate(samples):
            struct.pack_into(
                "<3f", data, offset + i * buffer.get("byteStride", 12), *value
            )
        accessor.pop("min", None)
        accessor.pop("max", None)
        write_glb(OUT / filename, document, data)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    mode = sys.argv[sys.argv.index("--") + 1] if "--" in sys.argv else "characters"
    if mode == "characters":
        bake_skin(
            ROOT / "assets/models/skins/attacker_default_skin.glb", "attacker.glb"
        )
        bake_skin(ROOT / "assets/models/skins/defense_default_skin.glb", "defender.glb")
        (ROOT / "assets/config/character_clips.json").write_text(
            json.dumps(
                {
                    **CLIPS,
                    "menu_hold_rifle": "authored: planted carry, breathing and deliberate glances",
                },
                indent=2,
            )
            + "\n"
        )
    elif mode == "viewmodel":
        bake_weapon(first_person_only=True)
    elif mode == "menu":
        add_menu_to_generated()
        (ROOT / "assets/config/character_clips.json").write_text(
            json.dumps(
                {
                    **CLIPS,
                    "menu_hold_rifle": "authored: planted carry, breathing and deliberate glances",
                },
                indent=2,
            )
            + "\n"
        )
    elif mode == "weapon":
        bake_weapon()
        bake_weapon(first_person_only=True)
    else:
        raise ValueError(f"Unknown export mode: {mode}")


if __name__ == "__main__":
    main()
