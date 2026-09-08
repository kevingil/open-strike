"""Apply the inspected Dust2 surface material policy; preserve geometry/textures."""

from inspect_assets import ROOT, read_glb, write_glb


def main():
    document, tail = read_glb(ROOT / "assets/maps/de_dust_2/de_dust_2.glb")
    # The source has eleven atlas partitions of masonry, sand, wood and metal.
    # No emissive/sky surfaces are represented by these materials.
    surfaces = {f"part{index}" for index in range(1, 12)}
    assert {material["name"] for material in document["materials"]} == surfaces
    for material in document["materials"]:
        material.get("extensions", {}).pop("KHR_materials_unlit", None)
        pbr = material.setdefault("pbrMetallicRoughness", {})
        pbr.update(metallicFactor=0.0, roughnessFactor=0.95)
    for key in ("extensionsUsed", "extensionsRequired"):
        if key in document:
            document[key] = [
                name for name in document[key] if name != "KHR_materials_unlit"
            ]
    output = ROOT / "assets/generated/dust2.glb"
    output.parent.mkdir(parents=True, exist_ok=True)
    write_glb(output, document, tail)
    print(f"Exported {output}")


if __name__ == "__main__":
    main()
