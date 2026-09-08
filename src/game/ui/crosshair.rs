use crate::game::{matchplay::Combatant, player::player::LocalPlayer, GameState};
use bevy::{prelude::*, ui::FocusPolicy};
#[derive(Component)]
pub struct CrosshairRoot;
pub struct CrosshairPlugin;
impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_crosshair)
            .add_systems(OnExit(GameState::Playing), cleanup_crosshair)
            .add_systems(Update, visibility);
    }
}
fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn((
            CrosshairRoot,
            FocusPolicy::Pass,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(50),
        ))
        .with_children(|root| {
            root.spawn((
                FocusPolicy::Pass,
                Node {
                    width: Val::Vw(1.3),
                    height: Val::Vw(1.3),
                    ..default()
                },
            ))
            .with_children(|cross| {
                for (left, top, width, height) in [
                    (0.0, 44.0, 33.0, 12.0),
                    (67.0, 44.0, 33.0, 12.0),
                    (44.0, 0.0, 12.0, 33.0),
                    (44.0, 67.0, 12.0, 33.0),
                ] {
                    cross.spawn((
                        FocusPolicy::Pass,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(left),
                            top: Val::Percent(top),
                            width: Val::Percent(width),
                            height: Val::Percent(height),
                            border: UiRect::all(Val::Px(0.4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.9, 0.8)),
                        BorderColor(Color::BLACK),
                    ));
                }
            });
        });
}
fn visibility(
    actors: Query<&Combatant, With<LocalPlayer>>,
    mut roots: Query<&mut Visibility, With<CrosshairRoot>>,
) {
    for mut visible in &mut roots {
        *visible = if actors.single().is_ok_and(Combatant::alive) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
fn cleanup_crosshair(mut commands: Commands, roots: Query<Entity, With<CrosshairRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
