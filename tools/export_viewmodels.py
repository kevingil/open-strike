"""Fit dedicated first-person art to the existing weapon actions (Blender 4.2).

Run after export_assets.py and export_knife.py. Source weapon GLBs are inputs,
never outputs. Reference: DJMaesen, knife Animated, CC BY 4.0; see ASSET_LICENSES.
"""

import math
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_assets import clean, export, import_rig  # noqa: E402

OUT = ROOT / "assets/generated"
REFERENCE = ROOT / "assets/models/viewmodels/knife_animated.glb"
# Explicit semantic correspondence; no name guessing or automatic bone matching.
SIDES = {
    "Left": ["L_arm_02", "L_elbow_03", "L_wrist_04", "L_palm_016"],
    "Right": ["R_arm_025", "R_elbow_026", "R_wrist_027", "R_palm_040"],
}
FINGERS = {
    "Left": {
        "Thumb": ["L_thumb1_05", "L_thumb2_06", "L_thumb3_07"],
        "Index": ["L_point1_00", "L_point2_09", "L_point3_010"],
        "Middle": ["L_middle1_012", "L_middle2_013", "L_middle3_014"],
        "Ring": ["L_ring1_017", "L_ring2_018", "L_ring3_019"],
        "Pinky": ["L_pink1_021", "L_pink2_022", "L_pink3_023"],
    },
    "Right": {
        "Thumb": ["R_thumb1_028", "R_thumb2_029", "R_thumb3_030"],
        "Index": ["R_point1_032", "R_point2_033", "R_point3_034"],
        "Middle": ["R_middle1_036", "R_middle2_037", "R_middle3_038"],
        "Ring": ["R_ring1_041", "R_ring2_042", "R_ring3_043"],
        "Pinky": ["R_pink1_045", "R_pink2_046", "R_pink3_047"],
    },
}


def basis(origin, tip, across):
    """Anatomical axes avoid the different rolls in the two imported rigs."""
    y = (tip - origin).normalized()
    x = (across - y * across.dot(y)).normalized()
    z = x.cross(y).normalized()
    result = Matrix((x, y, z)).transposed().to_4x4()
    result.translation = origin
    return result


def fit_reference(rig):
    reference, imported = import_rig(REFERENCE)
    mesh = next(o for o in imported if o.type == "MESH" and o.name == "Object_7")
    transforms = {}
    mapping = {}
    for side, (arm, elbow, wrist, palm) in SIDES.items():
        prefix = f"mixamorig:{side}"

        def ref(name):
            return reference.matrix_world @ reference.data.bones[name].head_local

        def dst(name):
            return rig.matrix_world @ rig.data.bones[prefix + name].head_local

        across_ref = ref(FINGERS[side]["Index"][0]) - ref(FINGERS[side]["Pinky"][0])
        across_dst = dst("HandIndex1") - dst("HandPinky1")
        pairs: list[tuple[str, str | None, str, str]] = [
            (arm, elbow, "Arm", "ForeArm"),
            (elbow, wrist, "ForeArm", "Hand"),
            (wrist, FINGERS[side]["Middle"][0], "Hand", "HandMiddle1"),
        ]
        for digit, names in FINGERS[side].items():
            for i, name in enumerate(names):
                # Terminal source joints are exporter end markers at the origin.
                # Use the preceding phalanx direction for the distal segment.
                tip = names[i + 1] if i < 2 else None
                pairs.append((name, tip, f"Hand{digit}{i + 1}", f"Hand{digit}{i + 2}"))
        for name, tip, target, target_tip in pairs:
            a = ref(name)
            b = ref(tip) if tip else a + (a - ref(FINGERS[side][target[4:-1]][1]))
            c, d = dst(target), dst(target_tip)
            source_basis = basis(a, b, across_ref)
            target_basis = basis(c, d, across_dst)
            # Source units are centimetres. Preserve sleeve volume, with only
            # longitudinal fitting to the authored animation skeleton.
            transverse = 0.01 if target in {"Arm", "ForeArm"} else 0.009
            scale = Matrix.Diagonal(
                (transverse, (d - c).length / (b - a).length, transverse, 1)
            )
            transforms[name] = target_basis @ scale @ source_basis.inverted()
            mapping[name] = prefix + target
        # The extra palm bone controls the ulnar side of the reference glove.
        transforms[palm] = transforms[wrist]
        mapping[palm] = prefix + "Hand"
    groups = {g.index: g.name for g in mesh.vertex_groups}
    world = mesh.matrix_world.copy()
    vertices = []
    weights = []
    for v in mesh.data.vertices:
        influences = [(groups[g.group], g.weight) for g in v.groups if g.weight > 0]
        assert influences and all(name in mapping for name, _ in influences), influences
        total = sum(weight for _, weight in influences)
        point = world @ v.co
        vertices.append(
            sum(
                (transforms[name] @ point * weight for name, weight in influences),
                Vector(),
            )
            / total
        )
        weights.append([(mapping[name], weight / total) for name, weight in influences])
    mesh.parent = rig
    mesh.matrix_world = Matrix.Identity(4)
    mesh.name = "ViewmodelSleevesAndGloves"
    mesh.vertex_groups.clear()
    for name in sorted(set(mapping.values())):
        mesh.vertex_groups.new(name=name)
    for vertex, position, influences in zip(mesh.data.vertices, vertices, weights):
        vertex.co = position
        combined = {}
        for name, weight in influences:
            combined[name] = combined.get(name, 0) + weight
        for name, weight in combined.items():
            mesh.vertex_groups[name].add([vertex.index], weight, "REPLACE")
    for modifier in mesh.modifiers:
        if modifier.type == "ARMATURE":
            modifier.object = rig
    for obj in imported:
        if obj != mesh:
            bpy.data.objects.remove(obj, do_unlink=True)
    return mesh


