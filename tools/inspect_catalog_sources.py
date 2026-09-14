"""Dump rest-pose bounds, mesh names, and raw side/top previews."""

from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_catalog import bounds, clean, import_source, meshes  # noqa: E402
from export_menu import camera, render  # noqa: E402
from weapon_catalog import source_path  # noqa: E402

KEYS = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [
    "p90",
    "bizon",
    "nova",
    "xm1014",
    "sawedoff",
    "m249",
    "negev",
    "galil",
    "famas",
    "m4a4",
    "m4a1s",
    "sg553",
    "aug",
    "ssg08",
    "awp",
    "g3sg1",
    "scar20",
    "knife_ct",
    "knife_t",
    "karambit",
]
OUT = ROOT / "assets/generated/ui/inspect"


def preview(key: str, tag: str):
    objects = meshes()
    if not objects:
        return
    low, high = bounds(objects)
    center = (low + high) / 2
    size = high - low
    span = max(size.x, size.y, size.z, 0.001)
    views = {
        "side": ((center.x + span, center.y, center.z), (0, 0, 90)),
        "top": ((center.x, center.y, center.z + span), (0, 0, 0)),
        "front": ((center.x, center.y - span, center.z), (90, 0, 0)),
    }
    position, _ = views[tag]
    cam = camera(position, tuple(center))
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = span * 1.3
    scene = bpy.context.scene
    scene.render.film_transparent = True
    if scene.world is None:
        scene.world = bpy.data.worlds.new("Inspect")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.7, 0.74, 0.8, 1)
    bpy.ops.object.light_add(type="AREA", location=(center.x + span, center.y + span, center.z + span))
    bpy.context.object.data.energy = 400
    OUT.mkdir(parents=True, exist_ok=True)
    render(OUT / f"{key}_{tag}.png", 480, 360)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for key in KEYS:
        path = source_path(key)
        print(f"\n=== {key} {path} ===")
        clean()
        import_source(path, frame=1 if key == "sawedoff" else None)
        objects = meshes()
        print("meshes", [obj.name for obj in objects])
        if not objects:
            continue
        low, high = bounds(objects)
        size = high - low
        print(f"size {tuple(round(v, 4) for v in size)}")
        print(f"center {tuple(round(v, 4) for v in (low + high) / 2)}")
        axis = max(range(3), key=lambda i: size[i])
        print("longest", "XYZ"[axis])
        for tag in ("side", "top", "front"):
            preview(key, tag)


if __name__ == "__main__":
    main()
