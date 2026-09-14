# Audio packs

`generated/` contains the original MIT-licensed default cues and their provenance.
Rebuild with `python3 tools/generate_sounds.py`; no external inputs are needed.

`csgo/` and `local/` are explicitly ignored by Git. Other audio directories are
trackable by default. Keep private recordings in one of those ignored directories.
When present, `assets/audio/local/catalog.ron` loads automatically. Select a
different catalog with `OPEN_STRIKE_AUDIO_PACK`; this takes precedence over the
local catalog. Use `OPEN_STRIKE_AUDIO_PACK=assets/audio/generated/catalog.ron`
to explicitly select generated cues.
Catalog paths are relative to `assets/`. Packs may add IDs beyond
`generated/catalog.ron`; weapon bindings live in
`src/game/weapons/audio_bindings.rs` and preserve the downloaded filenames.
Missing IDs and missing files use generated defaults. An unreadable or malformed
catalog falls back to the complete default pack. Invalid audio encoding remains
an asset-loading error; use `tools/index_sounds.py` to validate your local WAVs.

This override is a development convenience, not a grant of redistribution rights.

The restored local pack contains the WAVs from `~/Downloads/weapons/` under
`local/weapons/`, plus impact and walking cues from the existing private pack.
`local/weapons/knife_draw.wav` is a copy of `deploy1.wav` and
`local/weapons/knife_slash.wav` is a copy of `slash1.wav`; these explicit aliases
survive reindexing with `python3 tools/index_sounds.py assets/audio/local`.
The local catalog and recordings remain ignored by Git.

The default local walking cue is `misc/step_test_loop`, mapped to
`audio/local/misc/step_test_loop.wav`. It plays as one loop per grounded, moving
actor and stops when movement stops, the actor jumps, or the actor dies. Packs
without this optional cue retain the generated individual-step fallback.

All 33 guns have explicit fire, equip and reload bindings. The downloaded CZ75
has only fire recordings, so it shares P250 handling; guns without a dedicated
draw recording use their slide/bolt cue (Negev uses `movement1`). AK retains its
authored first-person reload timings. Other reload cues are staged across the
simulation duration, including shells, dual magazines and belt handling.
Network protocol 2 carries the firing weapon with each shot; restart match
servers along with clients after updating.
