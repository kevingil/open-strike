//! Client side of strike-hub: session, friends, presence and matchmaking, all
//! non-blocking. HTTP runs on the IO task pool; results come back as `HubEvent`s.
pub mod scenario;
use crate::game::{
    config::{GameConfig, GameMode, MapId},
    local::{LocalDb, KEY_HUB},
    net::{client::ClientNet, NetRole},
    GameState,
};
use bevy::{prelude::*, tasks::IoTaskPool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubRequest {
    Health,
    Register,
    Login,
    Me,
    Logout,
    Friends,
    Search,
    FriendRequest(String),
    Accept(String),
    Decline(String),
    Remove(String),
    Presence,
    FindMatch(GameMode),
    JoinMatch(String),
    Servers,
}

#[derive(Clone, Debug)]
pub struct HubError {
    pub code: String,
    pub message: String,
    pub transport: bool,
}

#[derive(Event, Clone, Debug)]
pub struct HubEvent {
    pub request: HubRequest,
    pub result: Result<Value, HubError>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub enum HubSession {
    #[default]
    NoHub,
    LoggedOut,
    Connecting,
    LoggedIn(Account),
    Offline(Option<Account>),
}

impl HubSession {
    pub fn account(&self) -> Option<&Account> {
        match self {
            HubSession::LoggedIn(a) => Some(a),
            HubSession::Offline(Some(a)) => Some(a),
            _ => None,
        }
    }
    pub fn logged_in(&self) -> bool {
        matches!(self, HubSession::LoggedIn(_))
    }
}

#[derive(Resource, Debug)]
pub struct HubHealth {
    pub online: bool,
    pub hub_name: String,
    retry_at: f64,
    backoff: f64,
    checking: bool,
}

impl Default for HubHealth {
    fn default() -> Self {
        Self {
            online: true,
            hub_name: String::new(),
            retry_at: 0.0,
            backoff: 5.0,
            checking: false,
        }
    }
}

impl HubHealth {
    pub fn retry_in(&self, now: f64) -> f64 {
        (self.retry_at - now).max(0.0)
    }
}

#[derive(Clone, Debug, Deserialize, Default, PartialEq)]
pub struct Presence {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub server: Option<Value>,
    #[serde(default)]
    pub last_seen: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct FriendEntry {
    pub username: String,
    pub display_name: String,
    #[serde(default)]
    pub presence: Presence,
    #[serde(default)]
    pub relationship: String,
}

#[derive(Resource, Default, Debug)]
pub struct FriendsState {
    pub friends: Vec<FriendEntry>,
    pub pending_in: Vec<FriendEntry>,
    pub pending_out: Vec<FriendEntry>,
    pub search_results: Vec<FriendEntry>,
    pub search_query_sent: String,
    pub version: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FormMode {
    #[default]
    Idle,
    Login,
    Register,
}

/// Values behind the sidebar forms; the widgets are rebuilt from these.
#[derive(Resource, Default, Debug)]
pub struct HubForms {
    pub mode: FormMode,
    pub identifier: String,
    pub password: String,
    pub username: String,
    pub email: String,
    pub confirm: String,
    pub search: String,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub busy: bool,
    pub version: u64,
}

/// A match the hub told us to join; consumed by `start_pending_match`.
#[derive(Resource, Clone, Debug)]
pub struct PendingMatch {
    pub host: String,
    pub port: u16,
    pub ticket: String,
    pub mode: GameMode,
    pub map: MapId,
    pub server_id: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct StoredConfig {
    url: String,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Resource)]
pub struct HubClient {
    pub url: String,
    session: Option<String>,
    username: Option<String>,
    tx: Sender<HubEvent>,
    rx: Mutex<Receiver<HubEvent>>,
    device_label: String,
    pub in_flight: usize,
    db: LocalDb,
}

impl HubClient {
    fn new(url: String, session: Option<String>, username: Option<String>, db: LocalDb) -> Self {
        let (tx, rx) = channel();
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "desktop".into());
        Self {
            url,
            session,
            username,
            tx,
            rx: Mutex::new(rx),
            device_label: format!("{host} ({})", std::env::consts::OS),
            in_flight: 0,
            db,
        }
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    fn persist(&self) {
        let stored = StoredConfig {
            url: self.url.clone(),
            session: self.session.clone(),
            username: self.username.clone(),
        };
        if let Ok(text) = serde_json::to_string(&stored) {
            self.db.set(KEY_HUB, &text);
        }
    }

    pub fn request(&mut self, request: HubRequest, method: &'static str, path: String, body: Option<Value>) {
        let url = format!("{}{}", self.url, path);
        let token = self.session.clone();
        let tx = self.tx.clone();
        self.in_flight += 1;
        IoTaskPool::get()
            .spawn(async move {
                let agent = ureq::AgentBuilder::new()
                    .timeout_connect(std::time::Duration::from_secs(3))
                    .timeout(std::time::Duration::from_secs(60))
                    .build();
                let mut req = agent.request(method, &url);
                if let Some(token) = token {
                    req = req.set("Authorization", &format!("Bearer {token}"));
                }
                let response = match body {
                    Some(body) => req.send_json(body),
                    None => req.call(),
                };
                let result = match response {
                    Ok(resp) => resp
                        .into_json::<Value>()
                        .map_err(|e| HubError { code: "bad_response".into(), message: e.to_string(), transport: false }),
                    Err(ureq::Error::Status(code, resp)) => {
                        let value: Value = resp.into_json().unwrap_or(Value::Null);
                        Err(HubError {
                            code: value["error"].as_str().unwrap_or("http").to_string(),
                            message: value["message"]
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| format!("HTTP {code}")),
                            transport: false,
                        })
                    }
                    Err(ureq::Error::Transport(t)) => Err(HubError {
                        code: "unreachable".into(),
                        message: t.to_string(),
                        transport: true,
                    }),
                };
                let _ = tx.send(HubEvent { request, result });
            })
            .detach();
    }

    pub fn health(&mut self) {
        self.request(HubRequest::Health, "GET", "/v1/health".into(), None);
    }
    pub fn register(&mut self, username: &str, email: &str, password: &str) {
        let body = json!({ "username": username, "email": email, "password": password, "device_label": self.device_label });
        self.request(HubRequest::Register, "POST", "/v1/auth/register".into(), Some(body));
    }
    pub fn login(&mut self, identifier: &str, password: &str) {
        let body = json!({ "identifier": identifier, "password": password, "device_label": self.device_label });
        self.request(HubRequest::Login, "POST", "/v1/auth/login".into(), Some(body));
    }
    pub fn me(&mut self) {
        self.request(HubRequest::Me, "GET", "/v1/auth/me".into(), None);
    }
    pub fn logout(&mut self) {
        self.request(HubRequest::Logout, "POST", "/v1/auth/logout".into(), Some(json!({})));
        self.session = None;
        self.persist();
    }
    pub fn friends(&mut self) {
        self.request(HubRequest::Friends, "GET", "/v1/friends".into(), None);
    }
    pub fn search(&mut self, query: &str) {
        let q: String = query
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        self.request(HubRequest::Search, "GET", format!("/v1/friends/search?q={q}"), None);
    }
    pub fn friend_request(&mut self, username: &str) {
        self.request(HubRequest::FriendRequest(username.into()), "POST", "/v1/friends/request".into(), Some(json!({ "username": username })));
    }
    pub fn accept(&mut self, username: &str) {
        self.request(HubRequest::Accept(username.into()), "POST", "/v1/friends/accept".into(), Some(json!({ "username": username })));
    }
    pub fn decline(&mut self, username: &str) {
        self.request(HubRequest::Decline(username.into()), "POST", "/v1/friends/decline".into(), Some(json!({ "username": username })));
    }
    pub fn remove(&mut self, username: &str) {
        self.request(HubRequest::Remove(username.into()), "DELETE", format!("/v1/friends/{username}"), None);
    }
    pub fn presence(&mut self, state: &str) {
        self.request(HubRequest::Presence, "POST", "/v1/presence".into(), Some(json!({ "state": state })));
    }
    pub fn find_match(&mut self, mode: GameMode) {
        self.request(HubRequest::FindMatch(mode.clone()), "POST", "/v1/match/find".into(), Some(json!({ "mode": mode.key() })));
    }
    pub fn join_match(&mut self, server_id: &str) {
        self.request(HubRequest::JoinMatch(server_id.into()), "POST", "/v1/match/join".into(), Some(json!({ "server_id": server_id })));
    }
}

#[derive(Resource)]
struct Timers {
    friends: Timer,
    presence: Timer,
}

pub struct HubPlugin;
impl Plugin for HubPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HubSession>()
            .init_resource::<HubHealth>()
            .init_resource::<FriendsState>()
            .init_resource::<HubForms>()
            .insert_resource(Timers {
                friends: Timer::from_seconds(10.0, TimerMode::Repeating),
                presence: Timer::from_seconds(30.0, TimerMode::Repeating),
            })
            .add_event::<HubEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (poll_responses, apply_events, health_loop, periodic, start_pending_match).chain(),
            )
            .add_systems(OnEnter(GameState::MainMenu), presence_menu);
    }
}

