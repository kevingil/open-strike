//! Dedicated match server. Runs the core simulation headless, admits players with hub
//! tickets, applies their reported inputs and broadcasts snapshots every tick.
use super::{
    proto::*,
    NetRole,
};
use crate::game::{
    assets::GameAssets,
    bots::BotController,
    config::{GameConfig, GameMode, MapId, PlayerSettings, WeaponId},
    game::SimulationSet,
    level::level::LoadedGameplayMapConfig,
    matchplay::{ActorIntent, Combatant, KillNotice, MatchSession, Team},
    player::player::{spawn_actor, team_skin, ActorKind, ActorSpawn, PlayerEntity},
    player::player_model::PlayerModel,
    weapons::{ShotFired, WeaponSelection, WeaponState},
    GameState,
};
use bevy::{app::ScheduleRunnerPlugin, prelude::*, window::ExitCondition, winit::WinitPlugin};
use bevy_fps_controller::controller::{FpsController, FpsControllerInput};
use bevy_rapier3d::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

#[derive(Resource, Clone, Debug)]
pub struct ServerArgs {
    pub hub: Option<String>,
    pub server_id: String,
    pub port: u16,
    pub advertise: String,
    pub mode: GameMode,
    pub map: MapId,
    pub bots: usize,
    pub max_players: usize,
    pub key: String,
    pub time_limit_secs: u64,
    pub score_limit: u32,
}

impl ServerArgs {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let port = value("--port").and_then(|v| v.parse().ok()).unwrap_or(27015);
        Self {
            hub: value("--hub"),
            server_id: value("--server-id").unwrap_or_else(|| format!("srv-{port}")),
            port,
            advertise: value("--advertise").unwrap_or_else(|| "127.0.0.1".into()),
            mode: value("--mode")
                .and_then(|m| GameMode::from_key(&m))
                .unwrap_or(GameMode::Deathmatch),
            map: value("--map")
                .and_then(|m| MapId::from_key(&m))
                .unwrap_or(MapId::Dust2),
            bots: value("--bots").and_then(|v| v.parse().ok()).unwrap_or(6),
            max_players: value("--max-players")
                .and_then(|v| v.parse().ok())
                .unwrap_or(12),
            key: std::env::var("STRIKE_SERVER_KEY").unwrap_or_default(),
            time_limit_secs: value("--time-limit").and_then(|v| v.parse().ok()).unwrap_or(600),
            score_limit: value("--score-limit").and_then(|v| v.parse().ok()).unwrap_or(40),
        }
    }
}

struct ClientConn {
    net_id: u32,
    entity: Option<Entity>,
    name: String,
    account_id: i64,
    team: Team,
    slot: usize,
    last_seen: f64,
    latest: Option<PlayerInput>,
    last_seq: u32,
    welcomed: bool,
}

#[derive(Resource)]
pub struct ServerNet {
    socket: UdpSocket,
    clients: HashMap<SocketAddr, ClientConn>,
    next_net_id: u32,
    next_slot: usize,
    events: VecDeque<NetEvent>,
    event_base: u32,
    tick: u32,
    buffer: Vec<u8>,
    finished_at: Option<f64>,
    reported: bool,
}

impl ServerNet {
    fn push_event(&mut self, event: NetEvent) {
        self.events.push_back(event);
        while self.events.len() > EVENT_HISTORY {
            self.events.pop_front();
            self.event_base = self.event_base.wrapping_add(1);
        }
    }
    fn allocate_net_id(&mut self) -> u32 {
        let id = self.next_net_id;
        self.next_net_id += 1;
        id
    }
    fn send(&self, addr: SocketAddr, message: &ServerMessage) {
        let _ = self.socket.send_to(&encode(message), addr);
    }
}

#[derive(Resource)]
struct Heartbeat(Timer);

