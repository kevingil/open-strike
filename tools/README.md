# Tools and development

Build instructions, asset scripts, audio workflows, and native diagnostics for [Open Strike](../README.md).

Run all commands below from the repository root. Generated art and default audio are included; rebuilding them is optional.

## Run the game

```sh
cargo run --locked --bin open-strike
```

## Build

```sh
cargo build --locked --bin open-strike
```

`rust-toolchain.toml` pins `nightly-2026-01-02` (Rust 1.94 nightly), matching the compiler used for the locked build. Keep Cargo.lock. The stack remains Bevy 0.16.1, Rapier integration 0.30 and bevy_fps_controller 0.16. macOS builds require Apple's command-line developer tools and a working Metal device. The first build needs network access for the pinned compiler and locked crates.

Run from the repository so `assets/` resolves. Generated GLBs are included; Blender is only required when rebuilding art. Development uses Bevy dynamic linking, so prefer `cargo run` rather than launching the bare binary without Cargo's library environment. Release packaging and other platforms have not been validated.

## Ownership and extension points

- `src/game/config.rs` owns settings, supported loadouts, map choice and match defaults; menus edit these resources.
- `src/game/level/level.rs` owns map loading, required asset validation, collision readiness and recoverable loading errors. Pause preserves the session. Menu/restart removes session entities.
- `src/game/player/input.rs` and `src/game/bots/` produce per-actor controller/weapon intents. Movement, Rapier, combat and match rules run in an explicit 60 Hz order.
- `src/game/matchplay/` owns health, armor, teams, deaths, score, spawn protection and respawn. `src/game/weapons/` owns accepted shots, cadence, ammo, reload, hit queries and presentation events.
- `src/game/player/animation.rs` binds named per-skin clips and layers a rifle upper-body pose over locomotion. Cosmetic meshes follow logical bodies; damage zones belong to the logical actors. First-person arms/weapon use a separate camera and render layer.
- `src/game/ui/` reads state for the HUD. Radar derives from map navigation and authored callouts. Sound and visual effects consume accepted gameplay actions.
- Human networking is v2: add stable network IDs, remote intent validation, authoritative snapshots and prediction/reconciliation at these boundaries. Current Bevy entity IDs and local events are not a network protocol.

Dust2 uses authored spawns, callouts and patrol destinations. A collision-checked graph is generated at load time, including floor layers under bridges. Bots use graph paths, line of sight, reaction delay and local passing behavior; they fire, reload and respawn through the same systems as the human.

## Rebuild generated art

Original models and animation sources are preserved. Do not use the old scene-mutating repair scripts as an export workflow. The supported pipeline requires Blender **4.2 LTS**:

```sh
blender --background --factory-startup --python tools/export_assets.py -- characters
blender --background --factory-startup --python tools/export_assets.py -- weapon
python3 tools/export_map.py
python3 tools/inspect_assets.py
```

On macOS, replace `blender` with `/Applications/Blender.app/Contents/MacOS/Blender` if it is not on PATH. Export characters before weapons. Outputs go to `assets/generated/`; required character action bindings are recorded in `assets/config/character_clips.json`.

The exporter bakes the donor's evaluated motion onto each target's own rest skeleton, authors grounded crouch/death variants, aligns the support hand, limits character textures to 2K, and exports separate world/first-person AK scenes. Weapon sockets are `WeaponGrip`, `Muzzle`, `Magazine` and `Bolt`; first-person sleeves retain their length and the rifle retains its stock. First-person reload uses weapon-space magazine insertion from below, a charging-handle pull, and a recovery pose matching idle. Body materials use a declared cloth/skin/painted-armor surface policy instead of the imported metallic response. Inspection checks scene/clip/socket/material contracts, skin joint counts and finite geometry/animation values. Blender-only imports are explicitly marked for static checking; actual export execution validates those APIs.

World convention: meters, +Y up, camera forward -Z. Imported characters face +Z and receive the declared visual yaw correction. Standing body height is 1.8 m, crouch height 1.4 m, radius 0.3 m. Map spawn/callout/navigation positions are map-local and receive the configured transform once. Cosmetic skeleton units never determine hitbox size.

The menu has its own showcase scale and framing. Those display transforms are deliberately separate from gameplay bodies.

### Script guide

| Script | Purpose |
| --- | --- |
| `tools/export_assets.py` | Export characters, world weapons, and menu animation clips. |
| `tools/export_map.py` | Prepare the Dust 2 map's surface materials. |
| `tools/export_menu.py` | Export the menu scene, map card, and rifle icon. |
| `tools/export_inventory.py` | Render weapon previews for the inventory. |
| `tools/export_knife.py` | Generate the default knife, character clips, portraits, and HUD icons. |
| `tools/export_viewmodels.py` | Rebuild first-person rifle and knife variants after the base exports. |
| `tools/export_rifle_view.py` | Rebuild only the first-person AK variants. |
| `tools/export_reference_knife.py` | Rebuild both knife choices and reference knife icons. |
| `tools/inspect_assets.py` | Inspect generated model and animation contracts. |
| `tools/generate_sounds.py` | Regenerate the bundled procedural sound effects. |
| `tools/index_sounds.py` | Index optional local WAV recordings. |

