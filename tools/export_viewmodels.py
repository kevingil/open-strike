"""Export dedicated first-person art on the native reference rig (Blender 4.2).

Run after export_assets.py and export_knife.py. Source weapon GLBs are inputs,
never outputs. Reference: DJMaesen, knife Animated, CC BY 4.0; see ASSET_LICENSES.
"""

import sys
from pathlib import Path

from mathutils import Matrix  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

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