pub struct NetServerPlugin;
impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        let args = ServerArgs::parse();
        app.insert_resource(args)
            .insert_resource(NetRole::Server)
            .insert_resource(Heartbeat(Timer::from_seconds(5.0, TimerMode::Repeating)))
            .add_systems(Startup, start)
            .add_systems(OnEnter(GameState::Playing), spawn_bots)
            .add_systems(
                FixedUpdate,
                (receive, spawn_pending_remotes)
                    .chain()
                    .in_set(SimulationSet::Intent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                apply_remote_inputs
                    .after(SimulationSet::Movement)
                    .before(PhysicsSet::SyncBackend)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                broadcast
                    .in_set(SimulationSet::Match)
                    .after(crate::game::matchplay::update_match)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    heartbeat,
                    finished.run_if(in_state(GameState::Finished)),
                    receive_idle.run_if(not(in_state(GameState::Playing))),
                ),
            )
            .add_systems(OnEnter(GameState::LoadFailed), load_failed);
    }
}

fn start(
    mut commands: Commands,
    args: Res<ServerArgs>,
    mut config: ResMut<GameConfig>,
    mut next: ResMut<NextState<GameState>>,
) {
    config.mode = args.mode.clone();
    config.map = args.map.clone();
    config.match_settings.time_limit = Some(Duration::from_secs(args.time_limit_secs));
    config.match_settings.score_limit = Some(args.score_limit);
    let socket = UdpSocket::bind(("0.0.0.0", args.port)).expect("bind match port");
    socket.set_nonblocking(true).unwrap();
    info!(
        "strike-server {} listening on udp/{} ({} on {}, {} bots, {} seats)",
        args.server_id,
        args.port,
        args.mode.name(),
        args.map.name(),
        args.bots,
        args.max_players
    );
    commands.insert_resource(ServerNet {
        socket,
        clients: HashMap::new(),
        next_net_id: 1,
        next_slot: 0,
        events: VecDeque::new(),
        event_base: 0,
        tick: 0,
        buffer: vec![0; MAX_DATAGRAM],
        finished_at: None,
        reported: false,
    });
    if let Some(hub) = &args.hub {
        let body = serde_json::json!({
            "key": args.key, "server_id": args.server_id, "host": args.advertise, "port": args.port,
            "mode": args.mode.key(), "map": args.map.key(), "max_players": args.max_players,
        });
        match ureq::post(&format!("{hub}/v1/servers/register")).send_json(body) {
            Ok(_) => info!("registered with hub {hub}"),
            Err(e) => error!("hub registration failed: {e}"),
        }
    }
    next.set(GameState::Loading);
}

fn load_failed(status: Res<crate::game::level::level::LoadingStatus>) {
    error!("map failed to load: {}", status.message);
    std::process::exit(2);
}

fn spawn_bots(
    mut commands: Commands,
    args: Res<ServerArgs>,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    settings: Res<PlayerSettings>,
    config: Res<GameConfig>,
    map: Res<LoadedGameplayMapConfig>,
    mut net: ResMut<ServerNet>,
    existing: Query<(), With<BotController>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(map) = map.config.as_ref() else { return };
    for index in 0..args.bots {
        let slot = net.next_slot;
        net.next_slot += 1;
        let team = if index % 2 == 0 {
            Team::Attacker
        } else {
            Team::Defender
        };
        let placement = placement_for(map, &config.mode, team, slot);
        let net_id = net.allocate_net_id();
        let body = spawn_actor(
            &mut commands,
            &assets,
            &gltfs,
            &settings,
            ActorSpawn {
                slot,
                team,
                kind: ActorKind::Bot,
                name: Some(crate::game::matchplay::BOT_NAMES[index % 12].to_string()),
                placement: Some(placement),
                melee_weapon: WeaponId::DefaultKnife,
                skin: team_skin(team),
                map,
            },
        );
        commands.entity(body).insert(NetIdentity(net_id));
    }
}

/// Server-side tag so snapshot ids are stable even before Combatant is written.
#[derive(Component)]
pub struct NetIdentity(pub u32);

