"""Apply the inspected map surface material policy; preserve geometry/textures."""

from inspect_assets import ROOT, read_glb, write_glb

MAPS = {
    "dust2": {
        "source": ROOT / "assets/maps/de_dust_2/de_dust_2.glb",
        "output": ROOT / "assets/generated/dust2.glb",
        "surfaces": {f"part{index}" for index in range(1, 12)},
    },
    "mirage": {
        "source": ROOT / "assets/maps/de_mirage/de_mirage.glb",
        "output": ROOT / "assets/generated/mirage.glb",
        "surfaces": None,
    },
}


def export(name):
    spec = MAPS[name]
    document, tail = read_glb(spec["source"])
    names = {material["name"] for material in document["materials"]}
    if spec["surfaces"] is not None:
        assert names == spec["surfaces"]
    for material in document["materials"]:
        material.get("extensions", {}).pop("KHR_materials_unlit", None)
        pbr = material.setdefault("pbrMetallicRoughness", {})
        pbr.update(metallicFactor=0.0, roughnessFactor=0.95)
    for key in ("extensionsUsed", "extensionsRequired"):
        if key in document:
            document[key] = [
                item for item in document[key] if item != "KHR_materials_unlit"
            ]
    spec["output"].parent.mkdir(parents=True, exist_ok=True)
    write_glb(spec["output"], document, tail)
    print(f"Exported {spec['output']}")


def main():
    import sys

    targets = sys.argv[1:] or list(MAPS)
    for name in targets:
        export(name)


if __name__ == "__main__":
    main()
