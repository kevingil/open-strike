"""Resize and recompress images inside a GLB so the file stays under GitHub's limit."""

from __future__ import annotations

import argparse
import struct
import sys
from io import BytesIO
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from inspect_assets import read_glb, write_glb  # noqa: E402
from PIL import Image

DEFAULT_MAX_DIM = 1024
DEFAULT_JPEG_QUALITY = 85


def _load_image(raw: bytes) -> Image.Image:
    image = Image.open(BytesIO(raw))
    image.load()
    return image


def _encode(image: Image.Image, *, name: str, max_dim: int, quality: int) -> tuple[bytes, str]:
    width, height = image.size
    longest = max(width, height)
    if longest > max_dim:
        scale = max_dim / longest
        image = image.resize(
            (max(1, round(width * scale)), max(1, round(height * scale))),
            Image.Resampling.LANCZOS,
        )
    lowered = name.lower()
    keep_alpha = image.mode in {"RGBA", "LA"} and "normal" not in lowered
    out = BytesIO()
    if keep_alpha:
        image = image.convert("RGBA")
        image.save(out, format="PNG", optimize=True)
        return out.getvalue(), "image/png"
    image = image.convert("RGB")
    image.save(out, format="JPEG", quality=quality, optimize=True)
    return out.getvalue(), "image/jpeg"


def cap_glb(path: Path, max_dim: int = DEFAULT_MAX_DIM, quality: int = DEFAULT_JPEG_QUALITY) -> int:
    document, tail = read_glb(path)
    chunk_size, kind = struct.unpack_from("<I4s", tail, 0)
    if kind not in (b"BIN\x00", b"BIN "):
        raise ValueError(f"{path} has no BIN chunk")
    binary = tail[8 : 8 + chunk_size]
    views = document.get("bufferViews", [])
    buffers = document.setdefault("buffers", [{}])
    view_bytes = []
    for view in views:
        start = view.get("byteOffset", 0)
        view_bytes.append(binary[start : start + view["byteLength"]])

    changed = False
    seen: set[int] = set()
    for image in document.get("images", []):
        if "bufferView" not in image:
            continue
        index = image["bufferView"]
        if index in seen:
            continue
        seen.add(index)
        encoded, mime = _encode(
            _load_image(view_bytes[index]),
            name=image.get("name", ""),
            max_dim=max_dim,
            quality=quality,
        )
        if encoded != view_bytes[index] or image.get("mimeType") != mime:
            view_bytes[index] = encoded
            image["mimeType"] = mime
            image.pop("uri", None)
            changed = True

    if not changed:
        return path.stat().st_size

    rebuilt = bytearray()
    for index, payload in enumerate(view_bytes):
        while len(rebuilt) % 4:
            rebuilt.append(0)
        views[index]["byteOffset"] = len(rebuilt)
        views[index]["byteLength"] = len(payload)
        views[index]["buffer"] = 0
        rebuilt.extend(payload)
    while len(rebuilt) % 4:
        rebuilt.append(0)
    buffers[0]["byteLength"] = len(rebuilt)
    bin_chunk = struct.pack("<I4s", len(rebuilt), b"BIN\x00") + bytes(rebuilt)
    write_glb(path, document, bin_chunk)
    return path.stat().st_size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    parser.add_argument("--max-dim", type=int, default=DEFAULT_MAX_DIM)
    parser.add_argument("--quality", type=int, default=DEFAULT_JPEG_QUALITY)
    args = parser.parse_args()
    for path in args.paths:
        before = path.stat().st_size
        after = cap_glb(path, max_dim=args.max_dim, quality=args.quality)
        print(f"{path}: {before / 1e6:.1f}MB -> {after / 1e6:.1f}MB")


if __name__ == "__main__":
    main()
