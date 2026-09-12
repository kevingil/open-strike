//! Profile and friends sidebar. Signed out it offers login and account creation;
//! signed in it shows friends, requests and username search. When the hub is
//! unreachable it shows a badge and disables every social control.
use super::{
    style::*,
    text_field::{spawn_text_field, FieldChanged, FieldFocus, FieldId, FieldSubmit},
    MenuPage,
};
use crate::game::{
    hub::{FormMode, FriendsState, HubClient, HubForms, HubHealth, HubSession},
    ui::pause_menu::ExitConfirmation,
    GameState,
};
use bevy::{prelude::*, window::PrimaryWindow};
pub struct FriendsDrawerPlugin;
#[derive(Resource)]
pub struct DrawerState {
    pub pinned: bool,
    pub focused: bool,
    progress: f32,
    away: f32,
    dismissed: bool,
}
impl Default for DrawerState {
    fn default() -> Self {
        Self {
            pinned: false,
            focused: false,
            progress: 0.,
            away: 0.12,
            dismissed: false,
        }
    }
}
#[derive(Component)]
pub(super) struct DrawerRoot;
/// Clip menu chrome and pages beneath the drawer without resizing their layout.
#[derive(Component)]
pub(super) struct DrawerClip;
#[derive(Component)]
struct DrawerContent;
#[derive(Component)]
struct ProfileButton;
#[derive(Component)]
struct ProfileInitials;
#[derive(Component)]
struct ProfileBadge(Badge);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Badge {
    Offline,
    Requests,
}

#[derive(Component, Clone, Debug)]
enum DrawerAction {
    ShowLogin,
    ShowRegister,
    Cancel,
    SubmitLogin,
    SubmitRegister,
    Logout,
    AddFriend(String),
    Accept(String),
    Decline(String),
    Remove(String),
    Join(String),
    DismissNotice,
}

#[derive(Default)]
struct Rebuilt {
    key: String,
}

#[derive(Resource, Default)]
struct SearchDebounce {
    due: Option<f64>,
}

impl Plugin for FriendsDrawerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerState>()
            .init_resource::<SearchDebounce>()
            .add_systems(Startup, setup)
            .add_systems(PostStartup, setup_page_clip)
            .add_systems(
                Update,
                (update, rebuild, actions, submits, search_typing, badges),
            )
            .add_systems(
                PostUpdate,
                clip_to_drawer.before(bevy::ui::UiSystem::Layout),
            )
            .add_systems(
                OnExit(GameState::MainMenu),
                |mut state: ResMut<DrawerState>| *state = default(),
            );
    }
}
fn setup_page_clip(mut commands: Commands, pages: Query<Entity, With<MenuPage>>) {
    let clip = commands
        .spawn((
            DrawerClip,
            Name::new("Menu pages clipped behind Friends"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(56.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                overflow: Overflow::clip_x(),
                ..default()
            },
            bevy::ui::FocusPolicy::Pass,
        ))
        .id();
    // Pages keep their original viewport-sized containing block as the clip narrows.
    let viewport = commands
        .spawn((
            ChildOf(clip),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Px(0.),
                width: Val::Vw(100.),
                height: Val::Vh(100.),
                ..default()
            },
            bevy::ui::FocusPolicy::Pass,
        ))
        .id();
    for page in &pages {
        commands.entity(page).insert(ChildOf(viewport));
    }
}

