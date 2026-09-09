"""Adapt the supplied AKM hand rig/reload directly in glTF space.

Run with Blender 4.2's Python for mathutils. Preserves the source skeleton, twist
joints, weights and sampled motion; no Blender skin import or bone retargeting.
"""

import copy
import math
import struct
import sys
from pathlib import Path

from mathutils import Matrix, Quaternion, Vector  # ty: ignore[unresolved-import]

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from inspect_assets import read_glb, write_glb  # noqa: E402

SOURCE = ROOT / "assets/models/viewmodels/akm_reload_animation.glb"
OUT = ROOT / "assets/generated"
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
FORMATS = {5126: "f", 5125: "I", 5123: "H", 5121: "B"}


class Glb:
    def __init__(self, path):
        self.doc, tail = read_glb(path)
        self.data = bytearray(tail[8:])

    def values(self, index):
        a = self.doc["accessors"][index]
        b = self.doc["bufferViews"][a["bufferView"]]
        fmt = FORMATS[a["componentType"]] * WIDTHS[a["type"]]
        stride = b.get("byteStride", struct.calcsize(fmt))
        offset = b.get("byteOffset", 0) + a.get("byteOffset", 0)
        return [
            struct.unpack_from("<" + fmt, self.data, offset + i * stride)
            for i in range(a["count"])
        ]

    def accessor(self, values, kind, component=5126):
        self.data.extend(b"\0" * (-len(self.data) % 4))
        start = len(self.data)
        for value in values:
            self.data.extend(
                struct.pack("<" + FORMATS[component] * WIDTHS[kind], *value)
            )
        views = self.doc["bufferViews"]
        views.append(
            {"buffer": 0, "byteOffset": start, "byteLength": len(self.data) - start}
        )
        a = {
            "bufferView": len(views) - 1,
            "componentType": component,
            "count": len(values),
            "type": kind,
        }
        if component == 5126:
            a["min"] = [min(v[i] for v in values) for i in range(WIDTHS[kind])]
            a["max"] = [max(v[i] for v in values) for i in range(WIDTHS[kind])]
        self.doc["accessors"].append(a)
        return len(self.doc["accessors"]) - 1

    def write(self, path):
        self.data.extend(b"\0" * (-len(self.data) % 4))
        self.doc["buffers"] = [{"byteLength": len(self.data)}]
        write_glb(
            path, self.doc, struct.pack("<I4s", len(self.data), b"BIN\0") + self.data
        )


def matrix(values):
    return Matrix([values[i : i + 4] for i in range(0, 16, 4)]).transposed()


def first_pose(glb):
    nodes = copy.deepcopy(glb.doc["nodes"])
    for c in glb.doc["animations"][0]["channels"]:
        s = glb.doc["animations"][0]["samplers"][c["sampler"]]
        nodes[c["target"]["node"]][c["target"]["path"]] = glb.values(s["output"])[0]
    parents = {child: i for i, n in enumerate(nodes) for child in n.get("children", [])}
    cache = {}

    def world(i):
        if i not in cache:
            n = nodes[i]
            q = n.get("rotation", (0, 0, 0, 1))
            local = (
                matrix(n["matrix"])
                if "matrix" in n
                else Matrix.LocRotScale(
                    Vector(n.get("translation", (0, 0, 0))),
                    Quaternion((q[3], *q[:3])),
                    Vector(n.get("scale", (1, 1, 1))),
                )
            )
            cache[i] = world(parents[i]) @ local if i in parents else local
        return cache[i]

    return [world(i) for i in range(len(nodes))]


