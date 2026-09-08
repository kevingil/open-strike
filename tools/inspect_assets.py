"""Validate generated GLB contracts without Blender or third-party packages."""

import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHARACTER_CLIPS = {
    "menu_hold_rifle",
    "idle_rifle",
    "walk_forward",
    "walk_backward",
    "run_forward",
    "strafe_left",
    "strafe_right",
    "fire_rifle",
    "reload_rifle",
    "jump",
    "crouch_idle",
    "crouch_walk",
    "death",
}
WEAPON_CLIPS = {"idle_rifle", "fire_rifle", "reload_rifle"}
CHARACTER_MATERIALS = {"Ch18_Body", "Ch35_body", "Ch35_body1", "Ch35_body2"}


def read_glb(path):
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<4sII", data)
    assert magic == b"glTF" and version == 2 and length == len(data), path
    size, kind = struct.unpack_from("<I4s", data, 12)
    assert kind == b"JSON", path
    return json.loads(data[20 : 20 + size]), data[20 + size :]


def write_glb(path, document, tail):
    encoded = json.dumps(document, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    header = struct.pack(
        "<4sIII4s", b"glTF", 2, 20 + len(encoded) + len(tail), len(encoded), b"JSON"
    )
    path.write_bytes(header + encoded + tail)


def apply_character_materials(path):
    """The imported body atlases are cloth/skin/painted armor, not bare metal."""
    document, tail = read_glb(path)
    for material in document.get("materials", []):
        if material.get("name") in CHARACTER_MATERIALS:
            pbr = material.setdefault("pbrMetallicRoughness", {})
            pbr.pop("metallicRoughnessTexture", None)
            pbr.update(metallicFactor=0.0, roughnessFactor=0.8)
    write_glb(path, document, tail)


def validate(path, expected_clips, required_nodes):
    document, tail = read_glb(path)
    for material in document.get("materials", []):
        if material.get("name") in CHARACTER_MATERIALS:
            pbr = material["pbrMetallicRoughness"]
            assert pbr["metallicFactor"] == 0.0 and pbr["roughnessFactor"] == 0.8, path
            assert "metallicRoughnessTexture" not in pbr, path
    binary = tail[8:]
    widths = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
    for accessor in document.get("accessors", []):
        if accessor["componentType"] != 5126 or "bufferView" not in accessor:
            continue
        view = document["bufferViews"][accessor["bufferView"]]
        width = widths[accessor["type"]]
        offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        stride = view.get("byteStride", width * 4)
        for index in range(accessor["count"]):
            values = struct.unpack_from(
                "<" + "f" * width, binary, offset + stride * index
            )
            assert all(math.isfinite(value) for value in values), (
                path,
                "Non-finite geometry or animation data",
            )
    animations = document.get("animations", [])
    names = {a["name"] for a in animations}
    assert names == expected_clips, (path, names ^ expected_clips)
    nodes = document.get("nodes", [])
    assert required_nodes <= {n.get("name") for n in nodes}, path
    assert document.get("scenes"), path
    for skin in document.get("skins", []):
        assert (
            len(skin["joints"])
            == document["accessors"][skin["inverseBindMatrices"]]["count"]
        ), path
        assert all(0 <= joint < len(nodes) for joint in skin["joints"]), path
    for animation in animations:
        assert animation["channels"], (path, animation["name"])
        for channel in animation["channels"]:
            assert 0 <= channel["target"]["node"] < len(nodes), path
        for sampler in animation["samplers"]:
            time = document["accessors"][sampler["input"]]
            assert time["count"] >= 2 and time["max"][0] > 0, path
    for mesh in document.get("meshes", []):
        for primitive in mesh["primitives"]:
            attributes = primitive["attributes"]
            assert document["accessors"][attributes["POSITION"]]["count"] > 0, path
            if "JOINTS_0" in attributes:
                assert "WEIGHTS_0" in attributes, path
    durations = {
        a["name"]: round(
            max(document["accessors"][s["input"]]["max"][0] for s in a["samplers"]), 3
        )
        for a in animations
    }
    print(
        f"{path.name}: {path.stat().st_size / 1_000_000:.1f} MB, {len(nodes)} nodes, {len(document.get('skins', []))} skins, clips {durations}"
    )


def main():
    for name in ("attacker", "defender"):
        validate(
            ROOT / f"assets/generated/{name}.glb",
            CHARACTER_CLIPS,
            {"mixamorig:RightHand", "mixamorig:Head"},
        )
    for name in ("ak_world", "ak_view", "ak_view_soldier", "ak_view_police"):
        validate(
            ROOT / f"assets/generated/{name}.glb",
            WEAPON_CLIPS,
            {"Muzzle", "Magazine", "Bolt", "WeaponGrip"},
        )
    for name in (
        "knife_view",
        "knife_view_soldier",
        "knife_view_police",
        "knife_pose_attacker",
        "knife_pose_defender",
    ):
        validate(
            ROOT / f"assets/generated/{name}.glb",
            {"idle_knife", "draw_knife", "slash_knife"},
            {"KnifeGrip"} if name.startswith("knife_view") else {"mixamorig:RightHand"},
        )
    validate(ROOT / "assets/generated/knife_world.glb", set(), {"KnifeGrip"})
    document, _ = read_glb(ROOT / "assets/generated/dust2.glb")
    assert len(document["materials"]) == 11
    assert all(
        "KHR_materials_unlit" not in m.get("extensions", {})
        for m in document["materials"]
    )
    print(
        "Dust2: all 11 authored surface materials use lit shading; original textures retained"
    )


if __name__ == "__main__":
    main()
