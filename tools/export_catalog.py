"""Import catalog sources, apply loadout edits, and write world GLBs + icons.

Blender 4.2:
  /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \\
    --python tools/export_catalog.py
"""

from __future__ import annotations

import math
import sys
from pathlib import Path
from typing import Optional

import bpy  # ty: ignore[unresolved-import]
import bmesh  # ty: ignore[unresolved-import]
from mathutils import Euler, Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_menu import camera, render  # noqa: E402
from export_knife import knife_mesh, material as knife_material  # noqa: E402
from weapon_catalog import GENERATED, WEAPONS, source_path  # noqa: E402

OUT = GENERATED / "weapons"
UI = GENERATED / "ui" / "inventory"
AUTHORED = {"knife_stained", "knife_forest"}
# Authored from the downloaded source views, in Blender coordinates. No
# longest-axis guessing: each source declares its assembled pose and rotation
# into +Y barrel / +Z up before the final game-space conversion.
SOURCE_ROTATIONS = {
    "glock18": (0, 0, 180),
    "usps": (0, 0, 180),
    "p2000": (0, 0, 180),
    "dual_berettas": (30, 0, 0),
    "p250": (0, 0, 180),
    # Undo body_main body_0's presentation rotation: local -Z barrel, -Y up.
    "tec9": (172.592214, 115.859831, 0),
    "fiveseven": (0, 0, 90),
    "cz75": (0, 0, 90),
    "deagle": (0, 0, 180),
    "r8": (0, 0, 180),
    "mac10": (0, 0, 0),
    "mp9": (0, 0, 180),
    "mp7": (0, 0, -90),
    "mp5sd": (0, 0, 90),
    "ump45": (0, 0, -90),
}
LENGTHS = {
    "pistol": 0.22,
    "mid": 0.55,
    "rifle": 0.85,
    "sniper": 1.15,
    "knife": 0.32,
}
KIND = {
    "glock18": "pistol",
    "usps": "pistol",
    "p2000": "pistol",
    "dual_berettas": "pistol",
    "p250": "pistol",
    "tec9": "pistol",
    "fiveseven": "pistol",
    "cz75": "pistol",
    "deagle": "pistol",
    "r8": "pistol",
    "mac10": "mid",
    "mp9": "mid",
    "mp7": "mid",
    "mp5sd": "mid",
    "ump45": "mid",
    "p90": "mid",
    "bizon": "mid",
    "nova": "mid",
    "xm1014": "mid",
    "sawedoff": "mid",
    "m249": "mid",
    "negev": "mid",
    "galil": "rifle",
    "famas": "rifle",
    "ak47": "rifle",
    "m4a4": "rifle",
    "m4a1s": "rifle",
    "sg553": "rifle",
    "aug": "rifle",
    "ssg08": "sniper",
    "awp": "sniper",
    "g3sg1": "sniper",
    "scar20": "sniper",
    "knife": "knife",
    "knife_stained": "knife",
    "knife_forest": "knife",
    "knife_ct": "knife",
    "knife_t": "knife",
    "reference_knife": "knife",
    "karambit": "knife",
}


def clean():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30


def meshes():
    return [
        obj
        for obj in bpy.context.scene.objects
        if obj.type == "MESH" and not obj.hide_render
    ]


def bounds(objects):
    bpy.context.view_layer.update()
    points = [
        obj.matrix_world @ Vector(corner) for obj in objects for corner in obj.bound_box
    ]
    low = Vector(
        (min(p.x for p in points), min(p.y for p in points), min(p.z for p in points))
    )
    high = Vector(
        (max(p.x for p in points), max(p.y for p in points), max(p.z for p in points))
    )
    return low, high


def apply_world(obj):
    bpy.context.view_layer.update()
    transform = obj.matrix_world.copy()
    obj.parent = None
    obj.data.transform(transform)
    obj.matrix_world = Matrix.Identity(4)


