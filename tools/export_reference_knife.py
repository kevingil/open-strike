"""Export native reference hands and both knife choices with Blender 4.2.

Preserves DJMaesen's rest skeleton, skin weights, UVs and animated knife parent.
Source frames: idle 0-40, slash 60-80, draw 123-145. See ASSET_LICENSES.md.
"""

import math
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Matrix  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_assets import clean, export, import_rig  # noqa: E402
from export_knife import knife_mesh, material  # noqa: E402
from export_menu import camera, render  # noqa: E402
from export_viewmodels import REFERENCE, style  # noqa: E402
from inspect_assets import read_glb, write_glb  # noqa: E402

OUT = ROOT / "assets/generated"
# Default knife +Y blade / +Z face -> source +Z blade / +X face.
KNIFE_BASIS = Matrix(((0, 0, 1, 0), (1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 0, 1)))


def bake_clips(objects):
    animated = [o for o in objects if o.animation_data and o.animation_data.nla_tracks]
    sources = {o: o.animation_data.nla_tracks[0].strips[0].action for o in animated}
    for obj in animated:
        for track in obj.animation_data.nla_tracks:
            track.mute = True
    clips = {}
    for name, times in {
        "idle_knife": list(range(41)),
        # Frame 66 is the crossing stroke; align it with the game's 0.15 s impact.
        "slash_knife": [
            60 + 6 * f / 4.5 if f <= 4.5 else 66 + 14 * (f - 4.5) / 12.5
            for f in range(18)
        ],
        "draw_knife": [123 + 22 * f / 11 for f in range(12)],
    }.items():
        for obj, source in sources.items():
            obj.animation_data.action = source
        samples = []
        for source_frame in times:
            frame = min(source_frame, 145)
            bpy.context.scene.frame_set(int(frame), subframe=frame % 1)
            bpy.context.view_layer.update()
            samples.append(
                {
                    obj: (
                        obj.matrix_basis.copy(),
                        {b.name: b.matrix_basis.copy() for b in obj.pose.bones}
                        if obj.type == "ARMATURE"
                        else {},
                    )
                    for obj in animated
                }
            )
        actions = {obj: bpy.data.actions.new(f"{name}_{obj.name}") for obj in animated}
        for obj, action in actions.items():
            obj.animation_data.action = action
        for frame, sample in enumerate(samples):
            for obj, (matrix, bones) in sample.items():
                obj.matrix_basis = matrix
                targets = [obj]
                for bone_name, pose in bones.items():
                    bone = obj.pose.bones[bone_name]
                    bone.matrix_basis = pose
                    targets.append(bone)
                for target in targets:
                    target.rotation_mode = "QUATERNION"
                    for channel in ["location", "rotation_quaternion", "scale"]:
                        target.keyframe_insert(channel, frame=frame)
        clips[name] = actions
    for obj in animated:
        obj.animation_data.action = None
        for track in list(obj.animation_data.nla_tracks):
            obj.animation_data.nla_tracks.remove(track)
        for name, actions in clips.items():
            track = obj.animation_data.nla_tracks.new()
            track.name = name
            track.strips.new(name, 0, actions[obj])
    bpy.context.scene.frame_set(0)


def compatible(path):
    document, tail = read_glb(path)
    for surface in document.get("materials", []):
        if "first-person" in surface.get("name", ""):
            surface["pbrMetallicRoughness"]["baseColorFactor"] = (
                [1.0, 0.95, 0.88, 1.0]
                if "soldier" in path.name
                else [0.85, 0.92, 1.0, 1.0]
            )
    for mesh in document["meshes"]:
        for primitive in mesh["primitives"]:
            primitive["attributes"].pop("TEXCOORD_2", None)
    write_glb(path, document, tail)


def export_views():
    clean()
    _, objects = import_rig(REFERENCE)
    # The glTF importer creates a bone-shape Icosphere outside the source scene.
    objects = {o for o in objects if o.name != "Icosphere"}
    arms = next(o for o in objects if o.name == "Object_7")
    blade = next(o for o in objects if o.name == "knife_knife_0")
    blade.parent.name = "KnifeGrip"
    bake_clips(objects)
    for character in ["soldier", "police"]:
        style(arms, character)
        shader = arms.data.materials[0].node_tree.nodes.get("Principled BSDF")
        # Retain the neutral glove texture for both teams, with a modest sleeve tint.
        shader.inputs["Base Color"].default_value = (1, 1, 1, 1)
        path = OUT / f"reference_knife_view_{character}.glb"
        export(path, objects)
        compatible(path)
    grip, meshes = knife_mesh()
    grip.parent = blade.parent
    grip.matrix_basis = KNIFE_BASIS @ Matrix.Diagonal((100, 100, 100, 1))
    # Match the reference blade length; retain the default knife's original grip.
    for vertex in meshes[0].data.vertices:
        if vertex.co.y > 0.08:
            vertex.co.y = 0.08 + (vertex.co.y - 0.08) * 0.60
    objects.remove(blade)
    bpy.data.objects.remove(blade, do_unlink=True)
    grip.name = "DefaultKnifeGeometry"
    objects.update([grip, *meshes])
    for character in ["soldier", "police"]:
        style(arms, character)
        path = OUT / f"knife_view_{character}.glb"
        export(path, objects)
        compatible(path)


def export_world_and_icons():
    clean()
    _, imported = import_rig(REFERENCE)
    blade = next(o for o in imported if o.name == "knife_knife_0")
    blade.parent = None
    blade.matrix_world = Matrix.Identity(4)
    conversion = Matrix.Diagonal((0.01, 0.01, 0.01, 1)) @ KNIFE_BASIS.inverted()
    for vertex in blade.data.vertices:
        vertex.co = conversion @ vertex.co
    for obj in list(imported):
        if obj != blade:
            bpy.data.objects.remove(obj, do_unlink=True)
    grip = bpy.data.objects.new("KnifeGrip", None)
    bpy.context.collection.objects.link(grip)
    blade.parent = grip
    path = OUT / "reference_knife_world.glb"
    export(path, [grip, blade])
    document, tail = read_glb(path)
    original, _ = read_glb(OUT / "knife_world.glb")
    socket = next(n for n in original["nodes"] if n.get("name") == "KnifeGrip")
    node = next(n for n in document["nodes"] if n.get("name") == "KnifeGrip")
    for key in ["translation", "rotation", "scale", "matrix"]:
        node.pop(key, None)
        if key in socket:
            node[key] = socket[key]
    write_glb(path, document, tail)
    compatible(path)
    cam = camera((0, 0.09, 1), (0, 0.09, 0))
    cam.rotation_euler.z = math.pi / 2
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = 0.42
    bpy.context.scene.render.film_transparent = True
    bpy.ops.object.light_add(type="AREA", location=(0, 0, 1))
    bpy.context.object.data.energy = 30
    bpy.context.object.data.size = 1
    render(OUT / "ui/inventory/reference_knife.png", 384, 256)
    white = material("Reference knife HUD", (1, 1, 1))
    shader = white.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Emission Color"].default_value = (1, 1, 1, 1)
    shader.inputs["Emission Strength"].default_value = 1
    blade.data.materials.clear()
    blade.data.materials.append(white)
    render(OUT / "ui/reference_knife.png", 384, 128)


if __name__ == "__main__":
    export_views()
    export_world_and_icons()