fn clip_to_drawer(
    drawers: Query<&Node, With<DrawerRoot>>,
    mut clips: Query<&mut Node, (With<DrawerClip>, Without<DrawerRoot>)>,
) {
    let Ok(drawer) = drawers.single() else {
        return;
    };
    for mut clip in &mut clips {
        clip.right = drawer.width;
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            DrawerRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                width: Val::Px(56.),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(GLASS),
            GlobalZIndex(230),
            bevy::ui::FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn((
                ProfileButton,
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(8.),
                    top: Val::Px(16.),
                    width: Val::Px(40.),
                    height: Val::Px(40.),
                    border: UiRect::all(Val::Px(1.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(SELECTED),
                BorderColor(ACCENT),
            ))
            .with_children(|button| {
                button.spawn((ProfileInitials, label("—", 16., WHITE)));
                for (badge, color, top) in [
                    (Badge::Offline, Color::srgb(0.85, 0.28, 0.24), -4.),
                    (Badge::Requests, ACCENT, 30.),
                ] {
                    button.spawn((
                        ProfileBadge(badge),
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(-4.),
                            top: Val::Px(top),
                            width: Val::Px(12.),
                            height: Val::Px(12.),
                            display: Display::None,
                            border: UiRect::all(Val::Px(2.)),
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderColor(INK),
                        BorderRadius::all(Val::Px(6.)),
                    ));
                }
            });
            root.spawn((
                DrawerContent,
                Node {
                    display: Display::None,
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    padding: UiRect {
                        left: Val::Px(22.),
                        right: Val::Px(66.),
                        top: Val::Px(24.),
                        bottom: Val::Px(24.),
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                bevy::ui::FocusPolicy::Pass,
            ));
        });
}

fn update(
    time: Res<Time>,
    game: Res<State<GameState>>,
    confirmation: Res<ExitConfirmation>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<FieldFocus>,
    clicks: Query<&Interaction, (Changed<Interaction>, With<ProfileButton>)>,
    mut state: ResMut<DrawerState>,
    mut roots: Query<&mut Node, With<DrawerRoot>>,
    mut content: Query<&mut Node, (With<DrawerContent>, Without<DrawerRoot>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut root) = roots.single_mut() else {
        return;
    };
    let active = *game.get() == GameState::MainMenu;
    root.display = if active { Display::Flex } else { Display::None };
    if !active || confirmation.open {
        return;
    }
    let expanded = 320_f32.min(window.width() * 0.75);
    let width = 56. + (expanded - 56.) * state.progress;
    let hover = window
        .cursor_position()
        .is_some_and(|p| p.x >= window.width() - width);
    if !hover {
        state.dismissed = false;
    }
    let typing = focus.0.is_some();
    // This panel has a single focus target. Tab focuses it; another Tab leaves it.
    if keys.just_pressed(KeyCode::Tab) && !typing {
        state.focused = !state.focused;
        state.dismissed = false;
    }
    if clicks.iter().any(|i| *i == Interaction::Pressed)
        || (state.focused
            && !typing
            && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)))
    {
        state.pinned = !state.pinned;
        state.dismissed = !state.pinned;
    }
    if keys.just_pressed(KeyCode::Escape) && !typing {
        state.pinned = false;
        state.focused = false;
        state.dismissed = true;
    }
    if hover {
        state.away = 0.;
    } else {
        state.away += time.delta_secs();
    }
    // Keep the drawer open while a field inside it has keyboard focus.
    let open =
        !state.dismissed && (hover || state.away < 0.12 || state.pinned || state.focused || typing);
    let delta = time.delta_secs() / 0.2;
    state.progress = (state.progress + if open { delta } else { -delta }).clamp(0., 1.);
    root.width = Val::Px(56. + (expanded - 56.) * state.progress);
    for mut node in &mut content {
        node.display = if state.progress > 0.75 {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn badges(
    session: Res<HubSession>,
    health: Res<HubHealth>,
    friends: Res<FriendsState>,
    mut initials: Query<&mut Text, With<ProfileInitials>>,
    mut badges: Query<(&ProfileBadge, &mut Node)>,
) {
    if let Ok(mut text) = initials.single_mut() {
        let value = match session.account() {
            Some(account) => account
                .username
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase(),
            None => "—".into(),
        };
        if **text != value {
            **text = value;
        }
    }
    let offline = !health.online && !matches!(*session, HubSession::NoHub);
    for (badge, mut node) in &mut badges {
        let shown = match badge.0 {
            Badge::Offline => offline,
            Badge::Requests => !offline && session.logged_in() && !friends.pending_in.is_empty(),
        };
        node.display = if shown { Display::Flex } else { Display::None };
    }
}

fn section(parent: &mut ChildSpawnerCommands, text: &str, top: f32) {
    parent.spawn((
        label(text, 13., MUTED),
        Node {
            margin: UiRect::top(Val::Px(top)),
            ..default()
        },
    ));
}

fn action_button(
    parent: &mut ChildSpawnerCommands,
    action: DrawerAction,
    text: &str,
    primary: bool,
    enabled: bool,
) {
    let mut node = Node {
        width: Val::Percent(100.),
        min_height: Val::Px(38.),
        padding: UiRect::axes(Val::Px(14.), Val::Px(8.)),
        ..button()
    };
    node.justify_content = JustifyContent::Center;
    let mut entity = parent.spawn((
        action,
        node,
        BackgroundColor(if !enabled {
            Color::srgba(1., 1., 1., 0.03)
        } else if primary {
            GLASS_SELECTED
        } else {
            Color::srgba(1., 1., 1., 0.06)
        }),
        BorderColor(if primary && enabled {
            ACCENT
        } else {
            Color::NONE
        }),
    ));
    if enabled {
        entity.insert(Button);
    }
    entity.with_child(label(
        text,
        15.,
        if enabled {
            WHITE
        } else {
            MUTED.with_alpha(0.5)
        },
    ));
}

fn small_button(
    parent: &mut ChildSpawnerCommands,
    action: DrawerAction,
    text: &str,
    enabled: bool,
) {
    let mut entity = parent.spawn((
        action,
        Node {
            padding: UiRect::axes(Val::Px(9.), Val::Px(4.)),
            border: UiRect::all(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(Color::srgba(1., 1., 1., if enabled { 0.06 } else { 0.02 })),
        BorderColor(if enabled {
            ACCENT.with_alpha(0.6)
        } else {
            Color::NONE
        }),
    ));
    if enabled {
        entity.insert(Button);
    }
    entity.with_child(label(
        text,
        12.,
        if enabled {
            ACCENT
        } else {
            MUTED.with_alpha(0.5)
        },
    ));
}

fn person_row(
    parent: &mut ChildSpawnerCommands,
    entry: &crate::game::hub::FriendEntry,
    online: bool,
    kind: RowKind,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.), Val::Px(7.)),
                column_gap: Val::Px(8.),
                ..default()
            },
            BackgroundColor(Color::srgba(1., 1., 1., 0.04)),
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(0.),
                flex_shrink: 1.,
                ..default()
            })
            .with_children(|names| {
                names.spawn(label(entry.username.clone(), 15., WHITE));
                let presence = &entry.presence;
                let status = if !online {
                    "unknown".to_string()
                } else if let Some(server) = presence.server.as_ref().filter(|s| !s.is_null()) {
                    let mode = server["mode"].as_str().unwrap_or("");
                    let mode = crate::game::config::GameMode::from_key(mode)
                        .map(|m| m.name())
                        .unwrap_or("match");
                    let map =
                        crate::game::config::MapId::from_key(server["map"].as_str().unwrap_or(""))
                            .map(|m| m.name())
                            .unwrap_or("");
                    format!("In match · {map} {mode}")
                } else {
                    match presence.state.as_str() {
                        "online" | "in_menu" => "Online".into(),
                        "in_match" => "In match".into(),
                        _ => "Offline".into(),
                    }
                };
                names.spawn(label(
                    status,
                    11.,
                    if presence.state == "offline" || !online {
                        MUTED.with_alpha(0.7)
                    } else {
                        ACCENT
                    },
                ));
            });
            row.spawn(Node {
                column_gap: Val::Px(6.),
                flex_shrink: 0.,
                ..default()
            })
            .with_children(|buttons| match kind {
                RowKind::Friend => {
                    if let Some(server) = entry.presence.server.as_ref().filter(|s| !s.is_null()) {
                        let id = server["server_id"].as_str().unwrap_or("").to_string();
                        small_button(buttons, DrawerAction::Join(id), "JOIN", online);
                    }
                    small_button(
                        buttons,
                        DrawerAction::Remove(entry.username.clone()),
                        "×",
                        online,
                    );
                }
                RowKind::PendingIn => {
                    small_button(
                        buttons,
                        DrawerAction::Accept(entry.username.clone()),
                        "ACCEPT",
                        online,
                    );
                    small_button(
                        buttons,
                        DrawerAction::Decline(entry.username.clone()),
                        "×",
                        online,
                    );
                }
                RowKind::PendingOut => {
                    small_button(
                        buttons,
                        DrawerAction::Remove(entry.username.clone()),
                        "SENT",
                        false,
                    );
                }
                RowKind::Search => match entry.relationship.as_str() {
                    "friends" => {
                        small_button(buttons, DrawerAction::DismissNotice, "FRIENDS", false)
                    }
                    "pending_out" => {
                        small_button(buttons, DrawerAction::DismissNotice, "SENT", false)
                    }
                    "pending_in" => small_button(
                        buttons,
                        DrawerAction::Accept(entry.username.clone()),
                        "ACCEPT",
                        online,
                    ),
                    _ => small_button(
                        buttons,
                        DrawerAction::AddFriend(entry.username.clone()),
                        "ADD",
                        online,
                    ),
                },
            });
        });
}