#[derive(Component)]
pub struct RemotePlayer {
    pub addr: SocketAddr,
}

fn placement_for(
    map: &crate::game::map::MapConfig,
    mode: &GameMode,
    team: Team,
    slot: usize,
) -> (Vec3, f32) {
    let candidates: Vec<_> = map
        .spawn_points
        .iter()
        .filter(|s| !mode.teams() || s.team.is_none() || s.team == Some(team))
        .collect();
    let point = candidates[slot % candidates.len().max(1)];
    (
        map.transform
            .to_transform()
            .transform_point(point.position.to_vec3()),
        point.rotation.to_radians(),
    )
}

fn verify_ticket(args: &ServerArgs, ticket: &str) -> Result<(i64, String), String> {
    let Some(hub) = &args.hub else {
        // Without a hub the ticket is the display name (LAN / development).
        return Ok((0, ticket.chars().take(20).collect()));
    };
    let response = ureq::post(&format!("{hub}/v1/servers/{}/verify_ticket", args.server_id))
        .send_json(serde_json::json!({ "key": args.key, "ticket": ticket }))
        .map_err(|e| format!("ticket rejected: {e}"))?;
    let value: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
    let account = &value["account"];
    Ok((
        account["id"].as_i64().unwrap_or(0),
        account["display_name"]
            .as_str()
            .or(account["username"].as_str())
            .unwrap_or("PLAYER")
            .to_string(),
    ))
}

fn receive_idle(net: Option<ResMut<ServerNet>>, time: Res<Time<bevy::time::Real>>) {
    // Outside Playing, keep the socket drained and keep Join attempts alive.
    let Some(mut net) = net else { return };
    let net = &mut *net;
    let now = time.elapsed_secs_f64();
    loop {
        let (len, addr) = match net.socket.recv_from(&mut net.buffer) {
            Ok(r) => r,
            Err(_) => break,
        };
        let bytes = net.buffer[..len].to_vec();
        if let Some(ClientMessage::Input(_)) = decode::<ClientMessage>(&bytes) {
            if let Some(conn) = net.clients.get_mut(&addr) {
                conn.last_seen = now;
            }
        }
    }
}

