"""Prepare the frostychaos144 CS2 Mirage source for gameplay and the menu vignette.

Blender 4.2: --background --factory-startup --python tools/export_mirage.py

Reads the Drive-sourced merged GLB (see ASSET_LICENSES / README credits),
joins collision into two meshes, caps textures at 1024, and writes the
gameplay GLB plus an A-site menu card.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import bmesh  # ty: ignore[unresolved-import]
import bpy  # ty: ignore[unresolved-import]
from mathutils import Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
SOURCE_CANDIDATES = [
    Path("/tmp/mirage-dl/full/more/merged_model.glb"),
    ROOT / "assets/maps/de_mirage/merged_model.glb",
]
OUT_MAP = ROOT / "assets/maps/de_mirage"
GENERATED = ROOT / "assets/generated"
MENU = GENERATED / "menu"
MAX_TEXTURE = 1024
# A-site letter/spray cluster from the source GLB (Y-up).
A_SITE = Vector((-11.7, -4.5, 55.5))


def source_path() -> Path:
    for path in SOURCE_CANDIDATES:
        if path.is_file():
            return path
    raise SystemExit(
        "Mirage source GLB not found. Place frostychaos144's merged_model.glb "
        "at assets/maps/de_mirage/merged_model.glb"
    )


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def apply_materials():
    used = set()
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        for slot in obj.material_slots:
            if slot.material:
                used.add(slot.material)
    for material in used:
        material.use_backface_culling = False
        tree = material.node_tree
        if tree is None:
            continue
        for node in tree.nodes:
            if node.type == "BSDF_PRINCIPLED":
                node.inputs["Metallic"].default_value = 0.0
                node.inputs["Roughness"].default_value = 0.95
            if node.type == "TEX_IMAGE" and node.image:
                image = node.image
                width, height = image.size
                largest = max(width, height)
                if largest > MAX_TEXTURE:
                    ratio = MAX_TEXTURE / largest
                    image.scale(max(1, round(width * ratio)), max(1, round(height * ratio)))


def delete_tool_volumes():
    remove = []
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            continue
        names = [slot.material.name.lower() for slot in obj.material_slots if slot.material]
        if names and all(name.startswith("tools") for name in names):
            remove.append(obj)
    for obj in remove:
        bpy.data.objects.remove(obj, do_unlink=True)
    return len(remove)


def is_helper_volume(obj) -> bool:
    return obj.name.split(".")[0] in {"Cube", "Cube001"}


def join_meshes(objects, name):
    objects = [
        obj
        for obj in objects
        if obj.type == "MESH" and obj.data.polygons and not is_helper_volume(obj)
    ]
    if not objects:
        return None
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    joined = bpy.context.view_layer.objects.active
    joined.name = name
    # Bake leftover node TRS so Bevy collision and authored spawns share world space.
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return joined


def is_alpha_material(material) -> bool:
    name = material.name.lower()
    return any(
        token in name
        for token in (
            "tree",
            "branch",
            "leaf",
            "plant",
            "glass",
            "trans",
            "alpha",
            "overlay",
        )
    )


def split_alpha(objects):
    opaque, alpha = [], []
    for obj in objects:
        mats = [slot.material for slot in obj.material_slots if slot.material]
        if mats and all(is_alpha_material(mat) for mat in mats):
            alpha.append(obj)
        else:
            opaque.append(obj)
    return opaque, alpha


def world_bounds(objects):
    mins = Vector((1e9, 1e9, 1e9))
    maxs = Vector((-1e9, -1e9, -1e9))
    for obj in objects:
        for corner in obj.bound_box:
            point = obj.matrix_world @ Vector(corner)
            mins.x, mins.y, mins.z = min(mins.x, point.x), min(mins.y, point.y), min(mins.z, point.z)
            maxs.x, maxs.y, maxs.z = max(maxs.x, point.x), max(maxs.y, point.y), max(maxs.z, point.z)
    return mins, maxs


def export_glb(path: Path, objects):
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        if obj is not None:
            obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_image_format="JPEG",
        export_jpeg_quality=80,
    )


def camera(position, target, lens=45):
    bpy.ops.object.camera_add(location=position)
    cam = bpy.context.object
    cam.rotation_euler = (Vector(target) - cam.location).to_track_quat("-Z", "Y").to_euler()
    cam.data.lens = lens
    bpy.context.scene.camera = cam
    return cam


def render(path: Path, width, height):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def gltf_to_blender(point: Vector) -> Vector:
    # glTF Y-up (+Z forward) -> Blender Z-up (-Y forward).
    return Vector((point.x, -point.z, point.y))


def menu_vignette():
    # Keep a local A-site pocket and put the character stand on the site floor.
    keep = []
    before = 0
    facing = Matrix.Rotation(-0.4, 4, "Z")
    anchor = gltf_to_blender(A_SITE)
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            continue
        before += len(obj.data.polygons)
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        remove = []
        for face in bm.faces:
            center = face.calc_center_median()
            offset = center - anchor
            if not (-22 <= offset.x <= 22 and -28 <= offset.y <= 18 and -6 <= offset.z <= 18):
                remove.append(face)
        bmesh.ops.delete(bm, geom=remove, context="FACES")
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
        for vertex in bm.verts:
            vertex.co = facing @ (vertex.co - anchor)
        bm.to_mesh(obj.data)
        bm.free()
        if obj.data.polygons:
            keep.append(obj)
        else:
            bpy.data.objects.remove(obj, do_unlink=True)
    if not keep:
        raise SystemExit("Mirage menu vignette deleted every face")
    MENU.mkdir(parents=True, exist_ok=True)
    export_glb(MENU / "mirage.glb", keep)
    scene = bpy.context.scene
    scene.world = bpy.data.worlds.new("Mirage sky")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.72, 0.80, 0.86, 1)
    scene.world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
    bpy.ops.object.light_add(type="SUN", location=(-4, -3, 9))
    sun = bpy.context.object
    sun.data.energy = 3
    sun.data.angle = 0.12
    sun.rotation_euler = (0.55, -0.35, -0.4)
    camera((0, -4.2, 1.8), (0, 8, 2), 22)
    render(MENU / "mirage-card.png", 360, 640)
    report = {
        "location": "A Site",
        "source_anchor": list(A_SITE),
        "source_faces": before,
        "vignette_faces": sum(len(o.data.polygons) for o in keep),
        "mesh_count": len(keep),
        "background_bytes": (MENU / "mirage.glb").stat().st_size,
        "card_bytes": (MENU / "mirage-card.png").stat().st_size,
        "max_texture_dimension": MAX_TEXTURE,
    }
    (MENU / "mirage-manifest.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    source = source_path()
    print(f"Importing {source}")
    reset()
    bpy.ops.import_scene.gltf(filepath=str(source))
    removed = delete_tool_volumes()
    apply_materials()
    meshes = [obj for obj in bpy.data.objects if obj.type == "MESH"]
    opaque, alpha = split_alpha(meshes)
    world = join_meshes(opaque, "mirage_world")
    foliage = join_meshes(alpha, "mirage_foliage")
    kept = [obj for obj in (world, foliage) if obj is not None]
    mins, maxs = world_bounds(kept)
    OUT_MAP.mkdir(parents=True, exist_ok=True)
    GENERATED.mkdir(parents=True, exist_ok=True)
    export_glb(OUT_MAP / "de_mirage.glb", kept)
    export_glb(GENERATED / "mirage.glb", kept)
    gameplay = {
        "source": str(source),
        "removed_tool_meshes": removed,
        "meshes": [obj.name for obj in kept],
        "bounds_min": list(mins),
        "bounds_max": list(maxs),
        "size": list(maxs - mins),
        "gameplay_bytes": (GENERATED / "mirage.glb").stat().st_size,
        "a_site": list(A_SITE),
        "b_site": [-52.2, -4.05, -7.11],
    }
    print("MIRAGE_EXPORT", json.dumps(gameplay))
    # Fresh scene for the menu crop so the joined gameplay mesh is not destroyed.
    reset()
    bpy.ops.import_scene.gltf(filepath=str(GENERATED / "mirage.glb"))
    apply_materials()
    report = menu_vignette()
    print("MIRAGE_MENU", json.dumps(report))


if __name__ == "__main__":
    if "--" in sys.argv:
        sys.argv = sys.argv[sys.argv.index("--") + 1 :]
    main()
