# Open Strike

An open-source tactical first-person shooter built with Rust and Bevy. The current prototype supports offline team deathmatch with bots on Dust 2.

## Play

```sh
cargo run --locked --bin open-strike
```

Open **Play**, select **Deathmatch / Dust 2**, then click **Start Game**. Inventory browses available equipment; **Load Out** equips Soldier or Police, selecting the corresponding team and world character. Both teams carry an AK-47 and one Default Knife. Bots currently use the AK. Home shows an animated character in a separate A-site scene viewed from Long A. Hover the right profile rail to open Friends; click pins it, Tab focuses it, and Escape closes it. The local placeholder shows **no friends**. Human networking remains v2.

- WASD: move; mouse: aim; hold left mouse: AK automatic fire or repeated knife slashes.
- 1: AK-47; 3: Default Knife; F: previous weapon. Q is reserved for the future circular inventory picker. Switching cancels an unfinished reload and preserves AK ammunition.
- R: reload; Space: jump; Left Shift: sprint; Left Ctrl: crouch.
- Esc: pause/resume. Losing window focus pauses the local match.
- At the result screen: Enter restarts; Esc returns to the menu.

Gameplay is first person only. The HUD uses a rotating circular radar, a live TDM portrait ranking ordered by individual kills, team scores, match clock, health/armor, ammunition, two weapon slots, and an icon-based kill feed. Dead participants remain in the ranking with dimmed portraits. Knife selection hides ammunition. Kill notices use team colors and show headshots only for lethal AK head hits. Radar shows living teammates, not hidden enemies. C4, bomb objectives, economy and networked 5v5 are outside this version.

Default TDM: one human and two bots against three bots; 10 minutes or 50 team kills; 3-second respawn; 2-second spawn protection that ends when firing; no friendly damage (teammates still block bullets). AK: 30-round magazine, 120 reserve, 600 RPM, 2.5-second reload. Armor absorbs 25% of incoming damage while available and resets on respawn. Knife: 1.5 m reach, 40 base damage, 0.15-second windup, 0.40-second recovery, and 0.35-second equip. A strike hits one target at most; walls, teammates and spawn protection block damage. These are prototype tuning values, not an exact CS:GO ballistics simulation.

## Build

```sh
cargo build --locked --bin open-strike
```

`rust-toolchain.toml` pins `nightly-2026-01-02` (Rust 1.94 nightly), matching the compiler used for the locked build. Keep Cargo.lock. The stack remains Bevy 0.16.1, Rapier integration 0.30 and bevy_fps_controller 0.16. macOS builds require Apple's command-line developer tools and a working Metal device. The first build needs network access for the pinned compiler and locked crates.

Run from the repository so `assets/` resolves. Generated GLBs are included; Blender is only required when rebuilding art. Development uses Bevy dynamic linking, so prefer `cargo run` rather than launching the bare binary without Cargo's library environment. Release packaging and other platforms have not been validated.

## Ownership and extension points

- `game/config.rs` owns settings, supported loadouts, map choice and match defaults; menus edit these resources.
- `level/level.rs` owns map loading, required asset validation, collision readiness and recoverable loading errors. Pause preserves the session. Menu/restart removes session entities.
- `player/input.rs` and `bots/` produce per-actor controller/weapon intents. Movement, Rapier, combat and match rules run in an explicit 60 Hz order.
- `matchplay/` owns health, armor, teams, deaths, score, spawn protection and respawn. `weapons/` owns accepted shots, cadence, ammo, reload, hit queries and presentation events.
- `player/animation.rs` binds named per-skin clips and layers a rifle upper-body pose over locomotion. Cosmetic meshes follow logical bodies; damage zones belong to the logical actors. First-person arms/weapon use a separate camera and render layer.
- `ui/` reads state for the HUD. Radar derives from map navigation and authored callouts. Sound and visual effects consume accepted gameplay actions.
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

## Audio

The game ships with 14 original, procedurally synthesized sound effects for firing,
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

## License and publication status

Open Strike's original code and generated default sounds use the [MIT license](LICENSE).
Third-party assets retain their own licenses; see [asset licensing and provenance](ASSET_LICENSES.md).
The MIT license does **not** relicense imported maps, models, animations or recordings.

Audio defaults are now independent of private downloads. The full art collection
still needs a provenance and redistribution-rights review before publishing the
complete game, including Dust 2 and its derived menu assets. Keep the existing
credits below while verifying or replacing those assets.

## Native diagnostics

These opt-in environment flags exercise the real application without operating the desktop:

```sh
CSRS_AUTOSTART=1 cargo run --locked --bin open-strike
CSRS_AUTOSTART=1 CSRS_DEMO=1 CSRS_CAPTURE=/tmp/csrs.png CSRS_EXIT_AFTER=18 cargo run --locked --bin open-strike
CSRS_AUTOSTART=1 CSRS_LIFECYCLE=1 CSRS_CAPTURE=/tmp/lifecycle.png CSRS_EXIT_AFTER=32 cargo run --locked --bin open-strike
```

`CSRS_DEMO` supplies a short fire/reload/move/crouch/jump sequence through the normal intent path. `CSRS_LIFECYCLE` checks pause preservation, menu cleanup, Warehouse practice, timer completion and restart. `CSRS_CAPTURE` uses Bevy's native screenshot path, opens a 1280×720 diagnostic window, reports frame-time samples, and exits after the requested duration. It does not change the normal fullscreen menu.

`CSRS_BOT_PLAYER=1` lets the normal bot controller drive the local actor for a full-match walkthrough. `CSRS_MATCH_SECONDS` and `CSRS_SCORE_LIMIT` shorten that walkthrough. `CSRS_INVALID_ASSET=1` deliberately requests a missing required asset; `CSRS_FAILURE_RECOVERY=1` replaces that diagnostic reference and retries. `CSRS_DEBUG=1` enables the inherited scene inspection tools. Observer cameras, when enabled for asset inspection, are development diagnostics only.

Current delivery evidence and remaining polish are recorded in `.plans/de-dust2-implementation.md`.

### Credits

- "Police_ru combat online" (https://skfb.ly/oG86L) by Am I dead? is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

- "Soldier_1 Combat Online" (https://skfb.ly/oGrZC) by Am I dead? is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

- "Offlow Field Map" (https://skfb.ly/p7KLB) by Shiro Morturn is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

- "Armory" (https://skfb.ly/6DxAW) by RyanMurphyLucas is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

- "Ak 47" (https://skfb.ly/6yrzJ) by jeferson is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

- "Warehouse fbx model" (https://skfb.ly/pEKFT) by mason_roman's helloneighborfangamingmodelworks is licensed under Creative Commons Attribution (http://creativecommons.org/licenses/by/4.0/).

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

Soldier and Police now use separate first-person sleeve/glove variants derived
from the supplied DJMaesen arm model, selected by the equipped `SkinId` for both
AK and knife. The full-body character mesh is not rendered as first-person arms.
The existing AK fire, magazine, charging-handle and reload wrist motion is
retained. First-person elbow placement is authored separately; world characters
and weapon attachments keep their existing assets.

Rebuild these variants **after** the base AK/knife exporters:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup --python tools/export_viewmodels.py
```

The base `ak_view.glb` and `knife_view.glb` remain motion inputs. The new exporter
rebuilds all four character/weapon variants without overwriting those inputs or
the supplied reference. Its explicit bone mapping retains reference UVs and
textures, fits gloves to the existing weapon contacts, continues the open upper
sleeves behind the camera, and restores head-relative weapon sockets after the
Blender round trip. See `ASSET_LICENSES.md` for attribution and modifications.
