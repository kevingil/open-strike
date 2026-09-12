use super::{style::*, MenuPage, MenuTab};
use crate::game::hub::{HubHealth, HubSession};
use bevy::prelude::*;
pub struct HomeTabPlugin;
#[derive(Component)]
struct ProfileName;
#[derive(Component)]
struct ProfileLine;
impl Plugin for HomeTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, profile_labels);
    }
}
fn profile_labels(
    session: Res<HubSession>,
    health: Res<HubHealth>,
    mut names: Query<&mut Text, (With<ProfileName>, Without<ProfileLine>)>,
    mut lines: Query<&mut Text, (With<ProfileLine>, Without<ProfileName>)>,
) {
    if !session.is_changed() && !health.is_changed() {
        return;
    }
    let (name, line) = match &*session {
        HubSession::LoggedIn(account) => (
            account.username.to_uppercase(),
            if health.hub_name.is_empty() {
                "ONLINE".to_string()
            } else {
                health.hub_name.to_uppercase()
            },
        ),
        HubSession::Offline(Some(account)) => (account.username.to_uppercase(), "HUB OFFLINE".into()),
        _ => ("LOCAL PLAYER".into(), "DUST 2".into()),
    };
    for mut text in &mut names {
        **text = name.clone();
    }
    for mut text in &mut lines {
        **text = line.clone();
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
            root.spawn((ProfileName, label("LOCAL PLAYER", 16., WHITE)));
            root.spawn((ProfileLine, label("DUST 2", 11., MUTED)));
        });
}
