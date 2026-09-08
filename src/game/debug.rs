//! Opt-in native rendering diagnostics. No OS input injection or external UI automation.
use crate::game::GameState;
use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure);
        if std::env::var_os("CSRS_WEAPON_CAPTURE").is_some() {
            app.add_systems(
                Update,
                capture_weapon_phases.run_if(in_state(GameState::Playing)),
            );
        }
        crate::game::player::contact_diagnostics::install(app);
        if std::env::var_os("CSRS_FAILURE_RECOVERY").is_some() {
            app.add_systems(Update, recover_asset);
        }
        if std::env::var_os("CSRS_LIFECYCLE").is_some() {
            app.add_systems(Update, lifecycle);
        }
        if std::env::var_os("CSRS_CAPTURE").is_some() {
            app.init_resource::<FrameSamples>()
                .add_systems(Update, (sample_frames, sample_actors));
        }
        if std::env::var_os("CSRS_AUTOSTART").is_some() {
            app.add_systems(Startup, |mut next: ResMut<NextState<GameState>>| {
                next.set(GameState::Loading)
            });
        }
        if std::env::var_os("CSRS_OBSERVER").is_some() {
            app.add_systems(
                PostUpdate,
                observer
                    .after(crate::game::player::presentation::cameras)
                    .before(TransformSystem::TransformPropagate),
            );
        }
        if std::env::var_os("CSRS_DEMO").is_some() {
            app.add_systems(
                FixedUpdate,
                demo.in_set(crate::game::game::SimulationSet::Intent)
                    .run_if(in_state(GameState::Playing)),
            );
        }
        if std::env::var_os("CSRS_CAPTURE_KILLS").is_some() {
            app.add_systems(Update, capture_kills.run_if(in_state(GameState::Playing)));
        }
        if std::env::var_os("CSRS_CAPTURE").is_some() {
            app.add_systems(Update, capture)
                .add_systems(OnEnter(GameState::Finished), capture_result);
        }
    }
}
fn capture(
    mut commands: Commands,
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut done: Local<bool>,
    mut exit: EventWriter<AppExit>,
    cameras: Query<(Entity, &Camera, Option<&Camera3d>, Option<&GlobalTransform>)>,
    actors: Query<(Entity, &Transform, &crate::game::matchplay::Combatant)>,
    bots: Query<&crate::game::bots::BotController>,
    nav: Res<crate::game::bots::navigation::Navigation>,
    frames: Res<FrameSamples>,
    windows: Query<&Window>,
    poses: Query<(
        &Name,
        &GlobalTransform,
        Option<&bevy::render::view::RenderLayers>,
        Option<&bevy::render::primitives::Aabb>,
    )>,
) {
    let seconds = std::env::var("CSRS_EXIT_AFTER")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(20.0);
    if !*done
        && time.elapsed_secs()
            > std::env::var("CSRS_CAPTURE_AT")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(seconds - 3.0)
    {
        info!("Diagnostic capture state: {:?}", state.get());
        if let Ok(window) = windows.single() {
            info!(
                "Render size: {}x{}; scale {}",
                window.physical_width(),
                window.physical_height(),
                window.scale_factor()
            );
        }
        let mut timings = frames.0.clone();
        timings.sort_by(f32::total_cmp);
        if !timings.is_empty() {
            info!(
                "Frame timing: {} samples, p50 {:.2}ms, p95 {:.2}ms",
                timings.len(),
                timings[timings.len() / 2],
                timings[timings.len() * 95 / 100]
            );
        }
        for (e, c, kind, t) in &cameras {
            info!(
                "Camera {:?} order {} 3d {} at {:?}",
                e,
                c.order,
                kind.is_some(),
                t.map(|t| t.translation())
            );
        }
        for (e, t, a) in &actors {
            info!(
                "Actor {:?} {:?} health {} at {:?}",
                e, a.team, a.health, t.translation
            );
            if let Ok(bot) = bots.get(e) {
                info!(
                    "Route {} / {}, next {:?}, target {:?}",
                    bot.waypoint,
                    bot.path.len(),
                    bot.path.get(bot.waypoint).map(|&n| nav.positions[n]),
                    bot.target
                );
            }
        }
        for (name, transform, layers, bounds) in &poses {
            if [
                "mixamorig:LeftHandMiddle1",
                "mixamorig:RightHand",
                "Magazine",
            ]
            .contains(&name.as_str())
                && layers.is_some_and(|layers| {
                    layers.intersects(&bevy::render::view::RenderLayers::layer(1))
                })
            {
                info!(
                    "First-person socket {}: {:?}, mesh center {:?}",
                    name,
                    transform.translation(),
                    bounds.map(|bounds| transform.transform_point(bounds.center.into()))
                );
            }
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::env::var("CSRS_CAPTURE").unwrap()));
        *done = true;
    }
    if time.elapsed_secs() > seconds {
        exit.write(AppExit::Success);
    }
}

