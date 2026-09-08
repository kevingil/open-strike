//! A local view only: no simulated online presence or social service.
use super::{style::*, MenuPage};
use crate::game::{ui::pause_menu::ExitConfirmation, GameState};
use bevy::{prelude::*, window::PrimaryWindow};
pub struct FriendsDrawerPlugin;
#[derive(Resource)]
pub(super) struct DrawerState {
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
impl Plugin for FriendsDrawerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerState>()
            .add_systems(Startup, setup)
            .add_systems(PostStartup, setup_page_clip)
            .add_systems(Update, update)
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
            .with_child(label("LP", 16., WHITE));
            root.spawn((
                DrawerContent,
                Node {
                    display: Display::None,
                    width: Val::Percent(100.),
                    padding: UiRect {
                        left: Val::Px(22.),
                        right: Val::Px(66.),
                        top: Val::Px(24.),
                        ..default()
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.),
                    ..default()
                },
                bevy::ui::FocusPolicy::Pass,
            ))
            .with_children(|content| {
                content.spawn(label("Local Player", 19., WHITE));
                content.spawn(label("Local profile", 12., MUTED));
                content.spawn((
                    label("FRIENDS", 14., MUTED),
                    Node {
                        margin: UiRect::top(Val::Px(48.)),
                        ..default()
                    },
                ));
                content.spawn(label("no friends", 17., WHITE));
            });
        });
}
fn update(
    time: Res<Time>,
    game: Res<State<GameState>>,
    confirmation: Res<ExitConfirmation>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
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
    let expanded = 300_f32.min(window.width() * 0.75);
    let width = 56. + (expanded - 56.) * state.progress;
    let hover = window
        .cursor_position()
        .is_some_and(|p| p.x >= window.width() - width);
    if !hover {
        state.dismissed = false;
    }
    // This panel has a single focus target. Tab focuses it; another Tab leaves it.
    if keys.just_pressed(KeyCode::Tab) {
        state.focused = !state.focused;
        state.dismissed = false;
    }
    if clicks.iter().any(|i| *i == Interaction::Pressed)
        || (state.focused
            && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)))
    {
        state.pinned = !state.pinned;
        state.dismissed = !state.pinned;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.pinned = false;
        state.focused = false;
        state.dismissed = true;
    }
    if hover {
        state.away = 0.;
    } else {
        state.away += time.delta_secs();
    }
    let open = !state.dismissed && (hover || state.away < 0.12 || state.pinned || state.focused);
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
