//! Opt-in end-to-end walkthrough driven through the same handlers the menus use:
//! create or sign into an account, befriend another player, start or join an online
//! match, and capture screenshots along the way. No OS input injection.
//!
//! `CSRS_ONLINE_SCENARIO=host|friend`, `CSRS_USER`, `CSRS_PASS`, `CSRS_EMAIL`,
//! `CSRS_FRIEND`, `CSRS_ONLINE_MODE=dm|tdm`, `CSRS_CAPTURE_DIR`, `CSRS_EXIT_AFTER`.
use super::{FriendsState, HubClient, HubEvent, HubForms, HubRequest, HubSession};
use crate::game::{config::GameMode, ui::menu::friends_drawer::DrawerState, GameState};
use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Host,
    Friend,
}

#[derive(Resource)]
struct Scenario {
    role: Role,
    user: String,
    pass: String,
    email: String,
    friend: String,
    mode: GameMode,
    dir: String,
    exit_after: f64,
    step: u32,
    step_at: f64,
    last_poll: f64,
    playing_since: Option<f64>,
    captures: Vec<&'static str>,
    started: bool,
}

pub struct ScenarioPlugin;
impl Plugin for ScenarioPlugin {
    fn build(&self, app: &mut App) {
        let role = match std::env::var("CSRS_ONLINE_SCENARIO").as_deref() {
            Ok("friend") => Role::Friend,
            _ => Role::Host,
        };
        let user = std::env::var("CSRS_USER").unwrap_or_else(|_| "kevin".into());
        app.insert_resource(Scenario {
            role,
            email: std::env::var("CSRS_EMAIL").unwrap_or_else(|_| format!("{user}@example.com")),
            pass: std::env::var("CSRS_PASS").unwrap_or_else(|_| "open-strike-demo".into()),
            friend: std::env::var("CSRS_FRIEND").unwrap_or_else(|_| "cedar".into()),
            mode: std::env::var("CSRS_ONLINE_MODE")
                .ok()
                .and_then(|m| GameMode::from_key(&m))
                .unwrap_or(GameMode::Deathmatch),
            dir: std::env::var("CSRS_CAPTURE_DIR").unwrap_or_else(|_| "/tmp".into()),
            exit_after: std::env::var("CSRS_EXIT_AFTER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(240.0),
            user,
            step: 0,
            step_at: 0.0,
            last_poll: 0.0,
            playing_since: None,
            captures: Vec::new(),
            started: false,
        })
        .add_systems(Update, (drive, on_events));
    }
}

fn shot(commands: &mut Commands, scenario: &mut Scenario, name: &'static str) {
    if scenario.captures.contains(&name) {
        return;
    }
    scenario.captures.push(name);
    let path = format!("{}/{}-{}.png", scenario.dir, scenario.role_name(), name);
    info!("ONLINE_SCENARIO capture {path}");
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

impl Scenario {
    fn role_name(&self) -> &'static str {
        match self.role {
            Role::Host => "host",
            Role::Friend => "friend",
        }
    }
    fn advance(&mut self, step: u32, now: f64) {
        info!("ONLINE_SCENARIO {} step {step}", self.role_name());
        self.step = step;
        self.step_at = now;
    }
}

fn on_events(
    mut events: EventReader<HubEvent>,
    client: Option<ResMut<HubClient>>,
    mut scenario: ResMut<Scenario>,
) {
    let Some(mut client) = client else { return };
    for event in events.read() {
        match (&event.request, &event.result) {
            (HubRequest::Register, Err(error)) if error.code == "username_taken" => {
                info!("ONLINE_SCENARIO account exists, signing in");
                client.login(&scenario.user, &scenario.pass);
            }
            (HubRequest::Register | HubRequest::Login, Err(error)) => {
                error!("ONLINE_SCENARIO auth failed: {}", error.message);
            }
            (HubRequest::FindMatch(_) | HubRequest::JoinMatch(_), Err(error)) => {
                error!("ONLINE_SCENARIO match failed: {}", error.message);
                scenario.step_at = 0.0;
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    time: Res<Time<bevy::time::Real>>,
    client: Option<ResMut<HubClient>>,
    session: Res<HubSession>,
    friends: Res<FriendsState>,
    mut forms: ResMut<HubForms>,
    mut drawer: ResMut<DrawerState>,
    state: Res<State<GameState>>,
    mut scenario: ResMut<Scenario>,
    mut exit: EventWriter<AppExit>,
) {
    let now = time.elapsed_secs_f64();
    if now > scenario.exit_after {
        info!("ONLINE_SCENARIO exit after {} s with {} captures", scenario.exit_after, scenario.captures.len());
        exit.write(AppExit::Success);
        return;
    }
    let Some(mut client) = client else { return };
    let in_menu = *state.get() == GameState::MainMenu;
    if in_menu && scenario.step < 6 {
        drawer.pinned = true;
    }
    match scenario.step {
        0 => match &*session {
            HubSession::LoggedIn(_) => scenario.advance(2, now),
            HubSession::LoggedOut if !scenario.started => {
                scenario.started = true;
                forms.username = scenario.user.clone();
                forms.email = scenario.email.clone();
                forms.password = scenario.pass.clone();
                forms.confirm = scenario.pass.clone();
                forms.mode = super::FormMode::Register;
                forms.version += 1;
                scenario.advance(1, now);
            }
            _ => {}
        },
        1 => {
            // Show the filled-in form once, then submit through the hub client.
            if now - scenario.step_at > 1.5 && !scenario.captures.contains(&"sidebar-register") {
                shot(&mut commands, &mut scenario, "sidebar-register");
                let (user, email, pass) = (scenario.user.clone(), scenario.email.clone(), scenario.pass.clone());
                forms.busy = true;
                forms.version += 1;
                client.register(&user, &email, &pass);
            }
            if session.logged_in() {
                scenario.advance(2, now);
            }
        }
        2 => {
            // Befriend the other player.
            if now - scenario.last_poll > 3.0 {
                scenario.last_poll = now;
                client.friends();
                match scenario.role {
                    Role::Host => {
                        let already = friends
                            .friends
                            .iter()
                            .chain(friends.pending_out.iter())
                            .any(|f| f.username.eq_ignore_ascii_case(&scenario.friend));
                        if !already {
                            if forms.search.is_empty() {
                                forms.search = scenario.friend.clone();
                                forms.version += 1;
                                let q = scenario.friend.clone();
                                client.search(&q);
                            } else {
                                shot(&mut commands, &mut scenario, "sidebar-search");
                                let f = scenario.friend.clone();
                                client.friend_request(&f);
                            }
                        }
                    }
                    Role::Friend => {
                        if let Some(request) = friends
                            .pending_in
                            .iter()
                            .find(|f| f.username.eq_ignore_ascii_case(&scenario.friend))
                        {
                            shot(&mut commands, &mut scenario, "sidebar-request");
                            let name = request.username.clone();
                            client.accept(&name);
                        }
                    }
                }
            }
            let accepted = friends
                .friends
                .iter()
                .any(|f| f.username.eq_ignore_ascii_case(&scenario.friend));
            if accepted {
                forms.search.clear();
                forms.version += 1;
                scenario.advance(3, now);
            }
        }
        3 => {
            if now - scenario.step_at > 2.5 {
                shot(&mut commands, &mut scenario, "sidebar-friends");
                scenario.advance(4, now);
            }
        }
        4 => {
            // Host finds a match; the friend joins the host's match from the sidebar.
            if !in_menu {
                scenario.advance(5, now);
                return;
            }
            if now - scenario.last_poll > 4.0 {
                scenario.last_poll = now;
                match scenario.role {
                    Role::Host => {
                        if !forms.busy {
                            forms.busy = true;
                            forms.version += 1;
                            client.find_match(scenario.mode.clone());
                        }
                    }
                    Role::Friend => {
                        client.friends();
                        let server = friends
                            .friends
                            .iter()
                            .find(|f| f.username.eq_ignore_ascii_case(&scenario.friend))
                            .and_then(|f| f.presence.server.clone())
                            .and_then(|s| s["server_id"].as_str().map(str::to_string));
                        if let Some(server_id) = server {
                            if !forms.busy {
                                shot(&mut commands, &mut scenario, "sidebar-join");
                                forms.busy = true;
                                forms.version += 1;
                                client.join_match(&server_id);
                            }
                        }
                    }
                }
            }
        }
        5 => {
            if *state.get() == GameState::Playing {
                let since = *scenario.playing_since.get_or_insert(now);
                let elapsed = now - since;
                if elapsed > 6.0 {
                    shot(&mut commands, &mut scenario, "gameplay-1");
                }
                if elapsed > 20.0 {
                    shot(&mut commands, &mut scenario, "gameplay-2");
                }
                if elapsed > 40.0 {
                    shot(&mut commands, &mut scenario, "gameplay-3");
                    scenario.advance(6, now);
                }
            } else if *state.get() == GameState::LoadFailed {
                error!("ONLINE_SCENARIO load failed");
                exit.write(AppExit::error());
            }
        }
        _ => {
            if *state.get() == GameState::Finished && !scenario.captures.contains(&"finished") {
                shot(&mut commands, &mut scenario, "finished");
            }
        }
    }
}
