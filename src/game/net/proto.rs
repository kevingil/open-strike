//! Wire types. Plain UDP datagrams, bincode encoded. Inputs and snapshots are
//! idempotent so nothing needs reliable delivery; events ride along in snapshots
//! with sequence numbers and the client deduplicates them.
use crate::game::{config::WeaponId, matchplay::Team};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_DATAGRAM: usize = 60_000;
/// Snapshots and inputs both go once per fixed tick.
pub const TICK_HZ: f64 = 64.0;
pub const CLIENT_TIMEOUT_SECS: f32 = 15.0;
pub const EVENT_HISTORY: usize = 64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    Join {
        version: u16,
        ticket: String,
        name: String,
    },
    Input(PlayerInput),
    /// Keeps the seat while the client is still loading the map.
    Ping,
    Leave,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlayerInput {
    pub seq: u32,
    /// The respawn generation this input was produced for; the server ignores
    /// client positions until the client has seen its latest respawn.
    pub respawn_ack: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub height: f32,
    pub movement: Vec3,
    pub crouch: bool,
    pub sprint: bool,
    pub jump: bool,
    pub fire: bool,
    pub reload: bool,
    pub select: Option<u8>,
    pub select_previous: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    Welcome {
        net_id: u32,
        slot: u8,
        team: Team,
        mode: u8,
        map: u8,
        position: Vec3,
        yaw: f32,
        time_limit_secs: u32,
        score_limit: u32,
    },
    Rejected {
        reason: String,
    },
    Snapshot(Snapshot),
    Finished {
        result: String,
        score: [u32; 2],
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Snapshot {
    pub tick: u32,
    pub elapsed: f32,
    pub score: [u32; 2],
    pub actors: Vec<ActorState>,
    /// Sequence number of the first event in `events`.
    pub event_base: u32,
    pub events: Vec<NetEvent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActorState {
    pub net_id: u32,
    pub slot: u8,
    pub name: String,
    pub team: Team,
    pub bot: bool,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub height: f32,
    pub health: f32,
    pub armor: f32,
    pub kills: u32,
    pub deaths: u32,
    pub respawn_remaining: f32,
    pub respawn_seq: u32,
    pub protection_remaining: f32,
    pub weapon: u8,
    pub magazine: u32,
    pub reserve: u32,
    pub cooldown: f32,
    pub reload_remaining: f32,
    pub knife_remaining: f32,
    pub equip_remaining: f32,
    pub flash_remaining: f32,
    pub shots: u64,
    pub slashes: u64,
    pub equips: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetEvent {
    Kill {
        killer: u32,
        victim: u32,
        killer_name: String,
        victim_name: String,
        killer_team: Team,
        victim_team: Team,
        weapon: u8,
        headshot: bool,
    },
    Shot {
        actor: u32,
        origin: Vec3,
        end: Vec3,
    },
    Joined {
        name: String,
    },
    Left {
        name: String,
    },
}

pub fn weapon_code(id: WeaponId) -> u8 {
    match id {
        WeaponId::AK47 => 0,
        WeaponId::DefaultKnife => 1,
        WeaponId::ReferenceKnife => 2,
    }
}

pub fn weapon_from_code(code: u8) -> WeaponId {
    match code {
        1 => WeaponId::DefaultKnife,
        2 => WeaponId::ReferenceKnife,
        _ => WeaponId::AK47,
    }
}

pub fn encode<T: Serialize>(message: &T) -> Vec<u8> {
    bincode::serialize(message).expect("wire types serialize")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Option<T> {
    bincode::deserialize(bytes).ok()
}
