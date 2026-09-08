"""Render textured inventory previews from the shipped weapon models.

Blender 4.2: --background --factory-startup --python tools/export_inventory.py
HUD silhouettes and gameplay assets are left untouched.
"""

import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Matrix, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_menu import camera, render  # noqa: E402
from export_knife import knife_mesh  # noqa: E402

OUT = ROOT / "assets/generated/ui/inventory"


def studio(target):
    scene = bpy.context.scene
    scene.render.film_transparent = True
    scene.world = bpy.data.worlds.new("Inventory studio")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (
        0.65,
        0.72,
        0.82,
        1,
    )
    scene.world.node_tree.nodes["Background"].inputs[1].default_value = 0.7
    for position, power, size in [((-1, 1, 2), 180, 2), ((1, -1, 1), 100, 1.5)]:
        bpy.ops.object.light_add(
            type="AREA", location=Vector(target) + Vector(position)
        )
        light = bpy.context.object
        light.data.energy = power
        light.data.size = size
        light.rotation_euler = (
            (Vector(target) - light.location).to_track_quat("-Z", "Y").to_euler()
        )


def rifle():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(ROOT / "assets/models/weapons/ak_47.glb"))
    meshes = []
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
        meshes.append(obj)
        points.extend(vertex.co.copy() for vertex in obj.data.vertices)
    minimum = Vector(tuple(min(p[i] for p in points) for i in range(3)))
    maximum = Vector(tuple(max(p[i] for p in points) for i in range(3)))
    center = (minimum + maximum) / 2
    width = maximum.x - minimum.x
    for obj in meshes:
        for vertex in obj.data.vertices:
            vertex.co = (vertex.co - center) / width
    cam = camera((0, 2, 0.18), (0, 0, 0))
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = 1.18
    studio((0, 0, 0))
    render(OUT / "ak47.png", 480, 360)


def knife():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    knife_mesh()
    cam = camera((0.05, 0.15, 2), (0, 0.15, 0))
    cam.rotation_euler.z = 1.25
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = 0.62
    studio((0, 0.15, 0))
    render(OUT / "knife.png", 480, 360)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    rifle()
    knife()