#[derive(Clone, Copy)]
enum RowKind {
    Friend,
    PendingIn,
    PendingOut,
    Search,
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    session: Res<HubSession>,
    health: Res<HubHealth>,
    friends: Res<FriendsState>,
    forms: Res<HubForms>,
    time: Res<Time<Real>>,
    client: Option<Res<HubClient>>,
    content: Query<Entity, With<DrawerContent>>,
    mut cache: Local<Rebuilt>,
) {
    let Ok(root) = content.single() else { return };
    let offline = !health.online && !matches!(*session, HubSession::NoHub);
    let retry = health.retry_in(time.elapsed_secs_f64()).ceil() as i64;
    let key = format!(
        "{:?}|{}|{}|{}|{:?}|{}|{}",
        *session,
        health.online,
        friends.version,
        forms.version,
        forms.mode,
        forms.busy,
        if offline { retry } else { 0 }
    );
    if cache.key == key {
        return;
    }
    cache.key = key;
    commands.entity(root).despawn_related::<Children>();
    let hub_name = if health.hub_name.is_empty() {
        client
            .as_ref()
            .map(|c| {
                c.url
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .to_string()
            })
            .unwrap_or_default()
    } else {
        health.hub_name.clone()
    };
    commands.entity(root).with_children(|content| {
        match &*session {
            HubSession::LoggedIn(account) | HubSession::Offline(Some(account)) => {
                content
                    .spawn(Node {
                        width: Val::Percent(100.),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn(label(account.username.clone(), 19., WHITE));
                        small_button(header, DrawerAction::Logout, "LOG OUT", !offline);
                    });
                content.spawn(label(format!("{} · {hub_name}", account.role), 12., MUTED));
            }
            HubSession::Connecting => {
                content.spawn(label("Signing in…", 19., WHITE));
                content.spawn(label(hub_name.clone(), 12., MUTED));
            }
            HubSession::NoHub => {
                content.spawn(label("Not signed in", 19., WHITE));
                content.spawn(label("No hub configured", 12., MUTED));
                content.spawn(label(
                    "Set STRIKE_HUB_URL to the address of a hub to play online.",
                    12.,
                    MUTED.with_alpha(0.8),
                ));
            }
            HubSession::LoggedOut | HubSession::Offline(None) => {
                content.spawn(label("Not signed in", 19., WHITE));
                content.spawn(label(format!("Hub: {hub_name}"), 12., MUTED));
            }
        }
        if offline {
            content
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
                        margin: UiRect::top(Val::Px(4.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.85, 0.28, 0.24, 0.25)),
                ))
                .with_child(label(
                    format!("HUB OFFLINE · retrying in {retry}s"),
                    12.,
                    Color::srgb(1.0, 0.72, 0.68),
                ));
        }
        if let Some(notice) = &forms.notice {
            content
                .spawn((
                    Node {
                        width: Val::Percent(100.),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(10.), Val::Px(6.)),
                        ..default()
                    },
                    BackgroundColor(GLASS_SELECTED),
                ))
                .with_children(|row| {
                    row.spawn(label(notice.clone(), 12., WHITE));
                    small_button(row, DrawerAction::DismissNotice, "OK", true);
                });
        }
        let enabled = !offline && !forms.busy;
        match (&*session, forms.mode) {
            (HubSession::LoggedIn(_) | HubSession::Offline(Some(_)), _) => {
                let requests = friends.pending_in.len();
                let online_count = friends
                    .friends
                    .iter()
                    .filter(|f| f.presence.state != "offline")
                    .count();
                content
                    .spawn(Node {
                        width: Val::Percent(100.),
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::top(Val::Px(28.)),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(label("FRIENDS", 13., MUTED));
                        row.spawn(label(format!("{online_count} online"), 12., MUTED));
                    });
                spawn_text_field(content, FieldId::Search, "search by username…");
                if let Some(error) = &forms.error {
                    content.spawn(label(error.clone(), 12., Color::srgb(1.0, 0.6, 0.55)));
                }
                if !forms.search.trim().is_empty() && forms.search.trim().chars().count() >= 2 {
                    if friends.search_results.is_empty() {
                        content.spawn(label("No players match", 13., MUTED));
                    }
                    for entry in &friends.search_results {
                        person_row(content, entry, !offline, RowKind::Search);
                    }
                } else {
                    if friends.friends.is_empty() && friends.pending_out.is_empty() {
                        content.spawn(label(
                            "No friends yet. Search a username above.",
                            13.,
                            MUTED,
                        ));
                    }
                    for entry in &friends.friends {
                        person_row(content, entry, !offline, RowKind::Friend);
                    }
                    for entry in &friends.pending_out {
                        person_row(content, entry, !offline, RowKind::PendingOut);
                    }
                    if requests > 0 {
                        section(content, &format!("REQUESTS ({requests})"), 14.);
                        for entry in &friends.pending_in {
                            person_row(content, entry, !offline, RowKind::PendingIn);
                        }
                    }
                }
            }
            (HubSession::NoHub, _) | (HubSession::Connecting, _) => {}
            (_, FormMode::Idle) => {
                content.spawn(Node {
                    height: Val::Px(12.),
                    ..default()
                });
                action_button(content, DrawerAction::ShowLogin, "LOG IN", true, enabled);
                action_button(
                    content,
                    DrawerAction::ShowRegister,
                    "CREATE ACCOUNT",
                    false,
                    enabled,
                );
                content.spawn((
                    label(
                        "Local play works without an account.",
                        12.,
                        MUTED.with_alpha(0.8),
                    ),
                    Node {
                        margin: UiRect::top(Val::Px(20.)),
                        ..default()
                    },
                ));
            }
            (_, FormMode::Login) => {
                section(content, "LOG IN", 12.);
                spawn_text_field(content, FieldId::Identifier, "username or email");
                spawn_text_field(content, FieldId::Password, "password");
                if let Some(error) = &forms.error {
                    content.spawn(label(error.clone(), 12., Color::srgb(1.0, 0.6, 0.55)));
                }
                action_button(
                    content,
                    DrawerAction::SubmitLogin,
                    if forms.busy {
                        "SIGNING IN…"
                    } else {
                        "LOG IN"
                    },
                    true,
                    enabled,
                );
                action_button(content, DrawerAction::Cancel, "BACK", false, !forms.busy);
                content.spawn(label(
                    "Forgot your password? Ask a hub administrator for a reset code.",
                    11.,
                    MUTED.with_alpha(0.7),
                ));
            }
            (_, FormMode::Register) => {
                section(content, "CREATE ACCOUNT", 12.);
                spawn_text_field(content, FieldId::Username, "username");
                content.spawn(label(
                    "3 to 20 letters, digits or underscores",
                    11.,
                    MUTED.with_alpha(0.7),
                ));
                spawn_text_field(content, FieldId::Email, "email (for account recovery)");
                spawn_text_field(content, FieldId::Password, "password (8+ characters)");
                spawn_text_field(content, FieldId::Confirm, "confirm password");
                if let Some(error) = &forms.error {
                    content.spawn(label(error.clone(), 12., Color::srgb(1.0, 0.6, 0.55)));
                }
                action_button(
                    content,
                    DrawerAction::SubmitRegister,
                    if forms.busy {
                        "CREATING…"
                    } else {
                        "CREATE ACCOUNT"
                    },
                    true,
                    enabled,
                );
                action_button(content, DrawerAction::Cancel, "BACK", false, !forms.busy);
            }
        }
    });
}