def repair_arm_bind_space(glb):
    """Undo the GLB's baked whole-mesh scales, not individual joint lengths.

    Both source arms share identical indexed geometry, mirrored across X. The
    left mesh carries the FBX rig scale (3200); the right includes an additional
    uniform scale and sign. Recover that exact factor from corresponding source
    vertices and verify the correspondence before converting either mesh.
    """
    primitives = [glb.doc["meshes"][i]["primitives"][0] for i in (2, 3)]
    left, right = [glb.values(p["attributes"]["POSITION"]) for p in primitives]
    ratio = math.sqrt(
        sum(sum(v * v for v in p) for p in right)
        / sum(sum(v * v for v in p) for p in left)
    )
    assert len(left) == len(right) == 1685
    assert (
        max(
            abs(b - sign * ratio * a)
            for p, q in zip(left, right)
            for a, b, sign in zip(p, q, (1, -1, -1))
        )
        < 0.003
    )
    for p, vertices, divisor in zip(
        primitives, (left, right), (3200.0, -3200.0 * ratio)
    ):
        p["attributes"]["POSITION"] = glb.accessor(
            [tuple(v / divisor for v in vertex) for vertex in vertices], "VEC3"
        )
        if divisor < 0:
            for name in ("NORMAL", "TANGENT"):
                values = glb.values(p["attributes"][name])
                p["attributes"][name] = glb.accessor(
                    [tuple(-v for v in vertex) for vertex in values],
                    "VEC3" if name == "NORMAL" else "VEC4",
                )
            values = glb.values(p["indices"])
            reversed_indices = []
            for i in range(0, len(values), 3):
                reversed_indices.extend((values[i], values[i + 2], values[i + 1]))
            p["indices"] = glb.accessor(reversed_indices, "SCALAR", 5125)


def channel(glb, clip, node, path, times, values):
    kind = "VEC4" if path == "rotation" else "VEC3"
    clip["channels"].append(
        {"sampler": len(clip["samplers"]), "target": {"node": node, "path": path}}
    )
    clip["samplers"].append(
        {
            "input": glb.accessor([(t,) for t in times], "SCALAR"),
            "output": glb.accessor(values, kind),
            "interpolation": "LINEAR",
        }
    )


def make_clips(glb, root):
    source = glb.doc["animations"][0]
    reload = copy.deepcopy(source)
    reload["name"] = "reload_rifle"
    clips = []
    for name, duration in (("idle_rifle", 1.0), ("fire_rifle", 0.12)):
        clip = {"name": name, "channels": [], "samplers": []}
        for c in source["channels"]:
            s = source["samplers"][c["sampler"]]
            value = glb.values(s["output"])[0]
            channel(
                glb,
                clip,
                c["target"]["node"],
                c["target"]["path"],
                [0, duration],
                [value, value],
            )
        times = [0, 0.025, duration] if name == "fire_rifle" else [0, duration]
        offsets = (
            [(0, 0, 0), (0, 0.12, -0.45), (0, 0, 0)]
            if name == "fire_rifle"
            else [(0, 0, 0)] * 2
        )
        channel(glb, clip, root, "translation", times, offsets)
        clips.append(clip)
    channel(glb, reload, root, "translation", [0, 4.35], [(0, 0, 0)] * 2)
    glb.doc["animations"] = [*clips, reload]


def export_variants():
    glb = Glb(SOURCE)
    repair_arm_bind_space(glb)
    nodes = glb.doc["nodes"]
    nodes[92]["name"] = "WeaponGrip"
    nodes[93]["name"] = "Bolt"
    nodes[97]["name"] = "Magazine"
    nodes[99]["name"] = "MagazineSpare"
    muzzle = len(nodes)
    nodes.append({"name": "Muzzle", "translation": [0, 0.075, 0.812]})
    nodes[92].setdefault("children", []).append(muzzle)
    # Exclude the supplied transparent background plane, retaining the full rig.
    nodes[10]["children"] = []
    root = len(nodes)
    nodes.append({"name": "RifleView", "children": [0]})
    glb.doc["scenes"][0]["nodes"] = [root]
    make_clips(glb, root)
    for mesh in glb.doc["meshes"]:
        for p in mesh["primitives"]:
            p["attributes"].pop("TEXCOORD_2", None)
    for team in ("soldier", "police"):
        arms = next(m for m in glb.doc["materials"] if m["name"] == "Arms")
        arms["pbrMetallicRoughness"]["baseColorFactor"] = (
            [1, 1, 1, 1] if team == "soldier" else [0.94, 0.97, 1, 1]
        )
        glb.write(OUT / f"ak_view_{team}.glb")


if __name__ == "__main__":
    export_variants()
