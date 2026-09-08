use super::{friends_drawer::DrawerClip, style::*, MenuTab};
use crate::game::{ui::pause_menu::ExitConfirmation, GameState};
use bevy::prelude::*;
pub struct NavBarPlugin;
#[derive(Component)]
pub struct NavBarRoot;
#[derive(Component)]
struct NavBarClip;
#[derive(Component)]
pub(super) struct NavButton(pub MenuTab);
#[derive(Component)]
struct QuitGameButton;
#[derive(Component)]
struct QuitGameTooltip;
impl Plugin for NavBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, visibility)
            .add_systems(
                Update,
                (interact, interact_quit).run_if(in_state(GameState::MainMenu)),
            );
    }
}
fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let clip = commands
        .spawn((
            NavBarClip,
            DrawerClip,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(56.),
                height: Val::Px(HEADER),
                overflow: Overflow::clip_x(),
                ..default()
            },
            GlobalZIndex(220),
        ))
        .id();
    commands
        .spawn((
            NavBarRoot,
            ChildOf(clip),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                width: Val::Vw(100.),
                padding: UiRect::right(Val::Px(56.)),
                height: Val::Px(HEADER),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(GLASS),
        ))
        .with_children(|bar| {
            for (tab, name) in [
                (MenuTab::Home, "HOME"),
                (MenuTab::Inventory, "INVENTORY"),
                (MenuTab::Play, "PLAY"),
                (MenuTab::LoadOut, "LOAD OUT"),
                (MenuTab::Settings, "SETTINGS"),
            ] {
                let size = if tab == MenuTab::Play { 26. } else { 17. };
                bar.spawn((
                    NavButton(tab),
                    Button,
                    Node {
                        width: Val::Vw(11.),
                        min_width: Val::Px(92.),
                        max_width: Val::Px(164.),
                        height: Val::Px(HEADER),
                        border: UiRect::bottom(Val::Px(2.)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor(Color::NONE),
                ))
                .with_child(label(name, size, WHITE));
            }
            bar.spawn((
                QuitGameButton,
                Name::new("Quit Game"),
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.),
                    width: Val::Px(40.),
                    height: Val::Px(40.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_child((
                ImageNode {
                    color: WHITE,
                    ..ImageNode::new(server.load("models/images/icon-shutdown-96.png"))
                },
                Node {
                    width: Val::Px(26.),
                    height: Val::Px(26.),
                    ..default()
                },
            ));
            bar.spawn((
                QuitGameTooltip,
                label("Quit Game", 14., WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.),
                    top: Val::Px(HEADER),
                    padding: UiRect::axes(Val::Px(12.), Val::Px(8.)),
                    ..default()
                },
                BackgroundColor(INK),
                Visibility::Hidden,
            ));
        });
}
fn interact_quit(
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitGameButton>),
    >,
    mut tooltips: Query<&mut Visibility, With<QuitGameTooltip>>,
    mut confirmation: ResMut<ExitConfirmation>,
) {
    for (interaction, mut background) in &mut buttons {
        let hovered = *interaction != Interaction::None;
        background.0 = if hovered {
            Color::srgba(1., 1., 1., 0.08)
        } else {
            Color::NONE
        };
        for mut visibility in &mut tooltips {
            *visibility = if hovered {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        if *interaction == Interaction::Pressed {
            confirmation.open = true;
            for mut visibility in &mut tooltips {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
fn interact(
    tab: Res<State<MenuTab>>,
    mut next: ResMut<NextState<MenuTab>>,
    mut buttons: Query<(
        &NavButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (button, interaction, mut bg, mut border) in &mut buttons {
        if *interaction == Interaction::Pressed && button.0 != *tab.get() {
            next.set(button.0.clone());
        }
        let active = button.0 == *tab.get();
        bg.0 = if active {
            GLASS_SELECTED
        } else if *interaction == Interaction::Hovered {
            Color::srgba(1., 1., 1., 0.08)
        } else {
            Color::NONE
        };
        border.0 = if active { ACCENT } else { Color::NONE };
    }
}
fn visibility(state: Res<State<GameState>>, mut roots: Query<&mut Node, With<NavBarClip>>) {
    for mut node in &mut roots {
        node.display = if *state.get() == GameState::MainMenu {
            Display::Flex
        } else {
            Display::None
        };
    }
}