## Audio

The game ships with original, procedurally synthesized sound effects for firing,
drawing the rifle, magazine removal/insertion, the charging handle, impacts and
footsteps. They work on a fresh checkout with no downloads or private sound pack.
The generator uses seeded noise and oscillators, without commercial recordings or
sampled audio. Both the script and generated cues are MIT-licensed. These are
functional prototype effects; contributors can replace them with higher-quality
original recordings or assets with verified redistribution licenses.

```sh
# Optional: regenerate the bundled defaults using Python's standard library.
python3 tools/generate_sounds.py
```

`assets/audio/generated/catalog.ron` maps stable gameplay IDs to WAV files.
`provenance.json` records the generation method and checksums. `SoundLibrary`
loads only requested cues. Local cues are non-spatial; other actors use positional
audio. The local default walking sound is `audio/csgo/misc/step_test_loop.wav`,
played as a single loop while an actor moves on the ground. Packs without that
optional cue retain the generated individual-step fallback. Surface-specific
footsteps are not yet implemented.

### Optional local recordings

Keep personal recordings under `assets/audio/local/`, which Git ignores. Give each
WAV the relative filename matching a catalog ID—for example,
`assets/audio/local/weapons/ak47-1.wav`. The supported import format is mono/stereo,
8- or 16-bit PCM WAV. Index existing files; the local catalog loads automatically:

```sh
python3 tools/index_sounds.py assets/audio/local
OPEN_STRIKE_AUDIO_PACK=assets/audio/local/catalog.ron cargo run --locked --bin open-strike
```

Partial packs override only matching IDs; missing files keep generated defaults.
`OPEN_STRIKE_AUDIO_PACK` takes precedence over the automatic local catalog.
Set it to `assets/audio/generated/catalog.ron` to explicitly use generated cues.
An unreadable or malformed catalog falls back to the default pack. The existing
private pack under `assets/audio/csgo/` is also ignored and can be used locally:

```sh
OPEN_STRIKE_AUDIO_PACK=assets/audio/csgo/catalog.ron cargo run --locked --bin open-strike
```

This preserves local files without requiring them to build or run the game.
Private packs must not be added to Git, release bundles or public demos without
appropriate permission. `.gitignore` prevents ordinary accidental additions but
does not remove previously committed files from history or prevent `git add -f`.

## Native diagnostics

These opt-in environment flags exercise the real application without operating the desktop:

```sh
CSRS_AUTOSTART=1 cargo run --locked --bin open-strike
CSRS_AUTOSTART=1 CSRS_DEMO=1 CSRS_CAPTURE=/tmp/csrs.png CSRS_EXIT_AFTER=18 cargo run --locked --bin open-strike
CSRS_AUTOSTART=1 CSRS_LIFECYCLE=1 CSRS_CAPTURE=/tmp/lifecycle.png CSRS_EXIT_AFTER=32 cargo run --locked --bin open-strike
```

`CSRS_DEMO` supplies a short fire/reload/move/crouch/jump sequence through the normal intent path. `CSRS_LIFECYCLE` checks pause preservation, menu cleanup, Warehouse practice, timer completion and restart. `CSRS_CAPTURE` uses Bevy's native screenshot path, opens a 1280×720 diagnostic window, reports frame-time samples, and exits after the requested duration. It does not change the normal fullscreen menu.

`CSRS_BOT_PLAYER=1` lets the normal bot controller drive the local actor for a full-match walkthrough. `CSRS_MATCH_SECONDS` and `CSRS_SCORE_LIMIT` shorten that walkthrough. `CSRS_INVALID_ASSET=1` deliberately requests a missing required asset; `CSRS_FAILURE_RECOVERY=1` replaces that diagnostic reference and retries. `CSRS_DEBUG=1` enables the inherited scene inspection tools. Observer cameras, when enabled for asset inspection, are development diagnostics only.

### Capture the README walkthrough

The existing menu scenario walks through Home, Inventory, Load Out, map selection,
and a live match using the game's own handlers and native screenshots. It also
checks pause, return, and restart. No desktop automation is needed.

```sh
RUST_LOG=info OPEN_STRIKE_AUDIO_PACK=assets/audio/generated/catalog.ron \
CSRS_MENU_SCENARIO=1 CSRS_CAPTURE=/private/tmp/open-strike-walkthrough.png \
CSRS_WIDTH=1600 CSRS_HEIGHT=900 CSRS_EXIT_AFTER=90 \
cargo run --locked --bin open-strike
```

After the log reports `MENU_SCENARIO passed`, copy the walkthrough captures:

```sh
mkdir -p docs/screenshots
cp /private/tmp/csrs-menu-home.png docs/screenshots/menu.png
cp /private/tmp/csrs-menu-inventory.png docs/screenshots/inventory.png
cp /private/tmp/csrs-menu-loadout.png docs/screenshots/loadout.png
cp /private/tmp/csrs-menu-play.png docs/screenshots/maps.png
cp /private/tmp/csrs-menu-hud.png docs/screenshots/gameplay-dust2.png
```