def import_source(
    path: Path,
    root: Optional[str] = None,
    frame: Optional[int] = None,
    remove_groups: tuple[str, ...] = (),
):
    suffix = path.suffix.lower()
    before = set(bpy.data.objects)
    if suffix == ".glb" or suffix == ".gltf":
        bpy.ops.import_scene.gltf(filepath=str(path))
    elif suffix == ".fbx":
        bpy.ops.import_scene.fbx(filepath=str(path))
    elif suffix == ".obj":
        bpy.ops.wm.obj_import(filepath=str(path))
    elif suffix == ".blend":
        bpy.ops.wm.open_mainfile(filepath=str(path))
        before = set()
    else:
        raise ValueError(path)
    imported = set(bpy.data.objects) - before
    # Some accessories share a skinned mesh with the gun. Remove the declared
    # accessory's deform group rather than relying on object-name matching.
    for obj in imported:
        if obj.type != "MESH":
            continue
        indices = [
            obj.vertex_groups[name].index
            for name in remove_groups
            if name in obj.vertex_groups
        ]
        if not indices:
            continue
        mesh = bmesh.new()
        mesh.from_mesh(obj.data)
        deform = mesh.verts.layers.deform.active
        if deform is not None:
            vertices = [
                v for v in mesh.verts if any(v[deform].get(i, 0) > 0 for i in indices)
            ]
            bmesh.ops.delete(mesh, geom=vertices, context="VERTS")
            mesh.to_mesh(obj.data)
        mesh.free()
    rig_helpers = {
        bone.custom_shape
        for obj in imported
        if obj.type == "ARMATURE"
        for bone in obj.pose.bones
        if bone.custom_shape is not None
    }
    if frame is not None:
        bpy.context.scene.frame_set(frame)
    else:
        for obj in imported:
            obj.animation_data_clear()
            if obj.type == "ARMATURE":
                obj.data.pose_position = "REST"
    selected = imported
    if root is not None:
        source_root = bpy.data.objects[root]
        selected = {source_root, *source_root.children_recursive}
    bpy.context.view_layer.update()
    depsgraph = bpy.context.evaluated_depsgraph_get()
    # Bake each evaluated mesh before detaching anything. Imported parent
    # transforms and skin modifiers must be applied exactly once.
    baked = [
        (
            obj,
            bpy.data.meshes.new_from_object(
                obj.evaluated_get(depsgraph), depsgraph=depsgraph
            ),
            obj.matrix_world.copy(),
        )
        for obj in selected
        if obj.type == "MESH"
        and obj.name in bpy.context.scene.objects
        and not obj.hide_render
        and obj not in rig_helpers
    ]
    for obj, mesh, transform in baked:
        obj.parent = None
        obj.animation_data_clear()
        obj.modifiers.clear()
        obj.constraints.clear()
        obj.data = mesh
        obj.data.transform(transform)
        obj.matrix_world = Matrix.Identity(4)
    retained = {obj for obj, _, _ in baked}
    for obj in imported:
        if obj not in retained:
            bpy.data.objects.remove(obj, do_unlink=True)


def paint_black():
    mat = bpy.data.materials.new("All black")
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (0.02, 0.02, 0.022, 1)
    shader.inputs["Metallic"].default_value = 0.7
    shader.inputs["Roughness"].default_value = 0.45
    for obj in meshes():
        obj.data.materials.clear()
        obj.data.materials.append(mat)


def named(obj, *needles):
    text = obj.name.lower()
    return any(needle in text for needle in needles)


def strip_named(*needles):
    for obj in list(meshes()):
        if named(obj, *needles):
            bpy.data.objects.remove(obj, do_unlink=True)


def normalize(key: str, kind: str):
    objects = meshes()
    if not objects:
        return
    rotation = Euler(tuple(math.radians(a) for a in SOURCE_ROTATIONS[key])).to_matrix()
    for obj in objects:
        obj.data.transform(rotation.to_4x4())
        obj.data.update()
    low, high = bounds(objects)
    size = high - low
    target = LENGTHS[kind]
    scale = target / size.y
    center = (low + high) / 2
    for obj in objects:
        for vertex in obj.data.vertices:
            vertex.co = (vertex.co - center) * scale
        obj.data.update()


def add_empty(name, location):
    empty = bpy.data.objects.new(name, None)
    empty.empty_display_size = 0.02
    empty.location = location
    bpy.context.collection.objects.link(empty)
    return empty


