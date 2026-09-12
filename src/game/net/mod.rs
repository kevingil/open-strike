//! UDP match protocol shared by the client and the dedicated server.
pub mod client;
pub mod proto;
pub mod server;

use bevy::prelude::*;

/// Which side of the match protocol this process plays.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetRole {
    #[default]
    Local,
    Client,
    Server,
}

impl NetRole {
    pub fn is_client(self) -> bool {
        self == NetRole::Client
    }
    pub fn is_server(self) -> bool {
        self == NetRole::Server
    }
    pub fn online(self) -> bool {
        self != NetRole::Local
    }
}

pub fn role_is(role: NetRole) -> impl Fn(Res<NetRole>) -> bool {
    move |current: Res<NetRole>| *current == role
}