def extend_sleeve_openings(mesh, rig):
    """Continue the two open upper sleeves behind the camera.

    The supplied arms intentionally have open upper ends. Add a hidden sleeve
    continuation there rather than stretching the visible forearms or gloves.
    """
    import bmesh  # ty: ignore[unresolved-import]

    bm = bmesh.new()
    bm.from_mesh(mesh.data)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=0.000001)
    deform = bm.verts.layers.deform.active
    for side in ["Left", "Right"]:
        name = f"mixamorig:{side}Arm"
        group = mesh.vertex_groups[name].index
        edges = [
            e
            for e in bm.edges
            if e.is_boundary and all(v[deform].get(group, 0) > 0.99 for v in e.verts)
        ]
        assert edges, f"Missing authored upper sleeve opening: {side}"
        old = set(bm.verts)
        bmesh.ops.extrude_edge_only(bm, edges=edges)
        new = [v for v in bm.verts if v not in old]
        upper = rig.data.bones[name].head_local
        elbow = rig.data.bones[f"mixamorig:{side}ForeArm"].head_local
        backwards = (rig.matrix_world.to_3x3() @ (upper - elbow)).normalized()
        bmesh.ops.translate(bm, verts=new, vec=backwards * 0.4)
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(mesh.data)
    bm.free()


def style(mesh, character):
    original = mesh.data.materials[0]
    material = original.copy()
    material.name = f"{character.title()} first-person cloth and gloves"
    shader = material.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Metallic"].default_value = 0
    for link in list(shader.inputs["Metallic"].links):
        material.node_tree.links.remove(link)
    mesh.data.materials[0] = material