fn receive(
    mut net: ResMut<ServerNet>,
    args: Res<ServerArgs>,
    time: Res<Time<bevy::time::Real>>,
    mut commands: Commands,
    mut actors: Query<&mut Combatant>,
    models: Query<(Entity, &PlayerModel)>,
) {
    let net = &mut *net;
    let now = time.elapsed_secs_f64();
    loop {
        let (len, addr) = match net.socket.recv_from(&mut net.buffer) {
            Ok(r) => r,
            Err(_) => break,
        };
        let bytes = net.buffer[..len].to_vec();
        let Some(message) = decode::<ClientMessage>(&bytes) else {
            continue;
        };
        match message {
            ClientMessage::Join { version, ticket, name } => {
                if version != PROTOCOL_VERSION {
                    net.send(
                        addr,
                        &ServerMessage::Rejected {
                            reason: format!("protocol {version}, server speaks {PROTOCOL_VERSION}"),
                        },
                    );
                    continue;
                }
                if net.clients.contains_key(&addr) {
                    // Duplicate Join while the Welcome is in flight; it is re-sent below.
                    if let Some(conn) = net.clients.get_mut(&addr) {
                        conn.last_seen = now;
                        conn.welcomed = false;
                    }
                    continue;
                }
                let humans = net.clients.len();
                if humans >= args.max_players {
                    net.send(addr, &ServerMessage::Rejected { reason: "match is full".into() });
                    continue;
                }
                match verify_ticket(&args, &ticket) {
                    Ok((account_id, display)) => {
                        let net_id = net.allocate_net_id();
                        let slot = net.next_slot;
                        net.next_slot += 1;
                        let counts = team_counts(&actors);
                        let team = if counts[0] <= counts[1] {
                            Team::Attacker
                        } else {
                            Team::Defender
                        };
                        let name = if display.is_empty() { name } else { display };
                        info!("{name} joined from {addr} (net {net_id}, slot {slot})");
                        net.push_event(NetEvent::Joined { name: name.clone() });
                        net.clients.insert(
                            addr,
                            ClientConn {
                                net_id,
                                entity: None,
                                name,
                                account_id,
                                team,
                                slot,
                                last_seen: now,
                                latest: None,
                                last_seq: 0,
                                welcomed: false,
                            },
                        );
                    }
                    Err(reason) => {
                        warn!("join from {addr} refused: {reason}");
                        net.send(addr, &ServerMessage::Rejected { reason });
                    }
                }
            }
            ClientMessage::Input(input) => {
                if let Some(conn) = net.clients.get_mut(&addr) {
                    conn.last_seen = now;
                    if input.seq.wrapping_sub(conn.last_seq) < u32::MAX / 2 || conn.latest.is_none() {
                        conn.last_seq = input.seq;
                        conn.latest = Some(input);
                    }
                }
            }
            ClientMessage::Leave => {
                if let Some(conn) = net.clients.remove(&addr) {
                    info!("{} left", conn.name);
                    despawn_remote(&mut commands, &conn, &models);
                    net.push_event(NetEvent::Left { name: conn.name });
                }
            }
        }
    }
    // Timeouts.
    let stale: Vec<SocketAddr> = net
        .clients
        .iter()
        .filter(|(_, c)| now - c.last_seen > CLIENT_TIMEOUT_SECS as f64)
        .map(|(a, _)| *a)
        .collect();
    for addr in stale {
        if let Some(conn) = net.clients.remove(&addr) {
            info!("{} timed out", conn.name);
            despawn_remote(&mut commands, &conn, &models);
            net.push_event(NetEvent::Left { name: conn.name });
        }
    }
    let _ = &mut actors;
}

fn team_counts(actors: &Query<&mut Combatant>) -> [usize; 2] {
    let mut counts = [0; 2];
    for actor in actors.iter() {
        counts[actor.team.index()] += 1;
    }
    counts
}

fn despawn_remote(commands: &mut Commands, conn: &ClientConn, models: &Query<(Entity, &PlayerModel)>) {
    if let Some(entity) = conn.entity {
        for (model, link) in models.iter() {
            if link.logical_entity == entity {
                commands.entity(model).despawn();
            }
        }
        commands.entity(entity).despawn();
    }
}

fn spawn_pending_remotes(
    mut commands: Commands,
    mut net: ResMut<ServerNet>,
    args: Res<ServerArgs>,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    settings: Res<PlayerSettings>,
    config: Res<GameConfig>,
    map: Res<LoadedGameplayMapConfig>,
) {
    let Some(map) = map.config.as_ref() else { return };
    let pending: Vec<SocketAddr> = net
        .clients
        .iter()
        .filter(|(_, c)| c.entity.is_none() || !c.welcomed)
        .map(|(a, _)| *a)
        .collect();
    for addr in pending {
        let Some(conn) = net.clients.get_mut(&addr) else { continue };
        let placement = placement_for(map, &config.mode, conn.team, conn.slot);
        if conn.entity.is_none() {
            let body = spawn_actor(
                &mut commands,
                &assets,
                &gltfs,
                &settings,
                ActorSpawn {
                    slot: conn.slot,
                    team: conn.team,
                    kind: ActorKind::Remote,
                    name: Some(conn.name.clone()),
                    placement: Some(placement),
                    melee_weapon: WeaponId::DefaultKnife,
                    skin: team_skin(conn.team),
                    map,
                },
            );
            commands
                .entity(body)
                .insert((NetIdentity(conn.net_id), RemotePlayer { addr }));
            conn.entity = Some(body);
        }
        let welcome = ServerMessage::Welcome {
            net_id: conn.net_id,
            slot: conn.slot as u8,
            team: conn.team,
            mode: config.mode.code(),
            map: config.map.code(),
            position: placement.0,
            yaw: placement.1,
            time_limit_secs: args.time_limit_secs as u32,
            score_limit: args.score_limit,
        };
        conn.welcomed = true;
        let _ = net.socket.send_to(&encode(&welcome), addr);
    }
}

