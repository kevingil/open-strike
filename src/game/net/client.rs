//! Match client. Joins a server with a hub ticket, reports the local actor's inputs and
//! position every tick, mirrors every other actor from snapshots and replays events
//! (kills, shots) into the same local systems the offline game uses.
use super::{proto::*, role_is, NetRole};
use crate::game::{
    assets::GameAssets,
    config::{GameConfig, PlayerLoadout, PlayerSettings},
    game::SimulationSet,
    level::level::LoadedGameplayMapConfig,
    matchplay::{ActorIntent, Combatant, KillNotice, MatchSession, Team},
    player::player::{
        spawn_actor, team_skin, ActorKind, ActorSpawn, LocalPlayer, PlayerEntity, BODY_HEIGHT,
    },
    player::player_model::PlayerModel,
    weapons::{ShotFired, WeaponSelection, WeaponState},
    GameState,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, FpsControllerInput};
use bevy_rapier3d::prelude::*;
use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

#[derive(Clone, Debug)]
pub struct WelcomeInfo {
    pub net_id: u32,
    pub slot: usize,
    pub team: Team,
    pub position: Vec3,
    pub yaw: f32,
}

#[derive(Clone, Debug)]
enum ConnState {
    Connecting,
    Welcomed(WelcomeInfo),
    Rejected(String),
}

#[derive(Resource)]
pub struct ClientNet {
    socket: UdpSocket,
    server: SocketAddr,
    ticket: String,
    name: String,
    state: ConnState,
    last_join_sent: f64,
    last_snapshot_at: f64,
    latest: Option<Snapshot>,
    dirty: bool,
    next_event: Option<u32>,
    input_seq: u32,
    puppets: HashMap<u32, Entity>,
    buffer: Vec<u8>,
    pub finished: Option<(String, [u32; 2])>,
    pub disconnect_reason: Option<String>,
    pending_reload: bool,
    pending_select: Option<WeaponSelection>,
    connect_started: Option<f64>,
}

impl ClientNet {
    pub fn connect(host: &str, port: u16, ticket: String, name: String) -> Result<Self, String> {
        let server = (host, port)
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .next()
            .ok_or_else(|| format!("cannot resolve {host}"))?;
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Self {
            socket,
            server,
            ticket,
            name,
            state: ConnState::Connecting,
            last_join_sent: -10.0,
            last_snapshot_at: 0.0,
            latest: None,
            dirty: false,
            next_event: None,
            input_seq: 0,
            puppets: HashMap::new(),
            buffer: vec![0; MAX_DATAGRAM],
            finished: None,
            disconnect_reason: None,
            pending_reload: false,
            pending_select: None,
            connect_started: None,
        })
    }
    pub fn welcomed(&self) -> bool {
        matches!(self.state, ConnState::Welcomed(_))
    }
    pub fn rejected(&self) -> Option<String> {
        match &self.state {
            ConnState::Rejected(reason) => Some(reason.clone()),
            _ => None,
        }
    }
    pub fn welcome(&self) -> Option<&WelcomeInfo> {
        match &self.state {
            ConnState::Welcomed(info) => Some(info),
            _ => None,
        }
    }
    pub fn my_net_id(&self) -> u32 {
        self.welcome().map(|w| w.net_id).unwrap_or(0)
    }
    fn send(&self, message: &ClientMessage) {
        let _ = self.socket.send_to(&encode(message), self.server);
    }
    pub fn player_count(&self) -> usize {
        self.latest.as_ref().map(|s| s.actors.len()).unwrap_or(0)
    }
}

/// A client-side mirror of a server-owned actor.
#[derive(Component)]
pub struct NetPuppet {
    pub net_id: u32,
    target_position: Vec3,
    target_velocity: Vec3,
    target_yaw: f32,
    target_pitch: f32,
    target_height: f32,
}

