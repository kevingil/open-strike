"""Export dedicated first-person art on the native reference rig (Blender 4.2).

Run after export_assets.py and export_knife.py. Source weapon GLBs are inputs,
never outputs. Uses the supplied AKM reload and knife reference rigs; see
ASSET_LICENSES.md for both CC BY 4.0 sources.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

REFERENCE = ROOT / "assets/models/viewmodels/knife_animated.glb"


def style(mesh, character):
    original = mesh.data.materials[0]
    material = original.copy()
    material.name = f"{character.title()} first-person cloth and gloves"
    shader = material.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Metallic"].default_value = 0
    for link in list(shader.inputs["Metallic"].links):
        material.node_tree.links.remove(link)
    mesh.data.materials[0] = material


if __name__ == "__main__":
    from export_rifle_view import export_variants

    export_variants()
    from export_reference_knife import export_views, export_world_and_icons

    export_views()
    export_world_and_icons()