For an individual screen, set `CSRS_CAPTURE` to its output PNG path and optionally
set one of `CSRS_CAPTURE_INVENTORY=1`, `CSRS_CAPTURE_LOADOUT=1`, or
`CSRS_CAPTURE_PLAY=1`. With none of these flags, the app opens Home.
`CSRS_CAPTURE_AT` chooses the capture time in seconds after startup;
`CSRS_EXIT_AFTER` must leave time for the image to save.

## Menu assets and diagnostics

The menu loads `generated/menu/dust2.glb`, a separately exported A-site scene with no navigation or collision dependency. The complete gameplay map loads only when starting a match. Both character rigs and first-person arms are still eagerly requested at startup; background size is not total menu residency.

```sh
blender --background --factory-startup --python tools/export_menu.py -- scene
blender --background --factory-startup --python tools/export_menu.py -- icon
blender --background --factory-startup --python tools/export_assets.py -- menu
```

`scene` writes the editable `assets/menu/dust2.blend`, standalone GLB, portrait card and size manifest. It selects an explicitly bounded A-site area, applies the measured floor anchor and caps its textures at 1024 pixels. `icon` renders the existing rifle from an orthographic side view. `menu` adds/replaces the four-second `menu_hold_rifle` clip while preserving gameplay channels and meshes. A full `characters` export also authors the menu clip. The UI bundles [Roboto Condensed from Google Fonts](https://github.com/google/fonts/tree/main/ofl/robotocondensed); its license is in `assets/fonts/OFL.txt`.

```sh
RUST_LOG=info CSRS_MENU_SCENARIO=1 CSRS_CAPTURE=/tmp/menu.png CSRS_EXIT_AFTER=65 cargo run --locked --bin open-strike
```

This opt-in native scenario exercises menu handlers, equipping, Play, 3v3 loading, pause/resume, return and restart. Screenshots go to `/private/tmp/csrs-menu-*.png`. It uses in-engine input state, not OS input injection; physical pointer hover remains a manual check. `CSRS_MENU_MISSING=1` checks cosmetic-scene failure. With `CSRS_CAPTURE` set, `CSRS_WIDTH` and `CSRS_HEIGHT` select a fixed diagnostic window size at startup. Use a fresh launch per size; changing window resolution programmatically during rendering is not part of this diagnostic.

### Default knife and HUD assets

Generate the authored knife, first-person gloves, world attachment, character
upper-body clips, portraits, and weapon/headshot icons with Blender 4.2:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/export_knife.py
```

The editable source is `assets/models/weapons/default_knife.blend`. The exporter
reads the existing AK arm and character assets without modifying them. Knife
clips and the `KnifeGrip` socket are checked before match loading completes.

The existing native weapon-capture workflow supports `CSRS_KNIFE_DEMO=1` together
with `CSRS_DEMO=1` and `CSRS_WEAPON_CAPTURE=<directory>`. It captures draw, idle,
slash contact/recovery, AK fire/reload, and switching away from an unfinished
reload through normal weapon intents. This is opt-in rendering diagnostics.

### Dedicated first-person arms

AK first-person views use the supplied **AKM reload animation** model by Vlasov
Daniil: its rifle, gloves, sleeve mesh, twist joints and original reload channels.
Knife views use the separate DJMaesen reference. Both select a tint from `SkinId`;
world characters and their AK attachments retain the existing assets.

The AK camera transform and 55-degree vertical FOV stay fixed throughout reload.
The source's 4.35-second motion is retimed to the existing 2.5-second gameplay
reload. Idle holds the source entry pose; fire adds a short whole-rig recoil.
Local magazine sounds follow the reference; world reload sounds remain unchanged.

Rebuild these variants **after** the base AK/knife exporters:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/export_viewmodels.py
```

`tools/export_rifle_view.py` converts the AKM GLB directly, preserving its weights,
inverse bind matrices and animation samples. It repairs the supplied arms' baked
whole-mesh scale without stretching individual hand/finger segments. To rebuild
only AK views, run it with Blender as above. Its optional `-- --existing-rifle`
argument fits the older AK mesh and two magazine copies to the reference rig;
the default retains the complete reference model to match its proportions.
Knife variants preserve the supplied model's native skeleton, skin weights, wrist
poses and animated knife attachment. `tools/export_reference_knife.py` exports
both knife choices for Soldier and Police, plus the reference blade's world model
and inventory/HUD icons. Its explicit source ranges are idle frames 0–40, slash
60–80 and draw 123–145; slash timing matches the gameplay contact window.

Choose **Default Knife** or **Reference Knife** under **Load Out → Equipment →
Knife · Both teams**. The selection applies to both characters, survives respawn,
and is equipped with `3` (or previous weapon). Weapon damage and reach are shared.
To regenerate just the knife assets:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/export_reference_knife.py
```

Add `CSRS_REFERENCE_KNIFE=1` to the native knife capture command to inspect the
reference blade, and `CSRS_TEAM=defender` to inspect Police. See
[asset licensing and provenance](../ASSET_LICENSES.md) for attribution and modifications.