fn observer(
    mut cameras: Query<
        &mut Transform,
        (
            With<bevy_fps_controller::controller::RenderPlayer>,
            Without<bevy::render::view::RenderLayers>,
            Without<crate::game::player::player::LocalPlayer>,
        ),
    >,
    actors: Query<&Transform, With<crate::game::player::player::LocalPlayer>>,
) {
    let Ok(body) = actors.single() else {
        return;
    };
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(body.translation + Vec3::new(3.0, 1.3, -3.0))
            .looking_at(body.translation + Vec3::Y * 0.25, Vec3::Y);
    }
}
fn demo(
    session: Res<crate::game::matchplay::MatchSession>,
    mut actor: Query<
        (
            &mut bevy_fps_controller::controller::FpsControllerInput,
            &mut crate::game::matchplay::ActorIntent,
        ),
        With<crate::game::player::player::LocalPlayer>,
    >,
) {
    let Ok((mut input, mut intent)) = actor.single_mut() else {
        return;
    };
    let t = session.elapsed;
    input.pitch = 0.0;
    input.yaw = 0.0;
    input.movement = Vec3::ZERO;
    // Leave ammunition for a deterministic manual reload in phase captures.
    let fire_until = if std::env::var_os("CSRS_WEAPON_CAPTURE").is_some() {
        2.0
    } else {
        4.5
    };
    if std::env::var_os("CSRS_KNIFE_DEMO").is_some() {
        use crate::game::{config::WeaponId, weapons::WeaponSelection};
        intent.selection = Some(WeaponSelection::Select(if t < 4.0 || t >= 7.0 {
            WeaponId::DefaultKnife
        } else {
            WeaponId::AK47
        }));
        intent.fire = (2.0..2.15).contains(&t) || (4.5..5.0).contains(&t);
        intent.reload = (6.5..6.7).contains(&t);
        return;
    }
    intent.fire = t > 1.0 && t < fire_until;
    intent.reload = t > 4.5 && t < 4.7;
    input.crouch = t > 9.0 && t < 11.0;
    input.jump = t > 12.0 && t < 12.1;
    if t > 5.0 && t < 8.0 {
        input.movement.x = 1.0;
    }
}

