# Audio packs

`generated/` contains the original MIT-licensed default cues and their provenance.
Rebuild with `python3 tools/generate_sounds.py`; no external inputs are needed.

`csgo/` and `local/` are explicitly ignored by Git. Other audio directories are
trackable by default. Keep private recordings in one of those ignored directories.
When present, `assets/audio/local/catalog.ron` loads automatically. Select a
different catalog with `OPEN_STRIKE_AUDIO_PACK`; this takes precedence over the
local catalog. Use `OPEN_STRIKE_AUDIO_PACK=assets/audio/generated/catalog.ron`
to explicitly select generated cues.
Catalog paths are relative to `assets/`; IDs match `generated/catalog.ron`.
Missing IDs and missing files use generated defaults. An unreadable or malformed
catalog falls back to the complete default pack. Invalid audio encoding remains
an asset-loading error; use `tools/index_sounds.py` to validate your local WAVs.

This override is a development convenience, not a grant of redistribution rights.

The local knife equip override maps `weapons/knife_draw` to the downloaded
`audio/csgo/weapons/deploy1.wav`. It is a partial catalog, so other cues retain
their default selections. The CS:GO catalog also includes this explicit alias
for launches that select that pack.

The default local walking cue is `misc/step_test_loop`, mapped to
`audio/csgo/misc/step_test_loop.wav`. It plays as one loop per grounded, moving
actor and stops when movement stops, the actor jumps, or the actor dies. Packs
without this optional cue retain the generated individual-step fallback.