pub struct NetClientPlugin;
impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (pump, watch_finished, watch_timeout).run_if(role_is(NetRole::Client)),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            spawn_local.run_if(role_is(NetRole::Client)),
        )
        .add_systems(
            FixedUpdate,
            (apply_snapshot, capture_intent)
                .chain()
                .in_set(SimulationSet::Intent)
                .run_if(in_state(GameState::Playing).and(role_is(NetRole::Client))),
        )
        .add_systems(
            FixedUpdate,
            drive_puppets
                .after(SimulationSet::Movement)
                .before(PhysicsSet::SyncBackend)
                .run_if(in_state(GameState::Playing).and(role_is(NetRole::Client))),
        )
        .add_systems(
            FixedUpdate,
            send_input
                .in_set(SimulationSet::Match)
                .after(crate::game::matchplay::update_match)
                .run_if(in_state(GameState::Playing).and(role_is(NetRole::Client))),
        )
        .add_systems(OnEnter(GameState::MainMenu), disconnect)
        .add_systems(OnEnter(GameState::Loading), leave_finished_match);
    }
}

/// Reads every datagram, keeps the newest snapshot, repeats Join until welcomed and
/// pings while the map is still loading so the server keeps the seat.
fn pump(
    mut net: ResMut<ClientNet>,
    time: Res<Time<bevy::time::Real>>,
    state: Res<State<GameState>>,
) {
    let net = &mut *net;
    let now = time.elapsed_secs_f64();
    if now - net.last_join_sent > 0.5 {
        net.last_join_sent = now;
        if matches!(net.state, ConnState::Connecting) {
            net.connect_started.get_or_insert(now);
            let join = ClientMessage::Join {
                version: PROTOCOL_VERSION,
                ticket: net.ticket.clone(),
                name: net.name.clone(),
            };
            net.send(&join);
        } else if net.welcomed() && *state.get() != GameState::Playing {
            net.send(&ClientMessage::Ping);
        }
    }
    loop {
        let (len, addr) = match net.socket.recv_from(&mut net.buffer) {
            Ok(r) => r,
            Err(_) => break,
        };
        if addr != net.server {
            continue;
        }
        let bytes = net.buffer[..len].to_vec();
        let Some(message) = decode::<ServerMessage>(&bytes) else {
            continue;
        };
        match message {
            ServerMessage::Welcome {
                net_id,
                slot,
                team,
                position,
                yaw,
                ..
            } => {
                if !net.welcomed() {
                    info!("welcomed as net {net_id}, slot {slot}, team {team:?}");
                    net.state = ConnState::Welcomed(WelcomeInfo {
                        net_id,
                        slot: slot as usize,
                        team,
                        position,
                        yaw,
                    });
                    net.last_snapshot_at = now;
                }
            }
            ServerMessage::Rejected { reason } => {
                if !net.welcomed() {
                    net.state = ConnState::Rejected(reason);
                }
            }
            ServerMessage::Snapshot(snapshot) => {
                let newer = net
                    .latest
                    .as_ref()
                    .is_none_or(|old| snapshot.tick.wrapping_sub(old.tick) < u32::MAX / 2);
                if newer {
                    net.latest = Some(snapshot);
                    net.dirty = true;
                    net.last_snapshot_at = now;
                }
            }
            ServerMessage::Finished { result, score } => {
                net.finished = Some((result, score));
                net.last_snapshot_at = now;
            }
        }
    }
}