fn configure(
    mut config: ResMut<crate::game::config::GameConfig>,
    mut loadout: ResMut<crate::game::config::PlayerLoadout>,
) {
    if std::env::var("CSRS_MAP").is_ok_and(|s| s == "warehouse") {
        config.map = crate::game::config::MapId::Warehouse;
        config.mode = crate::game::config::GameMode::Freemode;
    }
    if let Ok(seconds) = std::env::var("CSRS_MATCH_SECONDS") {
        config.match_settings.time_limit = seconds
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_secs);
    }
    if let Ok(score) = std::env::var("CSRS_SCORE_LIMIT") {
        config.match_settings.score_limit = score.parse().ok();
    }
    if std::env::var("CSRS_TEAM").is_ok_and(|s| s == "defender") {
        loadout.selected_skin = crate::game::player::skins::SkinId::Police;
    }
}
#[derive(Resource, Default)]
struct FrameSamples(Vec<f32>);
fn sample_actors(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut at: Local<f32>,
    actors: Query<(
        Entity,
        &Transform,
        &crate::game::matchplay::Combatant,
        &crate::game::weapons::WeaponState,
        Option<&crate::game::bots::BotController>,
    )>,
) {
    if *state.get() != GameState::Playing || time.elapsed_secs() - *at < 20.0 {
        return;
    }
    *at = time.elapsed_secs();
    for (entity, transform, actor, weapon, bot) in &actors {
        info!("Walkthrough actor {:?}: {} hp {:.0}, pos {:?}, ammo {}/{}, shots {}, target {:?}, route {:?}", entity, actor.name, actor.health, transform.translation, weapon.magazine, weapon.reserve, weapon.shots, bot.and_then(|b| b.target), bot.map(|b| (b.waypoint, b.path.len())));
    }
}
fn sample_frames(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut frames: ResMut<FrameSamples>,
) {
    if *state.get() == GameState::Playing && time.elapsed_secs() > 3.0 && frames.0.len() < 20000 {
        frames.0.push(time.delta_secs() * 1000.0);
    }
}
#[derive(Default)]
struct Lifecycle {
    stage: u8,
    at: f32,
    elapsed: f32,
    snapshot: Vec<(Entity, Vec3, u32, f32, u64)>,
}
fn lifecycle(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    session: Res<crate::game::matchplay::MatchSession>,
    mut next: ResMut<NextState<GameState>>,
    mut config: ResMut<crate::game::config::GameConfig>,
    actors: Query<
        (Entity, &Transform, &crate::game::weapons::WeaponState),
        With<crate::game::matchplay::Combatant>,
    >,
    mut run: Local<Lifecycle>,
) {
    use crate::game::config::{GameMode, MapId};
    match run.stage {
        0 if *state.get() == GameState::Playing && session.elapsed > 1.5 => {
            next.set(GameState::Paused);
            run.stage = 1;
        }
        1 if *state.get() == GameState::Paused => {
            run.snapshot = actors
                .iter()
                .map(|(e, t, w)| (e, t.translation, w.magazine, w.reload_remaining, w.shots))
                .collect();
            run.elapsed = session.elapsed;
            run.at = time.elapsed_secs();
            run.stage = 2;
        }
        2 if time.elapsed_secs() - run.at > 1.0 => {
            let current: Vec<_> = actors
                .iter()
                .map(|(e, t, w)| (e, t.translation, w.magazine, w.reload_remaining, w.shots))
                .collect();
            assert_eq!(current, run.snapshot, "Paused actor/weapon state changed");
            assert_eq!(session.elapsed, run.elapsed, "Paused clock advanced");
            info!("Lifecycle: pause preserved all actor IDs, positions, ammo, shots, reload timers and match time");
            next.set(GameState::Playing);
            run.stage = 3;
        }
        3 if *state.get() == GameState::Playing && session.elapsed > 3.0 => {
            next.set(GameState::MainMenu);
            run.stage = 4;
            run.at = time.elapsed_secs();
        }
        4 if *state.get() == GameState::MainMenu && time.elapsed_secs() - run.at > 1.0 => {
            assert!(actors.is_empty(), "Actors leaked into menu");
            config.map = MapId::Warehouse;
            config.mode = GameMode::Freemode;
            next.set(GameState::Loading);
            run.stage = 5;
        }
        5 if *state.get() == GameState::Playing && session.elapsed > 2.0 => {
            assert_eq!(actors.iter().count(), 1);
            info!("Lifecycle: warehouse Freemode loaded with one actor");
            next.set(GameState::MainMenu);
            run.at = time.elapsed_secs();
            run.stage = 6;
        }
        6 if *state.get() == GameState::MainMenu && time.elapsed_secs() - run.at > 1.0 => {
            assert!(actors.is_empty());
            config.map = MapId::Dust2;
            config.mode = GameMode::TeamDeathmatch;
            config.match_settings.time_limit = Some(std::time::Duration::from_secs(5));
            next.set(GameState::Loading);
            run.stage = 7;
        }
        7 if *state.get() == GameState::Finished => {
            assert_eq!(actors.iter().count(), 6);
            info!("Lifecycle: timer completed match: {}", session.result);
            next.set(GameState::Loading);
            run.stage = 8;
        }
        8 if *state.get() == GameState::Playing && session.elapsed > 1.0 => {
            assert_eq!(actors.iter().count(), 6);
            assert_eq!(session.score, [0, 0]);
            info!("Lifecycle: restarted clean 3v3 match; walkthrough complete");
            run.stage = 9;
        }
        _ => {}
    }
}

