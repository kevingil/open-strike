"""Standalone Dust 2 vignette and model-derived rifle icon.

Blender 4.2: --background --factory-startup --python tools/export_menu.py -- scene|icon
The source map and gameplay exports are never modified.
"""

import json
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
import bmesh  # ty: ignore[unresolved-import]
from mathutils import Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/generated/menu"


def camera(position, target, lens=45):
    bpy.ops.object.camera_add(location=position)
    cam = bpy.context.object
    cam.rotation_euler = (
        (Vector(target) - cam.location).to_track_quat("-Z", "Y").to_euler()
    )
    cam.data.lens = lens
    bpy.context.scene.camera = cam
    return cam


def render(path, width, height):
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


def scene_export():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(ROOT / "assets/generated/dust2.glb"))
    # A-site from the Long approach: crates, raised site and double doors.
    # Keep only this local set and anchor the character on the flat approach.
    scale = 0.7257143
    anchor = Vector((51, 74, 0))  # Blender Z-up, glTF Z maps to -Y.
    facing = Matrix.Rotation(-0.6, 4, "Z")
    meshes = []
    before = 0
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            continue
        before += len(obj.data.polygons)
        for vertex in obj.data.vertices:
            vertex.co = obj.matrix_world @ vertex.co
        obj.parent = None
        obj.matrix_world = Matrix.Identity(4)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        remove = []
        for face in bm.faces:
            center = face.calc_center_median()
            if not (
                8 <= center.x <= 72 and 60 <= center.y <= 125 and -10 <= center.z <= 25
            ):
                remove.append(face)
        bmesh.ops.delete(bm, geom=remove, context="FACES")
        bmesh.ops.delete(
            bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
        )
        for vertex in bm.verts:
            vertex.co = facing @ ((vertex.co - anchor) * scale)
        bm.to_mesh(obj.data)
        bm.free()
        if not obj.data.polygons:
            bpy.data.objects.remove(obj, do_unlink=True)
        else:
            meshes.append(obj)
    assert meshes
    used_materials = set()
    for obj in meshes:
        for polygon in obj.data.polygons:
            used_materials.add(obj.data.materials[polygon.material_index])
    for material in used_materials:
        for node in material.node_tree.nodes:
            if node.type == "TEX_IMAGE" and node.image:
                img = node.image
                if max(img.size) > 1024:
                    ratio = 1024 / max(img.size)
                    img.scale(round(img.size[0] * ratio), round(img.size[1] * ratio))
        material.use_backface_culling = False
    OUT.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / "dust2.glb"),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_image_format="JPEG",
        export_jpeg_quality=82,
    )
    scene = bpy.context.scene
    scene.world = bpy.data.worlds.new("Dust sky")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (
        0.65,
        0.76,
        0.85,
        1,
    )
    scene.world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
    bpy.ops.object.light_add(type="SUN", location=(-4, -3, 9))
    sun = bpy.context.object
    sun.data.energy = 3
    sun.data.angle = 0.12
    sun.rotation_euler = (0.5, -0.5, -0.6)
    camera((0, -4, 1.8), (0, 8, 2), 22)
    source = ROOT / "assets/menu/dust2.blend"
    source.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.file.pack_all()
    # Purge the discarded full-map data before saving the editable vignette.
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(source), compress=True)
    render(OUT / "dust2-card.png", 360, 640)
    report = {
        "location": "A Site viewed from Long A",
        "source_anchor": list(anchor),
        "source_yaw": -0.6,
        "source_faces": before,
        "vignette_faces": sum(len(o.data.polygons) for o in meshes),
        "mesh_count": len(meshes),
        "material_count": len(used_materials),
        "background_bytes": (OUT / "dust2.glb").stat().st_size,
        "card_bytes": (OUT / "dust2-card.png").stat().st_size,
        "max_texture_dimension": 1024,
    }
    (OUT / "manifest.json").write_text(json.dumps(report, indent=2) + "\n")
    print("MENU_ASSETS", report)


def icon_export():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(ROOT / "assets/models/weapons/ak_47.glb"))
    material = bpy.data.materials.new("Silhouette white")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    nodes.clear()
    emission = nodes.new("ShaderNodeEmission")
    output = nodes.new("ShaderNodeOutputMaterial")
    material.node_tree.links.new(emission.outputs[0], output.inputs[0])
    points = []
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            continue
        if obj.name == "BulletSpawn":
            bpy.data.objects.remove(obj, do_unlink=True)
            continue
        obj.parent = None
        obj.matrix_world = Matrix.Identity(4)
        for vertex in obj.data.vertices:
            x, y, z = vertex.co
            vertex.co = (x, z, -y)
        bpy.context.view_layer.update()
        obj.data.materials.clear()
        obj.data.materials.append(material)
        points.extend(obj.matrix_world @ Vector(corner) for corner in obj.bound_box)
    minimum = Vector(tuple(min(p[i] for p in points) for i in range(3)))
    maximum = Vector(tuple(max(p[i] for p in points) for i in range(3)))
    center = (minimum + maximum) / 2
    # Original rifle runs along X, muzzle +X. View from +Y for muzzle left.
    cam = camera(center + Vector((0, 60, 0)), center)
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = (maximum.x - minimum.x) * 1.08
    bpy.context.scene.render.film_transparent = True
    bpy.context.scene.view_settings.view_transform = "Standard"
    path = ROOT / "assets/generated/ui/ak47.png"
    path.parent.mkdir(parents=True, exist_ok=True)
    render(path, 768, 256)


if __name__ == "__main__":
    operation = sys.argv[sys.argv.index("--") + 1]
    {"scene": scene_export, "icon": icon_export}[operation]()
