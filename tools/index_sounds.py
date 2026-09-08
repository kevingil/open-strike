"""Index already available PCM WAVs in an ignored, local-only audio directory.

Names relative to the directory become sound IDs (weapons/ak47-1.wav maps to
weapons/ak47-1). This does not download or copy sound recordings.
"""

import argparse
import hashlib
import json
from pathlib import Path
import wave

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    directory = args.directory.resolve()
    private_root = (ASSETS / "audio/local").resolve()
    if directory != private_root and private_root not in directory.parents:
        parser.error("Use assets/audio/local or a subdirectory of it")
    files = sorted(directory.rglob("*.wav"))
    if not files:
        parser.error("No WAV files found; existing catalog was not changed")
    lines = [
        "// Local-only sound overrides. Do not distribute without appropriate rights.",
        "{",
    ]
    for path in files:
        if path.is_symlink():
            parser.error(f"Symbolic links are not supported: {path}")
        with wave.open(str(path), "rb") as sound:
            frames = sound.getnframes()
            channels = sound.getnchannels()
            width = sound.getsampwidth()
            if (
                sound.getcomptype() != "NONE"
                or width not in (1, 2)
                or channels not in (1, 2)
            ):
                parser.error(f"Expected mono/stereo 8- or 16-bit PCM WAV: {path}")
            if len(sound.readframes(frames)) != frames * channels * width:
                parser.error(f"Truncated WAV: {path}")
            clip = {
                "path": path.relative_to(ASSETS).as_posix(),
                "channels": channels,
                "sample_rate": sound.getframerate(),
                "bits_per_sample": width * 8,
                "frames": frames,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        sound_id = path.relative_to(directory).with_suffix("").as_posix()
        fields = ", ".join(f"{key}: {json.dumps(value)}" for key, value in clip.items())
        lines.append(f"    {json.dumps(sound_id)}: ({fields}),")
    destination = directory / "catalog.ron"
    destination.write_text("\n".join([*lines, "}", ""]))
    print(f"Indexed {len(files)} local recordings in {destination}")


if __name__ == "__main__":
    main()
