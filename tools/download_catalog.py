"""Download approved Sketchfab sources into assets/models/catalog/.

Requires SKETCHFAB_TOKEN (or SKETCHFAB_API_TOKEN) for the official API.
Exits unsuccessfully when any requested source could not be downloaded.
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

from weapon_catalog import SOURCE, WEAPONS

TOKEN = os.environ.get("SKETCHFAB_TOKEN") or os.environ.get("SKETCHFAB_API_TOKEN") or ""


def fetch(url: str, dest: Path | None = None):
    headers = {"User-Agent": "open-strike-catalog/1.0"}
    if TOKEN:
        headers["Authorization"] = f"Token {TOKEN}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as response:
        data = response.read()
    if dest:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
    return data


def download(key: str, uid: str) -> bool:
    folder = SOURCE / key
    if any(
        path
        for ext in ("*.glb", "*.gltf", "*.fbx", "*.obj", "*.blend")
        for path in folder.rglob(ext)
    ):
        print(f"skip {key}: already present")
        return True
    folder.mkdir(parents=True, exist_ok=True)
    if not TOKEN:
        print(f"fail {key}: configure SKETCHFAB_TOKEN or SKETCHFAB_API_TOKEN")
        return False
    try:
        payload = json.loads(fetch(f"https://api.sketchfab.com/v3/models/{uid}/download"))
    except urllib.error.HTTPError as error:
        print(f"fail {key}: download API {error.code}")
        return False
    except Exception as error:
        print(f"fail {key}: {error}")
        return False
    url = None
    for kind in ("glb", "gltf", "source"):
        entry = payload.get(kind) or {}
        url = entry.get("url")
        if url:
            break
    if not url:
        print(f"fail {key}: no archive URL")
        return False
    archive = folder / "source.zip"
    try:
        fetch(url, archive)
    except Exception as error:
        print(f"fail {key}: archive {error}")
        return False
    try:
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(folder)
    except zipfile.BadZipFile:
        # Some Sketchfab links return a raw GLB.
        raw = archive.read_bytes()
        if raw[:4] == b"glTF":
            (folder / f"{key}.glb").write_bytes(raw)
        else:
            print(f"fail {key}: not a zip or glb")
            return False
    archive.unlink(missing_ok=True)
    print(f"ok {key}")
    return True


if __name__ == "__main__":
    SOURCE.mkdir(parents=True, exist_ok=True)
    wanted = set(sys.argv[1:]) if len(sys.argv) > 1 else None
    if wanted:
        unknown = wanted - {key for key, *_ in WEAPONS}
        if unknown:
            sys.exit(f"Unknown catalog keys: {', '.join(sorted(unknown))}")
    ok = 0
    failed = 0
    for key, uid, *_ in WEAPONS:
        if wanted and key not in wanted:
            continue
        if download(key, uid):
            ok += 1
        else:
            failed += 1
    print(f"sources ready: {ok}; failed: {failed}")
    sys.exit(1 if failed else 0)