def pose_arms(rig):
    """Reframe the elbows without moving any wrist, finger or weapon contact."""
    for track in rig.animation_data.nla_tracks:
        track.mute = True
    for track in rig.animation_data.nla_tracks:
        action = track.strips[0].action
        rig.animation_data.action = action
        samples = []
        for frame in range(round(action.frame_range[1]) + 1):
            bpy.context.scene.frame_set(frame)
            bpy.context.view_layer.update()
            result = {}
            for side, sign in [("Left", 1), ("Right", -1)]:
                upper, lower, hand = [
                    rig.pose.bones[f"mixamorig:{side}{part}"]
                    for part in ["Arm", "ForeArm", "Hand"]
                ]
                wrist = hand.matrix.copy()
                target = wrist.translation
                shoulder = rig.matrix_world.inverted() @ Vector(
                    (sign * 0.42, 0.10, 1.10)
                )
                length1 = (lower.head - upper.head).length
                length2 = (hand.head - lower.head).length
                axis = (target - shoulder).normalized()
                distance = (target - shoulder).length
                reach = length1 + length2 - 0.01
                if distance > reach:
                    shoulder += axis * (distance - reach)
                    distance = reach
                distance = max(distance, abs(length1 - length2) + 0.001)
                along = (length1**2 - length2**2 + distance**2) / (2 * distance)
                pole = rig.matrix_world.inverted().to_3x3() @ Vector((sign, 0.1, -0.6))
                bend = (pole - axis * pole.dot(axis)).normalized()
                elbow = (
                    shoulder
                    + axis * along
                    + bend * math.sqrt(max(0, length1**2 - along**2))
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
                hand.matrix = wrist
                bpy.context.view_layer.update()
                for bone in [upper, lower, hand]:
                    result[bone.name] = bone.matrix_basis.copy()
            samples.append((frame, result))
        for frame, poses in samples:
            for name, matrix in poses.items():
                bone = rig.pose.bones[name]
                bone.matrix_basis = matrix
                bone.rotation_mode = "QUATERNION"
                for channel in ["location", "rotation_quaternion", "scale"]:
                    bone.keyframe_insert(channel, frame=frame, group=name)
    rig.animation_data.action = None


def export_variant(weapon):
    clean()
    rig, objects = import_rig(OUT / f"{weapon}_view.glb")
    for obj in list(objects):
        if obj.type == "MESH" and any(m.type == "ARMATURE" for m in obj.modifiers):
            objects.remove(obj)
            bpy.data.objects.remove(obj, do_unlink=True)
    pose_arms(rig)
    mesh = fit_reference(rig)
    extend_sleeve_openings(mesh, rig)
    objects.add(mesh)
    # The existing authored actions, gun hierarchy and mechanical channels stay
    # intact. Only arm geometry changes; the world character is never exported.
    for obj in objects:
        if obj.animation_data:
            obj.animation_data.action = None
            for track in obj.animation_data.nla_tracks:
                track.mute = False
    from inspect_assets import read_glb, write_glb

    for character in ["soldier", "police"]:
        style(mesh, character)
        path = OUT / f"{weapon}_view_{character}.glb"
        export(path, objects)
        document, tail = read_glb(path)
        source, _ = read_glb(OUT / f"{weapon}_view.glb")
        socket_name = "WeaponGrip" if weapon == "ak" else "KnifeGrip"
        socket = next(
            node for node in source["nodes"] if node.get("name") == socket_name
        )
        # Blender imports glTF bone children at the bone tail. Restore the
        # authoritative head-relative socket transform after the round trip.
        for node in document["nodes"]:
            if node.get("name") == socket_name:
                for key in ["translation", "rotation", "scale", "matrix"]:
                    node.pop(key, None)
                    if key in socket:
                        node[key] = socket[key]
        for material in document["materials"]:
            if "first-person" in material.get("name", ""):
                pbr = material["pbrMetallicRoughness"]
                pbr["baseColorFactor"] = (
                    [1.0, 0.9, 0.75, 1]
                    if character == "soldier"
                    else [0.65, 0.8, 1.0, 1]
                )
                pbr["metallicFactor"] = 0
        # The reference carries an unused third UV set; all its texture slots
        # use UV0. Bevy supports UV0/UV1, so omit the unreferenced attribute.
        for exported_mesh in document["meshes"]:
            for primitive in exported_mesh["primitives"]:
                primitive["attributes"].pop("TEXCOORD_2", None)
        write_glb(path, document, tail)


if __name__ == "__main__":
    for weapon in ["ak", "knife"]:
        export_variant(weapon)