def sockets(kind: str):
    objects = meshes()
    if not objects:
        return
    low, high = bounds(objects)
    grip = add_empty("WeaponGrip", (0, 0, 0))
    if kind == "knife":
        grip.name = "KnifeGrip"
        add_empty("Muzzle", (0, high.y, 0))
    else:
        add_empty("Muzzle", (0, high.y, 0))
        add_empty("Magazine", (0, (low.y + high.y) * 0.35, -0.03))
        add_empty("Bolt", (0, (low.y + high.y) * 0.55, 0.02))
    for obj in objects:
        obj.parent = grip
    for obj in bpy.context.scene.objects:
        if obj.type == "EMPTY" and obj != grip:
            obj.parent = grip
    return grip


def cylinder(name, location, radius, depth, axis="Y"):
    bpy.ops.mesh.primitive_cylinder_add(radius=radius, depth=depth, location=location)
    obj = bpy.context.object
    obj.name = name
    if axis == "Y":
        obj.rotation_euler = (math.pi / 2, 0, 0)
    bpy.ops.object.transform_apply(rotation=True)
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (0.08, 0.08, 0.09, 1)
    shader.inputs["Metallic"].default_value = 0.85
    shader.inputs["Roughness"].default_value = 0.35
    obj.data.materials.append(mat)
    return obj


def add_silencer():
    objects = meshes()
    if not objects:
        return
    low, high = bounds(objects)
    depth = max(0.08, (high.y - low.y) * 0.22)
    cylinder("Silencer", (0, high.y + depth * 0.5, high.z - 0.012), 0.012, depth)


def add_scope():
    objects = meshes()
    if not objects:
        return
    low, high = bounds(objects)
    y = (low.y + high.y) * 0.55
    z = high.z + 0.018
    body = cylinder("CompactScope", (0, y, z), 0.011, 0.07)
    ring = cylinder("CompactScopeRing", (0, y + 0.028, z), 0.014, 0.012)
    return body, ring


def authored_knife(finish: str):
    grip, objects = knife_mesh()
    if finish == "stained":
        for obj in objects:
            if obj.data.materials:
                mat = obj.data.materials[0]
                if mat.use_nodes:
                    shader = mat.node_tree.nodes.get("Principled BSDF")
                    shader.inputs["Base Color"].default_value = (0.18, 0.12, 0.07, 1)
                    shader.inputs["Roughness"].default_value = 0.55
    elif finish == "forest":
        rubber = knife_material("Forest grip", (0.05, 0.09, 0.04))
        for obj in objects:
            if "Handle" in obj.name:
                obj.data.materials.clear()
                obj.data.materials.append(rubber)
    return grip


def duplicate_akimbo():
    objects = meshes()
    if not objects:
        return
    copies = []
    for obj in objects:
        twin = obj.copy()
        twin.data = obj.data.copy()
        bpy.context.collection.objects.link(twin)
        copies.append(twin)
    for obj in objects:
        obj.location.x -= 0.04
        apply_world(obj)
    for obj in copies:
        obj.location.x += 0.04
        obj.location.y -= 0.035
        obj.location.z -= 0.04
        apply_world(obj)


def take_silencer_from(path: Path):
    before = set(bpy.data.objects)
    import_source(path)
    added = [obj for obj in set(bpy.data.objects) - before if obj.type == "MESH"]
    keep = [
        obj for obj in added if named(obj, "silencer", "suppress", "gemtech", "aurora")
    ]
    if not keep and added:
        keep = [min(added, key=lambda obj: sum(obj.dimensions))]
    for obj in added:
        if obj not in keep:
            bpy.data.objects.remove(obj, do_unlink=True)
    return keep


def export_glb(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in bpy.context.scene.objects:
        if obj.type in {"MESH", "EMPTY"}:
            obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_nla_strips=False,
        export_apply=True,
    )