fn recover_asset(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut assets: ResMut<crate::game::assets::GameAssets>,
    server: Res<AssetServer>,
    mut next: ResMut<NextState<GameState>>,
    mut seen: Local<Option<f32>>,
    mut recovered: Local<bool>,
) {
    if *state.get() == GameState::LoadFailed && seen.is_none() {
        *seen = Some(time.elapsed_secs());
        info!("Failure recovery: required missing asset correctly rejected");
    }
    if !*recovered && seen.is_some_and(|t| time.elapsed_secs() - t > 2.0) {
        assets.arms[0] = server.load("generated/ak_view_soldier.glb");
        next.set(GameState::Loading);
        *recovered = true;
    }
    if *recovered && *state.get() == GameState::Playing && seen.is_some() {
        info!("Failure recovery: valid asset loaded and match playable after retry");
        *seen = None;
    }
}

fn capture_result(
    mut commands: Commands,
    session: Res<crate::game::matchplay::MatchSession>,
    frames: Res<FrameSamples>,
) {
    let path = std::path::PathBuf::from(std::env::var("CSRS_CAPTURE").unwrap());
    let result = path.with_file_name(format!(
        "{}-result.png",
        path.file_stem().unwrap().to_string_lossy()
    ));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(result));
    let mut timings = frames.0.clone();
    timings.sort_by(f32::total_cmp);
    info!(
        "Match result: {} {:?} after {:.1}s",
        session.result, session.score, session.elapsed
    );
    if !timings.is_empty() {
        info!(
            "Match frame timing: {} samples, p50 {:.2}ms, p95 {:.2}ms",
            timings.len(),
            timings[timings.len() / 2],
            timings[timings.len() * 95 / 100]
        );
    }
}

/// Capture authored weapon phases against match time, independent of load time.
fn capture_weapon_phases(
    mut commands: Commands,
    weapons: Query<
        &crate::game::weapons::WeaponState,
        With<crate::game::player::player::LocalPlayer>,
    >,
    session: Res<crate::game::matchplay::MatchSession>,
    mut phase: Local<usize>,
) {
    const PHASES: &[(f32, &str)] = &[
        (0.7, "idle"),
        (2.0, "fire"),
        (5.2, "extract"),
        (5.7, "removed"),
        (6.15, "insert"),
        (6.48, "charge"),
        (6.92, "recover"),
        (7.2, "ready"),
    ];
    let phases = if std::env::var_os("CSRS_KNIFE_DEMO").is_some() {
        &[
            (0.1, "knife-draw"),
            (0.7, "knife-idle"),
            (2.16, "knife-contact"),
            (2.5, "knife-recovery"),
            (4.7, "ak-fire"),
            (6.8, "ak-reload"),
            (7.5, "knife-after-cancel"),
        ][..]
    } else {
        PHASES
    };
    let Some(&(at, name)) = phases.get(*phase) else {
        return;
    };
    if session.elapsed >= at {
        let directory = std::env::var("CSRS_WEAPON_CAPTURE").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{directory}/{name}.png")));
        info!(
            "Weapon capture: {} at match time {:.3}",
            name, session.elapsed
        );
        if let Ok(weapon) = weapons.single() {
            info!(
                "Captured equipment: {:?}, ammo {}/{}, reload {:.3}, equips {}, slashes {}",
                weapon.active,
                weapon.magazine,
                weapon.reserve,
                weapon.reload_remaining,
                weapon.equips,
                weapon.slashes
            );
        }
        *phase += 1;
    }
}

/// Capture real kill events after the HUD has received them; never synthesize scores.
fn capture_kills(
    mut commands: Commands,
    mut kills: EventReader<crate::game::matchplay::KillNotice>,
    mut captured: Local<u32>,
) {
    let count = kills.read().count();
    if count == 0 || *captured >= 3 {
        return;
    }
    let Ok(path) = std::env::var("CSRS_CAPTURE") else {
        return;
    };
    *captured += 1;
    let path = std::path::Path::new(&path);
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("match");
    let capture = path.with_file_name(format!("{name}-kill-{}.png", *captured));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(capture));
}