fn submit_login(client: &mut HubClient, forms: &mut HubForms) {
    let identifier = forms.identifier.trim().to_string();
    if identifier.is_empty() || forms.password.is_empty() {
        forms.error = Some("Enter your username or email and password".into());
        forms.version += 1;
        return;
    }
    forms.busy = true;
    forms.error = None;
    forms.version += 1;
    client.login(&identifier, &forms.password);
}

fn submit_register(client: &mut HubClient, forms: &mut HubForms) {
    let username = forms.username.trim().to_string();
    let email = forms.email.trim().to_string();
    if username.is_empty() || email.is_empty() || forms.password.is_empty() {
        forms.error = Some("Fill in username, email and password".into());
    } else if forms.password != forms.confirm {
        forms.error = Some("Passwords do not match".into());
    } else {
        forms.busy = true;
        forms.error = None;
        forms.version += 1;
        client.register(&username, &email, &forms.password);
        return;
    }
    forms.version += 1;
}

fn actions(
    client: Option<ResMut<HubClient>>,
    mut forms: ResMut<HubForms>,
    mut focus: ResMut<FieldFocus>,
    mut friends: ResMut<FriendsState>,
    buttons: Query<(&Interaction, &DrawerAction), (Changed<Interaction>, With<Button>)>,
) {
    let Some(mut client) = client else { return };
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            DrawerAction::ShowLogin => {
                forms.mode = FormMode::Login;
                forms.error = None;
                focus.0 = Some(FieldId::Identifier);
            }
            DrawerAction::ShowRegister => {
                forms.mode = FormMode::Register;
                forms.error = None;
                focus.0 = Some(FieldId::Username);
            }
            DrawerAction::Cancel => {
                forms.mode = FormMode::Idle;
                forms.error = None;
                focus.0 = None;
            }
            DrawerAction::SubmitLogin => submit_login(&mut client, &mut forms),
            DrawerAction::SubmitRegister => submit_register(&mut client, &mut forms),
            DrawerAction::Logout => {
                client.logout();
                forms.search.clear();
                friends.search_results.clear();
            }
            DrawerAction::AddFriend(name) => client.friend_request(name),
            DrawerAction::Accept(name) => client.accept(name),
            DrawerAction::Decline(name) => client.decline(name),
            DrawerAction::Remove(name) => client.remove(name),
            DrawerAction::Join(server_id) => {
                forms.busy = true;
                forms.error = None;
                client.join_match(server_id);
            }
            DrawerAction::DismissNotice => {
                forms.notice = None;
            }
        }
        forms.version += 1;
    }
}