fn setup(mut commands: Commands, mut session: ResMut<HubSession>, db: Res<LocalDb>) {
    let stored: Option<StoredConfig> = db
        .get(KEY_HUB)
        .and_then(|text| serde_json::from_str(&text).ok());
    let url = std::env::var("STRIKE_HUB_URL")
        .ok()
        .or_else(|| stored.as_ref().map(|s| s.url.clone()))
        .filter(|u| !u.is_empty());
    let Some(url) = url else {
        *session = HubSession::NoHub;
        return;
    };
    let url = url.trim_end_matches('/').to_string();
    let (token, username) = stored
        .filter(|s| s.url.trim_end_matches('/') == url)
        .map(|s| (s.session, s.username))
        .unwrap_or((None, None));
    let mut client = HubClient::new(url, token.clone(), username, db.clone());
    if token.is_some() {
        *session = HubSession::Connecting;
        client.me();
    } else {
        *session = HubSession::LoggedOut;
        client.health();
    }
    commands.insert_resource(client);
}

fn poll_responses(client: Option<ResMut<HubClient>>, mut events: EventWriter<HubEvent>) {
    let Some(mut client) = client else { return };
    let drained: Vec<HubEvent> = {
        let rx = client.rx.lock().unwrap();
        rx.try_iter().collect()
    };
    for event in drained {
        client.in_flight = client.in_flight.saturating_sub(1);
        events.write(event);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_events(
    mut commands: Commands,
    mut events: EventReader<HubEvent>,
    client: Option<ResMut<HubClient>>,
    mut session: ResMut<HubSession>,
    mut health: ResMut<HubHealth>,
    mut friends: ResMut<FriendsState>,
    mut forms: ResMut<HubForms>,
    time: Res<Time<Real>>,
) {
    let Some(mut client) = client else { return };
    let now = time.elapsed_secs_f64();
    for event in events.read() {
        match &event.result {
            Err(error) if error.transport => {
                if health.online {
                    warn!("hub unreachable: {}", error.message);
                }
                health.online = false;
                health.checking = false;
                health.retry_at = now + health.backoff;
                health.backoff = (health.backoff * 2.0).min(60.0);
                *session = match std::mem::take(&mut *session) {
                    HubSession::LoggedIn(a) | HubSession::Offline(Some(a)) => HubSession::Offline(Some(a)),
                    HubSession::Connecting => HubSession::Offline(None),
                    HubSession::NoHub => HubSession::NoHub,
                    _ => HubSession::Offline(None),
                };
                if matches!(event.request, HubRequest::Login | HubRequest::Register | HubRequest::FindMatch(_) | HubRequest::JoinMatch(_)) {
                    forms.busy = false;
                    forms.error = Some("Hub unreachable, try again".into());
                    forms.version += 1;
                }
                continue;
            }
            Ok(_) | Err(_) => {
                if !health.online {
                    info!("hub reachable again");
                }
                health.online = true;
                health.checking = false;
                health.backoff = 5.0;
            }
        }
        match (&event.request, &event.result) {
            (HubRequest::Health, Ok(value)) => {
                health.hub_name = value["name"].as_str().unwrap_or("").to_string();
                if matches!(*session, HubSession::Offline(_)) {
                    *session = match std::mem::take(&mut *session) {
                        HubSession::Offline(Some(_)) if client.session.is_some() => {
                            client.me();
                            HubSession::Connecting
                        }
                        _ => HubSession::LoggedOut,
                    };
                }
            }
            (HubRequest::Register | HubRequest::Login, Ok(value)) => {
                let account: Account = serde_json::from_value(value["account"].clone()).unwrap_or(Account {
                    id: 0,
                    username: "?".into(),
                    display_name: "?".into(),
                    role: "player".into(),
                    email: None,
                });
                client.session = value["session"].as_str().map(str::to_string);
                client.username = Some(account.username.clone());
                client.persist();
                health.hub_name = value["hub"]["name"].as_str().unwrap_or("").to_string();
                forms.busy = false;
                forms.error = None;
                forms.mode = FormMode::Idle;
                forms.password.clear();
                forms.confirm.clear();
                forms.notice = value["first_admin"].as_bool().and_then(|first| {
                    first.then(|| "You are the administrator of this hub".to_string())
                });
                forms.version += 1;
                info!("signed in as {}", account.username);
                *session = HubSession::LoggedIn(account);
                client.friends();
                client.presence("in_menu");
            }
            (HubRequest::Register | HubRequest::Login, Err(error)) => {
                forms.busy = false;
                forms.error = Some(error.message.clone());
                forms.version += 1;
            }
            (HubRequest::Me, Ok(value)) => {
                if let Ok(account) = serde_json::from_value::<Account>(value["account"].clone()) {
                    client.username = Some(account.username.clone());
                    health.hub_name = value["hub"]["name"].as_str().unwrap_or("").to_string();
                    *session = HubSession::LoggedIn(account);
                    client.friends();
                    client.presence("in_menu");
                }
            }
            (HubRequest::Me, Err(_)) => {
                client.session = None;
                client.persist();
                *session = HubSession::LoggedOut;
                forms.notice = Some("Session expired, sign in again".into());
                forms.version += 1;
                client.health();
            }
            (HubRequest::Logout, _) => {
                *session = HubSession::LoggedOut;
                *friends = FriendsState::default();
                forms.version += 1;
            }
            (HubRequest::Friends, Ok(value)) => {
                let parse = |key: &str| -> Vec<FriendEntry> {
                    serde_json::from_value(value[key].clone()).unwrap_or_default()
                };
                let (f, i, o) = (parse("friends"), parse("pending_in"), parse("pending_out"));
                if f != friends.friends || i != friends.pending_in || o != friends.pending_out {
                    friends.friends = f;
                    friends.pending_in = i;
                    friends.pending_out = o;
                    friends.version += 1;
                }
            }
            (HubRequest::Search, Ok(value)) => {
                friends.search_results = serde_json::from_value(value["results"].clone()).unwrap_or_default();
                friends.version += 1;
            }
            (HubRequest::FriendRequest(_) | HubRequest::Accept(_) | HubRequest::Decline(_) | HubRequest::Remove(_), _) => {
                client.friends();
                if !friends.search_query_sent.is_empty() {
                    let q = friends.search_query_sent.clone();
                    client.search(&q);
                }
            }
            (HubRequest::FindMatch(mode), Ok(value)) => {
                forms.busy = false;
                forms.version += 1;
                let server = &value["server"];
                commands.insert_resource(PendingMatch {
                    host: server["host"].as_str().unwrap_or("127.0.0.1").to_string(),
                    port: server["port"].as_u64().unwrap_or(27015) as u16,
                    ticket: value["ticket"].as_str().unwrap_or_default().to_string(),
                    mode: mode.clone(),
                    map: server["map"].as_str().and_then(MapId::from_key).unwrap_or_default(),
                    server_id: server["server_id"].as_str().unwrap_or_default().to_string(),
                });
            }
            (HubRequest::JoinMatch(_), Ok(value)) => {
                forms.busy = false;
                forms.version += 1;
                let server = &value["server"];
                commands.insert_resource(PendingMatch {
                    host: server["host"].as_str().unwrap_or("127.0.0.1").to_string(),
                    port: server["port"].as_u64().unwrap_or(27015) as u16,
                    ticket: value["ticket"].as_str().unwrap_or_default().to_string(),
                    mode: server["mode"].as_str().and_then(GameMode::from_key).unwrap_or_default(),
                    map: server["map"].as_str().and_then(MapId::from_key).unwrap_or_default(),
                    server_id: server["server_id"].as_str().unwrap_or_default().to_string(),
                });
            }
            (HubRequest::FindMatch(_) | HubRequest::JoinMatch(_), Err(error)) => {
                forms.busy = false;
                forms.error = Some(error.message.clone());
                forms.version += 1;
            }
            (_, Err(error)) if error.code == "session_expired" => {
                client.session = None;
                client.persist();
                *session = HubSession::LoggedOut;
                forms.notice = Some("Session expired, sign in again".into());
                forms.version += 1;
            }
            _ => {}
        }
    }
}

fn health_loop(client: Option<ResMut<HubClient>>, mut health: ResMut<HubHealth>, time: Res<Time<Real>>) {
    let Some(mut client) = client else { return };
    let now = time.elapsed_secs_f64();
    if !health.online && !health.checking && now >= health.retry_at {
        health.checking = true;
        client.health();
    }
}

fn periodic(
    client: Option<ResMut<HubClient>>,
    session: Res<HubSession>,
    health: Res<HubHealth>,
    mut timers: ResMut<Timers>,
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
) {
    let Some(mut client) = client else { return };
    if !session.logged_in() || !health.online {
        return;
    }
    if timers.friends.tick(time.delta()).just_finished() && *state.get() == GameState::MainMenu {
        client.friends();
    }
    if timers.presence.tick(time.delta()).just_finished() {
        client.presence(if *state.get() == GameState::MainMenu { "in_menu" } else { "online" });
    }
}

fn presence_menu(client: Option<ResMut<HubClient>>, session: Res<HubSession>) {
    let Some(mut client) = client else { return };
    if session.logged_in() {
        client.presence("in_menu");
        client.friends();
    }
}

fn start_pending_match(
    mut commands: Commands,
    pending: Option<Res<PendingMatch>>,
    client: Option<Res<HubClient>>,
    mut config: ResMut<GameConfig>,
    mut role: ResMut<NetRole>,
    mut next: ResMut<NextState<GameState>>,
    mut forms: ResMut<HubForms>,
    state: Res<State<GameState>>,
) {
    let Some(pending) = pending else { return };
    if *state.get() != GameState::MainMenu {
        commands.remove_resource::<PendingMatch>();
        return;
    }
    let name = client
        .as_ref()
        .and_then(|c| c.username().map(str::to_string))
        .unwrap_or_else(|| "PLAYER".into());
    match ClientNet::connect(&pending.host, pending.port, pending.ticket.clone(), name) {
        Ok(net) => {
            config.mode = pending.mode.clone();
            config.map = pending.map.clone();
            *role = NetRole::Client;
            commands.insert_resource(net);
            info!("joining {} at {}:{}", pending.server_id, pending.host, pending.port);
            next.set(GameState::Loading);
        }
        Err(error) => {
            forms.error = Some(format!("Could not open a connection: {error}"));
            forms.version += 1;
        }
    }
    commands.remove_resource::<PendingMatch>();
}
