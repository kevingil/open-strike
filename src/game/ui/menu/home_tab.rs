use super::{style::*, MenuPage, MenuTab};
use bevy::prelude::*;
pub struct HomeTabPlugin;
impl Plugin for HomeTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}
fn setup(mut commands: Commands) {
    commands.spawn((
        MenuPage(MenuTab::Home),
        label("OPEN STRIKE", 18., WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.),
            bottom: Val::Px(36.),
            ..default()
        },
        bevy::ui::FocusPolicy::Pass,
        GlobalZIndex(100),
    ));
    commands
        .spawn((
            MenuPage(MenuTab::Home),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                bottom: Val::Vh(3.),
                width: Val::Percent(100.),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.),
                ..default()
            },
            bevy::ui::FocusPolicy::Pass,
            GlobalZIndex(100),
        ))
        .with_children(|root| {
            root.spawn(label("LOCAL PLAYER", 16., WHITE));
            root.spawn(label("DUST 2", 11., MUTED));
        });
}
