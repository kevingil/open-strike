use bevy::{app::AppExit, prelude::*, ui::FocusPolicy};

use crate::game::{ui::menu::style::*, GameState};

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExitConfirmation>()
            .add_systems(Startup, setup_pause_menu)
            .add_systems(Update, handle_buttons)
            .add_systems(
                PostUpdate,
                sync_visibility.before(bevy::ui::UiSystem::Layout),
            )
            .add_systems(OnExit(GameState::MainMenu), reset_confirmation)
            .add_systems(OnExit(GameState::Paused), reset_confirmation);
    }
}

/// Shared by the shutdown icon and the pause menu. Only explicit confirmation exits.
#[derive(Resource, Default)]
pub struct ExitConfirmation {
    pub open: bool,
}

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct DialogContent {
    confirmation: bool,
}

#[derive(Component, Clone, Copy)]
enum DialogAction {
    Resume,
    MainMenu,
    RequestExit,
    CancelExit,
    ConfirmExit,
}

impl DialogAction {
    fn primary(self) -> bool {
        matches!(self, Self::Resume | Self::CancelExit)
    }

    fn destructive(self) -> bool {
        matches!(self, Self::ConfirmExit)
    }
}

fn setup_pause_menu(mut commands: Commands, server: Res<AssetServer>) {
    commands
        .spawn((
            PauseMenuRoot,
            Name::new("Pause and exit overlay"),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.025, 0.035, 0.46)),
            FocusPolicy::Block,
            GlobalZIndex(1000),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(420.),
                        max_width: Val::Percent(90.),
                        padding: UiRect::all(Val::Px(32.)),
                        border: UiRect::all(Val::Px(1.)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(GLASS),
                    BorderColor(Color::srgba(0.75, 0.86, 0.92, 0.22)),
                ))
                .with_children(|panel| {
                    panel
                        .spawn((
                            DialogContent {
                                confirmation: false,
                            },
                            content_node(),
                        ))
                        .with_children(|content| {
                            content.spawn(label("IN GAME", 13., ACCENT));
                            content.spawn(label("Paused", 38., WHITE));
                            content.spawn(label("Resume when you're ready.", 17., MUTED));
                            content.spawn(action_list()).with_children(|actions| {
                                spawn_action(actions, DialogAction::Resume, "Resume", "ESC");
                                spawn_action(actions, DialogAction::MainMenu, "Main Menu", "");
                                spawn_action(actions, DialogAction::RequestExit, "Exit Game", "");
                            });
                        });
                    panel
                        .spawn((DialogContent { confirmation: true }, content_node()))
                        .with_children(|content| {
                            content.spawn((
                                ImageNode {
                                    color: ACCENT,
                                    ..ImageNode::new(
                                        server.load("models/images/icon-shutdown-96.png"),
                                    )
                                },
                                Node {
                                    width: Val::Px(28.),
                                    height: Val::Px(28.),
                                    ..default()
                                },
                            ));
                            content.spawn(label("Are you sure?", 38., WHITE));
                            content.spawn(label(
                                "Exit Open Strike and return to your desktop?",
                                17.,
                                MUTED,
                            ));
                            content.spawn(action_list()).with_children(|actions| {
                                spawn_action(actions, DialogAction::CancelExit, "Cancel", "ESC");
                                spawn_action(actions, DialogAction::ConfirmExit, "Exit Game", "");
                            });
                        });
                });
        });
}

fn content_node() -> Node {
    Node {
        width: Val::Percent(100.),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(12.),
        ..default()
    }
}

fn action_list() -> Node {
    Node {
        margin: UiRect::top(Val::Px(16.)),
        row_gap: Val::Px(8.),
        ..content_node()
    }
}

fn spawn_action(parent: &mut ChildSpawnerCommands, action: DialogAction, text: &str, hint: &str) {
    parent
        .spawn((
            action,
            Button,
            Node {
                width: Val::Percent(100.),
                min_height: Val::Px(52.),
                padding: UiRect::axes(Val::Px(18.), Val::Px(12.)),
                justify_content: JustifyContent::SpaceBetween,
                ..button()
            },
            BackgroundColor(action_background(action, Interaction::None)),
            BorderColor(if action.primary() {
                ACCENT
            } else {
                Color::NONE
            }),
        ))
        .with_children(|button| {
            button.spawn(label(text, 20., WHITE));
            if !hint.is_empty() {
                button.spawn(label(hint, 12., MUTED));
            }
        });
}

fn action_background(action: DialogAction, interaction: Interaction) -> Color {
    if interaction == Interaction::Pressed {
        SELECTED
    } else if interaction == Interaction::Hovered {
        if action.destructive() {
            Color::srgba(0.75, 0.25, 0.22, 0.45)
        } else {
            GLASS_SELECTED
        }
    } else if action.primary() {
        GLASS_SELECTED
    } else if action.destructive() {
        Color::srgba(0.55, 0.19, 0.17, 0.28)
    } else {
        Color::srgba(1., 1., 1., 0.045)
    }
}

fn sync_visibility(
    state: Res<State<GameState>>,
    confirmation: Res<ExitConfirmation>,
    mut roots: Query<&mut Node, With<PauseMenuRoot>>,
    mut contents: Query<(&DialogContent, &mut Node), Without<PauseMenuRoot>>,
) {
    let visible = *state.get() == GameState::Paused
        || (*state.get() == GameState::MainMenu && confirmation.open);
    for mut root in &mut roots {
        root.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (content, mut node) in &mut contents {
        node.display = if content.confirmation == confirmation.open {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn handle_buttons(
    state: Res<State<GameState>>,
    mut confirmation: ResMut<ExitConfirmation>,
    mut buttons: Query<
        (
            &DialogAction,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit_events: EventWriter<AppExit>,
) {
    for (&action, interaction, mut background, mut border) in &mut buttons {
        background.0 = action_background(action, *interaction);
        border.0 = if action.primary() || *interaction != Interaction::None {
            if action.destructive() {
                Color::srgb(0.92, 0.49, 0.44)
            } else {
                ACCENT
            }
        } else {
            Color::NONE
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            DialogAction::CancelExit if confirmation.open => confirmation.open = false,
            DialogAction::ConfirmExit if confirmation.open => {
                exit_events.write(AppExit::Success);
            }
            _ if *state.get() != GameState::Paused || confirmation.open => {}
            DialogAction::Resume => next_state.set(GameState::Playing),
            DialogAction::MainMenu => next_state.set(GameState::MainMenu),
            DialogAction::RequestExit => confirmation.open = true,
            _ => {}
        }
    }
}

fn reset_confirmation(mut confirmation: ResMut<ExitConfirmation>) {
    confirmation.open = false;
}