def icon(key: str):
    objects = meshes()
    if not objects:
        return
    low, high = bounds(objects)
    center = (low + high) / 2
    authored = key in AUTHORED
    position = (
        (center.x, center.y, center.z + 1.2)
        if authored
        else (center.x + 1.2, center.y, center.z)
    )
    cam = camera(position, tuple(center))
    if authored:
        cam.rotation_euler.z = math.pi / 2
    cam.data.type = "ORTHO"
    height = high.x - low.x if authored else high.z - low.z
    cam.data.ortho_scale = max(high.y - low.y, height * 480 / 360) * 1.25
    scene = bpy.context.scene
    scene.render.film_transparent = True
    if scene.world is None:
        scene.world = bpy.data.worlds.new("Icon")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (
        0.65,
        0.72,
        0.82,
        1,
    )
    bpy.ops.object.light_add(
        type="AREA", location=(center.x + 2, center.y + 1, center.z + 2)
    )
    light = bpy.context.object
    light.data.energy = 200
    light.rotation_euler = (center - light.location).to_track_quat("-Z", "Y").to_euler()
    UI.mkdir(parents=True, exist_ok=True)
    render(UI / f"{key}.png", 480, 360)


def build(key: str, notes: str, path: Optional[Path]):
    authored = key in AUTHORED
    if not authored and (path is None or not path.is_file()):
        raise FileNotFoundError(
            f"Missing source for {key}; download it before exporting"
        )
    clean()
    kind = KIND[key]
    if authored:
        authored_knife({"knife_stained": "stained", "knife_forest": "forest"}[key])
    elif path and path.exists():
        import_source(
            path,
            root="Group063" if key == "dual_berettas" else None,
            frame=1 if key in {"glock18", "p250", "mac10"} else None,
            remove_groups=("Suppressor_07",) if key == "mac10" else (),
        )
        if key == "deagle":
            for name in ("Bullet_low_Bullet_0", "BulletCase_low_Bullet_0"):
                bpy.data.objects.remove(bpy.data.objects[name], do_unlink=True)
        if "silencer=strip" in notes:
            strip_named("silencer", "suppress", "sionics", "can")
        if "scope=strip" in notes:
            strip_named("aimpoint", "t2", "reddot", "red-dot", "optic", "holo", "acog")
        if "paint=black" in notes:
            paint_black()
        if "dual=1" in notes:
            strip_named("silencer", "suppress", "gemtech", "aurora")
        normalize(key, kind)
        if "dual=1" in notes:
            duplicate_akimbo()
        if "silencer=add" in notes:
            add_silencer()
        if "scope=add" in notes or (
            "scope=keep" in notes
            and not any(named(o, "scope", "optic") for o in meshes())
        ):
            add_scope()
    if not meshes():
        raise ValueError(f"No meshes imported for {key}")
    grip = sockets(kind) if not authored else None
    icon(key)
    if grip is not None:
        # glTF conversion turns Blender +Z into game +Y. The catalog view
        # profile expects +Y barrel and -Z up in weapon-local game space.
        grip.rotation_euler = (
            Matrix.Rotation(math.pi, 4, "Z") @ Matrix.Rotation(math.pi / 2, 4, "X")
        ).to_euler()
    export_glb(OUT / f"{key}.glb")
    origin = "authored" if authored else "source"
    print(f"exported {key} ({origin})")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    UI.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[sys.argv.index("--") + 1 :]) if "--" in sys.argv else None
    jobs = [(key, notes) for key, _uid, _author, _title, notes in WEAPONS]
    jobs += [
        ("knife_stained", ""),
        ("knife_forest", ""),
    ]
    if wanted:
        unknown = wanted - {key for key, _ in jobs}
        if unknown:
            raise ValueError(f"Unknown catalog keys: {', '.join(sorted(unknown))}")
        jobs = [(key, notes) for key, notes in jobs if key in wanted]
    sources = {key: source_path(key) for key, _ in jobs}
    missing = [
        key for key, path in sources.items() if key not in AUTHORED and path is None
    ]
    if missing:
        raise FileNotFoundError(
            f"Missing catalog sources: {', '.join(missing)}. "
            "Run tools/download_catalog.py with SKETCHFAB_TOKEN configured. "
            "No weapon assets were exported."
        )
    unconfigured = [
        key for key, _ in jobs if key not in AUTHORED and key not in SOURCE_ROTATIONS
    ]
    if unconfigured:
        raise ValueError(
            f"Source orientation needs authoring for: {', '.join(unconfigured)}. "
            "Inspect the source before adding its SOURCE_ROTATIONS entry."
        )
    for key, notes in jobs:
        build(key, notes, sources[key])


if __name__ == "__main__":
    main()
