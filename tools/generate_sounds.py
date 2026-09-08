"""Generate original Open Strike cues using only deterministic noise and oscillators.

No recordings, downloads or third-party sound packs are inputs. Outputs and this
script are covered by the project's MIT license. Python standard library only.
"""

import hashlib
import json
import math
from pathlib import Path
import random
import struct
import wave

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/audio/generated"
RATE = 44100
# Stable gameplay IDs also allow a private pack to override individual cues.
CUES = {
    "weapons/knife_draw": ("knife_draw", 0.25, 31, 720.0, 22.0),
    "weapons/knife_slash": ("knife_slash", 0.30, 32, 180.0, 12.0),
    "weapons/ak47-1": ("rifle_fire", 0.34, 1, 85.0, 22.0),
    "weapons/ak47_draw": ("rifle_draw", 0.24, 2, 480.0, 27.0),
    "weapons/ak47_clipout": ("magazine_out", 0.16, 3, 650.0, 38.0),
    "weapons/ak47_clipin": ("magazine_in", 0.18, 4, 320.0, 32.0),
    "weapons/ak47_boltpull": ("charging_handle", 0.25, 5, 900.0, 28.0),
    **{
        f"physics/flesh_impact_bullet{i}": (
            f"impact_{i}",
            0.12,
            10 + i,
            110.0 + 9 * i,
            42.0,
        )
        for i in range(1, 6)
    },
    **{
        f"physics/drywall_footstep{i}": (
            f"footstep_{i}",
            0.19,
            20 + i,
            70.0 + 8 * i,
            30.0,
        )
        for i in range(1, 5)
    },
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    catalog = {}
    for sound_id, (name, duration, seed, frequency, decay) in CUES.items():
        rng = random.Random(seed)
        samples = []
        low = 0.0
        for index in range(round(duration * RATE)):
            t = index / RATE
            noise = rng.uniform(-1.0, 1.0)
            low += 0.18 * (noise - low)
            envelope = min(t / 0.001, 1.0) * math.exp(-decay * t)
            tone = math.sin(2 * math.pi * frequency * t)
            value = envelope * (0.65 * low + 0.2 * noise + 0.3 * tone)
            # Two separate mechanical transients for the charging handle.
            if name == "charging_handle" and t >= 0.13:
                value += 0.35 * noise * math.exp(-70 * (t - 0.13))
            # Fade the tail to zero to avoid a discontinuity at playback end.
            value *= min((duration - t) / 0.015, 1.0)
            samples.append(value)
        peak = max(abs(value) for value in samples)
        pcm = b"".join(
            struct.pack("<h", round(value / peak * 24575)) for value in samples
        )
        path = OUT / f"{name}.wav"
        with wave.open(str(path), "wb") as sound:
            sound.setnchannels(1)
            sound.setsampwidth(2)
            sound.setframerate(RATE)
            sound.writeframes(pcm)
        catalog[sound_id] = {
            "path": f"audio/generated/{name}.wav",
            "channels": 1,
            "sample_rate": RATE,
            "bits_per_sample": 16,
            "frames": len(samples),
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
    lines = [
        "// Original procedural cues. Regenerate with tools/generate_sounds.py.",
        "{",
    ]
    for sound_id, clip in sorted(catalog.items()):
        fields = ", ".join(f"{key}: {json.dumps(value)}" for key, value in clip.items())
        lines.append(f"    {json.dumps(sound_id)}: ({fields}),")
    (OUT / "catalog.ron").write_text("\n".join([*lines, "}", ""]))
    (OUT / "provenance.json").write_text(
        json.dumps(
            {
                "license": "MIT",
                "generator": "tools/generate_sounds.py",
                "inputs": "Deterministic seeded noise and sine oscillators; no source recordings.",
                "sample_rate": RATE,
                "clips": catalog,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"Generated {len(catalog)} original cues in {OUT}")


if __name__ == "__main__":
    main()
