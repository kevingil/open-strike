"""Export downloaded equipment models and inventory previews from equipment.json.

Blender --background --factory-startup --python tools/export_equipment.py [-- keys]
Armor exports are inventory assets only, never character attachments.
"""

import json
import math
import sys
from pathlib import Path

import bpy  # ty: ignore[unresolved-import]
from mathutils import Euler, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from export_catalog import bounds, clean, export_glb, import_source, meshes  # noqa: E402
from export_inventory import studio  # noqa: E402
from export_menu import camera, render  # noqa: E402


def build(item, sources):
    clean()
    for part in item["parts"]:
        source = sources[part["source"]]
        before = set(meshes())
        import_source(ROOT / source["path"])
        objects = [obj for obj in meshes() if obj not in before]
        if "meshes" in source:
            selected = set(source["meshes"])
            found = {obj.name for obj in objects}
            if not selected <= found:
                raise ValueError(f"Missing authored meshes: {selected - found}")
            for obj in objects:
                if obj.name not in selected:
                    bpy.data.objects.remove(obj, do_unlink=True)
            objects = [obj for obj in meshes() if obj not in before]
        if not objects:
            raise ValueError(f"No equipment geometry: {part['source']}")
        rotation = Euler(tuple(math.radians(v) for v in source["rotation"])).to_matrix()
        for obj in objects:
            obj.data.transform(rotation.to_4x4())
        low, high = bounds(objects)
        center = (low + high) / 2
        # Source up is explicitly authored; physical height is catalog data.
        scale = source["height"] / (high.z - low.z)
        for obj in objects:
            for vertex in obj.data.vertices:
                vertex.co = (vertex.co - center) * scale + Vector(part["offset"])
    objects = meshes()
    low, high = bounds(objects)
    center = (low + high) / 2
    # Normalize only the preview's camera/light distance, not the exported asset.
    model = ROOT / "assets" / item["model"]
    export_glb(model)
    span = max(high - low)
    cam = camera(center + Vector(item.get("camera", [0.6, -2, 0.6])) * span, center)
    cam.data.type = "ORTHO"
    bpy.context.view_layer.update()
    inverse = cam.matrix_world.inverted()
    points = [
        inverse @ (obj.matrix_world @ Vector(p))
        for obj in objects
        for p in obj.bound_box
    ]
    width = max(p.x for p in points) - min(p.x for p in points)
    height = max(p.y for p in points) - min(p.y for p in points)
    cam.data.ortho_scale = max(width, height * 4 / 3) * 1.2
    studio(center)
    icon = ROOT / "assets" / item["icon"]
    icon.parent.mkdir(parents=True, exist_ok=True)
    render(icon, 480, 360)
    print(f"Exported equipment {item['key']}: {model}", flush=True)


def main():
    catalog = json.loads((ROOT / "assets/config/equipment.json").read_text())
    wanted = set(sys.argv[sys.argv.index("--") + 1 :]) if "--" in sys.argv else None
    items = catalog["items"]
    if wanted:
        unknown = wanted - {item["key"] for item in items}
        if unknown:
            raise ValueError(f"Unknown equipment keys: {sorted(unknown)}")
        items = [item for item in items if item["key"] in wanted]
    for item in items:
        for part in item["parts"]:
            source = ROOT / catalog["sources"][part["source"]]["path"]
            if not source.is_file():
                raise FileNotFoundError(source)
    for item in items:
        build(item, catalog["sources"])


if __name__ == "__main__":
    main()