fn apply_remote_inputs(
    net: Res<ServerNet>,
    mut remotes: Query<(
        &RemotePlayer,
        &mut Combatant,
        &mut Transform,
        &mut Velocity,
        &mut FpsControllerInput,
        &mut FpsController,
        &mut ActorIntent,
    )>,
) {
    for (remote, actor, mut transform, mut velocity, mut input, mut controller, mut intent) in
        &mut remotes
    {
        let Some(latest) = net.clients.get(&remote.addr).and_then(|c| c.latest.as_ref()) else {
            continue;
        };
        if !actor.alive() {
            continue;
        }
        input.yaw = latest.yaw;
        input.pitch = latest.pitch.clamp(-1.52, 1.52);
        input.movement = latest.movement.clamp_length_max(1.0);
        input.crouch = latest.crouch;
        input.sprint = latest.sprint;
        input.jump = latest.jump;
        intent.fire = latest.fire;
        intent.reload |= latest.reload;
        if let Some(code) = latest.select {
            intent.selection = Some(WeaponSelection::Select(weapon_from_code(code)));
        } else if latest.select_previous {
            intent.selection = Some(WeaponSelection::Previous);
        }
        let finite = latest.position.is_finite() && latest.velocity.is_finite();
        if finite && latest.respawn_ack == actor.respawn_seq && latest.position.length() < 5000.0 {
            transform.translation = latest.position;
            velocity.linvel = latest.velocity.clamp_length_max(60.0);
            controller.height = latest.height.clamp(controller.crouch_height, controller.upright_height);
        }
        let _ = &actor;
    }
}

fn broadcast(
    mut net: ResMut<ServerNet>,
    session: Res<MatchSession>,
    mut kills: EventReader<KillNotice>,
    mut shots: EventReader<ShotFired>,
    actors: Query<(
        Entity,
        &NetIdentity,
        &Combatant,
        &Transform,
        &Velocity,
        &FpsController,
        &FpsControllerInput,
        &WeaponState,
        Option<&BotController>,
    )>,
    ids: Query<&NetIdentity>,
) {
    net.tick = net.tick.wrapping_add(1);
    let id_of = |e: Entity| ids.get(e).map(|n| n.0).unwrap_or(0);
    for kill in kills.read() {
        let event = NetEvent::Kill {
            killer: id_of(kill.killer_entity),
            victim: id_of(kill.victim_entity),
            killer_name: kill.killer.clone(),
            victim_name: kill.victim.clone(),
            killer_team: kill.team,
            victim_team: kill.victim_team,
            weapon: weapon_code(kill.weapon),
            headshot: kill.headshot,
        };
        net.push_event(event);
    }
    for shot in shots.read() {
        let event = NetEvent::Shot {
            actor: id_of(shot.actor),
            origin: shot.origin,
            end: shot.end,
        };
        net.push_event(event);
    }
    let mut states = Vec::new();
    for (_, id, actor, transform, velocity, controller, input, weapon, bot) in &actors {
        states.push(ActorState {
            net_id: id.0,
            slot: actor.slot as u8,
            name: actor.name.clone(),
            team: actor.team,
            bot: bot.is_some(),
            position: transform.translation,
            velocity: velocity.linvel,
            yaw: input.yaw,
            pitch: input.pitch,
            height: controller.height,
            health: actor.health,
            armor: actor.armor,
            kills: actor.kills,
            deaths: actor.deaths,
            respawn_remaining: actor.respawn_remaining,
            respawn_seq: actor.respawn_seq,
            protection_remaining: actor.protection_remaining,
            weapon: weapon_code(weapon.active),
            magazine: weapon.magazine,
            reserve: weapon.reserve,
            cooldown: weapon.cooldown,
            reload_remaining: weapon.reload_remaining,
            knife_remaining: weapon.knife_remaining,
            equip_remaining: weapon.equip_remaining,
            flash_remaining: weapon.flash_remaining,
            shots: weapon.shots,
            slashes: weapon.slashes,
            equips: weapon.equips,
        });
    }
    let snapshot = ServerMessage::Snapshot(Snapshot {
        tick: net.tick,
        elapsed: session.elapsed,
        score: session.score,
        actors: states,
        event_base: net.event_base,
        events: net.events.iter().cloned().collect(),
    });
    let bytes = encode(&snapshot);
    for (addr, conn) in &net.clients {
        if conn.welcomed {
            let _ = net.socket.send_to(&bytes, *addr);
        }
    }
}