fn submits(
    client: Option<ResMut<HubClient>>,
    mut forms: ResMut<HubForms>,
    mut submitted: EventReader<FieldSubmit>,
) {
    let Some(mut client) = client else { return };
    for submit in submitted.read() {
        match (forms.mode, submit.0) {
            (FormMode::Login, FieldId::Identifier | FieldId::Password) => {
                submit_login(&mut client, &mut forms)
            }
            (FormMode::Register, _) if submit.0 != FieldId::Search => {
                submit_register(&mut client, &mut forms)
            }
            _ => {}
        }
    }
}

fn search_typing(
    client: Option<ResMut<HubClient>>,
    mut forms: ResMut<HubForms>,
    mut friends: ResMut<FriendsState>,
    mut changed: EventReader<FieldChanged>,
    mut debounce: ResMut<SearchDebounce>,
    time: Res<Time<Real>>,
) {
    let Some(mut client) = client else { return };
    let now = time.elapsed_secs_f64();
    for change in changed.read() {
        if change.0 == FieldId::Search {
            debounce.due = Some(now + 0.3);
            forms.version += 1;
        }
    }
    if debounce.due.is_some_and(|due| now >= due) {
        debounce.due = None;
        let query = forms.search.trim().to_string();
        if query.chars().count() >= 2 {
            friends.search_query_sent = query.clone();
            client.search(&query);
        } else {
            friends.search_query_sent.clear();
            friends.search_results.clear();
            friends.version += 1;
        }
    }
}