fn spawn_local(
    mut commands: Commands,
    net: Res<ClientNet>,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    settings: Res<PlayerSettings>,
    loadout: Res<PlayerLoadout>,
    map: Res<LoadedGameplayMapConfig>,
    existing: Query<(), With<LocalPlayer>>,
) {
    if !existing.is_empty() {
        return;
    }
    let (Some(map), Some(welcome)) = (map.config.as_ref(), net.welcome()) else {
        return;
    };
    let body = spawn_actor(
        &mut commands,
        &assets,
        &gltfs,
        &settings,
        ActorSpawn {
            slot: welcome.slot,
            team: welcome.team,
            kind: ActorKind::LocalHuman,
            name: Some(net.name.clone()),
            placement: Some((welcome.position, welcome.yaw)),
            melee_weapon: loadout.melee_weapon,
            primary_weapon: loadout.spawn_gun(if welcome.team == crate::game::matchplay::Team::Defender {
                crate::game::player::skins::PlayerSide::Defender
            } else {
                crate::game::player::skins::PlayerSide::Attacker
            }),
            skin: team_skin(welcome.team),
            map,
            cosmetic: true,
        },
    );
    let net_id = welcome.net_id;
    commands.queue(move |world: &mut World| {
        if let Some(mut actor) = world.get_mut::<Combatant>(body) {
            actor.net_id = net_id;
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn apply_snapshot(
    mut commands: Commands,
    mut net: ResMut<ClientNet>,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    settings: Res<PlayerSettings>,
    map: Res<LoadedGameplayMapConfig>,
    mut session: ResMut<MatchSession>,
    mut kills: EventWriter<KillNotice>,
    mut shots: EventWriter<ShotFired>,
    mut local: Query<
        (
            Entity,
            &mut Combatant,
            &mut Transform,
            &mut Velocity,
            &mut FpsControllerInput,
            &mut FpsController,
            &mut WeaponState,
        ),
        With<LocalPlayer>,
    >,
    mut puppets: Query<
        (
            Entity,
            &mut NetPuppet,
            &mut Combatant,
            &mut WeaponState,
            &mut Transform,
        ),
        Without<LocalPlayer>,
    >,
    models: Query<(Entity, &PlayerModel)>,
) {
    if !net.dirty {
        return;
    }
    net.dirty = false;
    let Some(snapshot) = net.latest.clone() else {
        return;
    };
    let Some(map) = map.config.as_ref() else {
        return;
    };
    let mine = net.my_net_id();
    session.elapsed = snapshot.elapsed;
    session.score = snapshot.score;
    let mut seen: Vec<u32> = Vec::with_capacity(snapshot.actors.len());
    for state in &snapshot.actors {
        seen.push(state.net_id);
        if state.net_id == mine {
            let Ok((
                entity,
                mut actor,
                mut transform,
                mut velocity,
                mut input,
                mut controller,
                mut weapon,
            )) = local.single_mut()
            else {
                continue;
            };
            let was_alive = actor.alive();
            actor.health = state.health;
            actor.armor = state.armor;
            actor.kills = state.kills;
            actor.deaths = state.deaths;
            actor.respawn_remaining = state.respawn_remaining;
            actor.protection_remaining = state.protection_remaining;
            actor.team = state.team;
            if actor.respawn_seq != state.respawn_seq {
                actor.respawn_seq = state.respawn_seq;
                transform.translation = state.position;
                velocity.linvel = Vec3::ZERO;
                input.yaw = state.yaw;
                input.pitch = 0.0;
                controller.height = BODY_HEIGHT;
                controller.ground_tick = 0;
                *weapon = WeaponState::armed(weapon.gun, weapon.melee_weapon);
                commands.entity(entity).remove::<ColliderDisabled>();
            } else if was_alive && !actor.alive() {
                commands.entity(entity).insert(ColliderDisabled);
                weapon.reload_remaining = 0.0;
                weapon.knife_remaining = 0.0;
            }
            continue;
        }
        let entity = match net.puppets.get(&state.net_id).copied() {
            Some(entity) if puppets.contains(entity) => entity,
            _ => {
                let body = spawn_actor(
                    &mut commands,
                    &assets,
                    &gltfs,
                    &settings,
                    ActorSpawn {
                        slot: state.slot as usize,
                        team: state.team,
                        kind: ActorKind::Puppet,
                        name: Some(state.name.clone()),
                        placement: Some((
                            state.position - Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03),
                            state.yaw,
                        )),
                        melee_weapon: crate::game::config::WeaponId::DefaultKnife,
                        primary_weapon: crate::game::config::WeaponId::AK47,
                        skin: team_skin(state.team),
                        map,
                        cosmetic: true,
                    },
                );
                commands.entity(body).insert(NetPuppet {
                    net_id: state.net_id,
                    target_position: state.position,
                    target_velocity: state.velocity,
                    target_yaw: state.yaw,
                    target_pitch: state.pitch,
                    target_height: state.height,
                });
                let net_id = state.net_id;
                commands.queue(move |world: &mut World| {
                    if let Some(mut actor) = world.get_mut::<Combatant>(body) {
                        actor.net_id = net_id;
                    }
                });
                net.puppets.insert(state.net_id, body);
                continue;
            }
        };
        let Ok((entity, mut puppet, mut actor, mut weapon, mut transform)) =
            puppets.get_mut(entity)
        else {
            continue;
        };
        let was_alive = actor.alive();
        puppet.target_position = state.position;
        puppet.target_velocity = state.velocity;
        puppet.target_yaw = state.yaw;
        puppet.target_pitch = state.pitch;
        puppet.target_height = state.height;
        actor.health = state.health;
        actor.armor = state.armor;
        actor.kills = state.kills;
        actor.deaths = state.deaths;
        actor.respawn_remaining = state.respawn_remaining;
        actor.protection_remaining = state.protection_remaining;
        actor.team = state.team;
        actor.name = state.name.clone();
        if actor.respawn_seq != state.respawn_seq {
            actor.respawn_seq = state.respawn_seq;
            transform.translation = state.position;
            commands.entity(entity).remove::<ColliderDisabled>();
        } else if was_alive && !actor.alive() {
            commands.entity(entity).insert(ColliderDisabled);
        }
        weapon.active = weapon_from_code(state.weapon);
        weapon.magazine = state.magazine;
        weapon.reserve = state.reserve;
        weapon.cooldown = state.cooldown;
        weapon.reload_remaining = state.reload_remaining;
        weapon.knife_remaining = state.knife_remaining;
        weapon.equip_remaining = state.equip_remaining;
        weapon.flash_remaining = state.flash_remaining;
        weapon.shots = state.shots;
        weapon.slashes = state.slashes;
        weapon.equips = state.equips;
    }
    // Actors that left.
    let gone: Vec<(u32, Entity)> = net
        .puppets
        .iter()
        .filter(|(id, _)| !seen.contains(id))
        .map(|(id, e)| (*id, *e))
        .collect();
    for (id, entity) in gone {
        net.puppets.remove(&id);
        for (model, link) in &models {
            if link.logical_entity == entity {
                commands.entity(model).despawn();
            }
        }
        if puppets.contains(entity) {
            commands.entity(entity).despawn();
        }
    }
    // Events, replayed once each. The first snapshot's history is skipped.
    let local_entity = local.single().map(|(e, ..)| e).ok();
    let puppet_map = net.puppets.clone();
    let entity_of = |id: u32| {
        if id == mine {
            local_entity.unwrap_or(Entity::PLACEHOLDER)
        } else {
            puppet_map.get(&id).copied().unwrap_or(Entity::PLACEHOLDER)
        }
    };
    let first_seq = snapshot.event_base;
    let next = *net
        .next_event
        .get_or_insert(first_seq.wrapping_add(snapshot.events.len() as u32));
    let mut cursor = next;
    for (index, event) in snapshot.events.iter().enumerate() {
        let seq = first_seq.wrapping_add(index as u32);
        if seq.wrapping_sub(next) >= u32::MAX / 2 {
            continue;
        }
        cursor = seq.wrapping_add(1);
        match event {
            NetEvent::Kill {
                killer,
                victim,
                killer_name,
                victim_name,
                killer_team,
                victim_team,
                weapon,
                headshot,
            } => {
                kills.write(KillNotice {
                    killer_entity: entity_of(*killer),
                    victim_entity: entity_of(*victim),
                    victim_team: *victim_team,
                    weapon: weapon_from_code(*weapon),
                    headshot: *headshot,
                    killer: killer_name.clone(),
                    victim: victim_name.clone(),
                    team: *killer_team,
                });
            }
            NetEvent::Shot { actor, origin, end } => {
                if *actor != mine {
                    shots.write(ShotFired {
                        actor: entity_of(*actor),
                        origin: *origin,
                        end: *end,
                    });
                }
            }
            NetEvent::Joined { name } => info!("{name} joined the match"),
            NetEvent::Left { name } => info!("{name} left the match"),
        }
    }
    net.next_event = Some(cursor);
}

/// Intents are consumed by the weapon simulation before the input is sent, so keep
/// the edge-triggered ones until they have gone out.
fn capture_intent(mut net: ResMut<ClientNet>, local: Query<&ActorIntent, With<LocalPlayer>>) {
    let Ok(intent) = local.single() else { return };
    net.pending_reload |= intent.reload;
    if let Some(selection) = intent.selection {
        net.pending_select = Some(selection);
    }
}

fn drive_puppets(
    time: Res<Time<Fixed>>,
    mut puppets: Query<(
        &NetPuppet,
        &Combatant,
        &mut Transform,
        &mut Velocity,
        &mut FpsControllerInput,
        &mut FpsController,
    )>,
) {
    let dt = time.delta_secs();
    let blend = 1.0 - (-dt * 18.0).exp();
    for (puppet, actor, mut transform, mut velocity, mut input, mut controller) in &mut puppets {
        let distance = transform.translation.distance(puppet.target_position);
        if distance > 3.0 || !actor.alive() {
            transform.translation = puppet.target_position;
        } else {
            // Extrapolate along the reported velocity, then ease onto the reported position.
            let predicted = puppet.target_position + puppet.target_velocity * dt;
            transform.translation = transform.translation.lerp(predicted, blend);
        }
        velocity.linvel = puppet.target_velocity;
        input.yaw = puppet.target_yaw;
        input.pitch = puppet.target_pitch;
        input.crouch = puppet.target_height < BODY_HEIGHT - 0.05;
        input.movement = if puppet.target_velocity.xz().length() > 0.3 {
            Vec3::Z
        } else {
            Vec3::ZERO
        };
        controller.yaw = puppet.target_yaw;
        controller.pitch = puppet.target_pitch;
        controller.height = puppet.target_height;
    }
}

fn send_input(
    mut net: ResMut<ClientNet>,
    local: Query<
        (
            &Combatant,
            &Transform,
            &Velocity,
            &FpsControllerInput,
            &FpsController,
            &ActorIntent,
        ),
        With<LocalPlayer>,
    >,
) {
    let Ok((actor, transform, velocity, input, controller, intent)) = local.single() else {
        return;
    };
    net.input_seq = net.input_seq.wrapping_add(1);
    let message = ClientMessage::Input(PlayerInput {
        seq: net.input_seq,
        respawn_ack: actor.respawn_seq,
        position: transform.translation,
        velocity: velocity.linvel,
        yaw: input.yaw,
        pitch: input.pitch,
        height: controller.height,
        movement: input.movement,
        crouch: input.crouch,
        sprint: input.sprint,
        jump: input.jump,
        fire: intent.fire,
        reload: net.pending_reload,
        select: match net.pending_select {
            Some(WeaponSelection::Select(id)) => Some(weapon_code(id)),
            _ => None,
        },
        select_previous: matches!(net.pending_select, Some(WeaponSelection::Previous)),
    });
    net.pending_reload = false;
    net.pending_select = None;
    net.send(&message);
}

fn watch_finished(
    net: Res<ClientNet>,
    state: Res<State<GameState>>,
    mut session: ResMut<MatchSession>,
    mut next: ResMut<NextState<GameState>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    if let Some((result, score)) = &net.finished {
        session.result = result.clone();
        session.score = *score;
        next.set(GameState::Finished);
    }
}

fn watch_timeout(
    mut net: ResMut<ClientNet>,
    time: Res<Time<bevy::time::Real>>,
    state: Res<State<GameState>>,
    mut status: ResMut<crate::game::level::level::LoadingStatus>,
    mut next: ResMut<NextState<GameState>>,
) {
    let now = time.elapsed_secs_f64();
    let timed_out = match state.get() {
        GameState::Playing => {
            net.welcomed() && now - net.last_snapshot_at > CLIENT_TIMEOUT_SECS as f64
        }
        GameState::Loading => {
            !net.welcomed()
                && net.rejected().is_none()
                && net
                    .connect_started
                    .is_some_and(|started| now - started > 45.0)
        }
        _ => false,
    };
    if timed_out && net.disconnect_reason.is_none() {
        net.disconnect_reason = Some("Lost connection to the match server".into());
        status.message = "Lost connection to the match server".into();
        warn!("connection to match server lost");
        next.set(GameState::LoadFailed);
    }
}

fn disconnect(mut commands: Commands, net: Option<Res<ClientNet>>, mut role: ResMut<NetRole>) {
    if let Some(net) = net {
        net.send(&ClientMessage::Leave);
        commands.remove_resource::<ClientNet>();
    }
    *role = NetRole::Local;
}

/// "Play again" after an online match starts a local match of the same mode.
fn leave_finished_match(
    mut commands: Commands,
    net: Option<Res<ClientNet>>,
    mut role: ResMut<NetRole>,
    _config: Res<GameConfig>,
) {
    if let Some(net) = net {
        if net.finished.is_some() || net.disconnect_reason.is_some() {
            net.send(&ClientMessage::Leave);
            commands.remove_resource::<ClientNet>();
            *role = NetRole::Local;
        }
    }
    let _ = PlayerEntity;
}