fn heartbeat(
    time: Res<Time<bevy::time::Real>>,
    mut timer: ResMut<Heartbeat>,
    args: Res<ServerArgs>,
    state: Res<State<GameState>>,
    net: Option<Res<ServerNet>>,
) {
    let Some(net) = net else { return };
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let Some(hub) = &args.hub else { return };
    let phase = match state.get() {
        GameState::Playing => "live",
        GameState::Finished => "finished",
        _ => "warmup",
    };
    let players: Vec<i64> = net
        .clients
        .values()
        .filter(|c| c.account_id != 0)
        .map(|c| c.account_id)
        .collect();
    let body = serde_json::json!({ "key": args.key, "phase": phase, "players": players });
    if let Err(e) = ureq::post(&format!("{hub}/v1/servers/{}/heartbeat", args.server_id)).send_json(body) {
        warn!("heartbeat failed: {e}");
    }
}

fn finished(
    time: Res<Time<bevy::time::Real>>,
    args: Res<ServerArgs>,
    session: Res<MatchSession>,
    mut net: ResMut<ServerNet>,
    actors: Query<(&Combatant, Option<&RemotePlayer>)>,
) {
    let now = time.elapsed_secs_f64();
    let started = *net.finished_at.get_or_insert(now);
    let message = ServerMessage::Finished {
        result: session.result.clone(),
        score: session.score,
    };
    let bytes = encode(&message);
    for addr in net.clients.keys() {
        let _ = net.socket.send_to(&bytes, *addr);
    }
    if !net.reported {
        net.reported = true;
        info!("match finished: {}", session.result);
        if let Some(hub) = &args.hub {
            let top = actors.iter().map(|(a, _)| a.kills).max().unwrap_or(0);
            let results: Vec<serde_json::Value> = actors
                .iter()
                .filter_map(|(actor, remote)| {
                    let addr = remote?.addr;
                    let conn = net.clients.get(&addr)?;
                    (conn.account_id != 0).then(|| {
                        serde_json::json!({
                            "account_id": conn.account_id, "kills": actor.kills, "deaths": actor.deaths,
                            "won": actor.kills == top && top > 0,
                        })
                    })
                })
                .collect();
            let body = serde_json::json!({ "key": args.key, "results": results });
            let _ = ureq::post(&format!("{hub}/v1/servers/{}/report", args.server_id)).send_json(body);
        }
    }
    if now - started > 12.0 {
        info!("shutting down after match end");
        std::process::exit(0);
    }
}

/// Entry point for the `strike-server` binary.
pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<WinitPlugin>()
                .set(bevy::log::LogPlugin {
                    filter: "wgpu=error,naga=warn,bevy_render=warn,bevy_audio=error,bevy_gltf=error".into(),
                    ..default()
                }),
        )
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(2)))
        .add_plugins(crate::game::game::ServerPlugin)
        .run();
    let _ = PlayerEntity;
}
