use super::{
    scene_definition::menu_scene,
    start_button::{self, StartButtonMaterial},
    style::*,
    LocalMatchOption, MenuPage, MenuTab,
};
use crate::game::{
    config::{GameConfig, MapId},
    GameState,
};
use bevy::prelude::*;
pub struct PlayTabPlugin;
#[derive(Component)]
pub(super) struct StartGameButton;
#[derive(Component)]
struct MapCard;
#[derive(Component)]
struct MatchDetails;
#[derive(Resource, Default)]
struct StartPending(bool);
impl Plugin for PlayTabPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StartPending>()
            .add_plugins(UiMaterialPlugin::<StartButtonMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                OnEnter(GameState::MainMenu),
                |mut pending: ResMut<StartPending>| pending.0 = false,
            )
            .add_systems(
                Update,
                (interact, details, start_button::animate)
                    .run_if(in_state(GameState::MainMenu).and(in_state(MenuTab::Play))),
            );
    }
}
fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut button_materials: ResMut<Assets<StartButtonMaterial>>,
) {
    commands
        .spawn((
            MenuPage(MenuTab::Play),
            Node {
                padding: UiRect::ZERO,
                row_gap: Val::Px(0.),
                ..page()
            },
            BackgroundColor(Color::srgba(0.16, 0.19, 0.22, 0.68)),
            GlobalZIndex(150),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(58.),
                    flex_shrink: 0.,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::bottom(Val::Px(1.)),
                    ..default()
                },
                BorderColor(Color::srgba(1., 1., 1., 0.14)),
            ))
            .with_children(|modes| {
                modes
                    .spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(20.), Val::Px(9.)),
                            ..button()
                        },
                        BackgroundColor(GLASS_SELECTED),
                        BorderColor(ACCENT),
                    ))
                    .with_child(label(LocalMatchOption::LABEL.to_uppercase(), 17., ACCENT));
            });
            root.spawn(Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(3, 1.),
                flex_grow: 1.,
                min_height: Val::Px(0.),
                padding: UiRect::all(Val::VMin(4.)),
                column_gap: Val::VMin(3.),
                align_items: AlignItems::Start,
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(18.),
                    width: Val::Percent(100.),
                    max_width: Val::Px(280.),
                    min_width: Val::Px(0.),
                    padding: UiRect::top(Val::Px(12.)),
                    ..default()
                })
                .with_children(|info| {
                    info.spawn(label("Deathmatch", 28., WHITE));
                    info.spawn((MatchDetails, label("", 17., WHITE)));
                    info.spawn(label(
                        "Eliminate the opposing team.\nRespawn and rejoin the fight.",
                        16.,
                        MUTED,
                    ));
                });
                row.spawn((
                    MapCard,
                    Button,
                    Node {
                        width: Val::VMin(30.),
                        height: Val::VMin(49.),
                        max_width: Val::Percent(100.),
                        justify_self: JustifySelf::Center,
                        border: UiRect::all(Val::Px(2.)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::End,
                        ..default()
                    },
                    BackgroundColor(PANEL),
                    BorderColor(ACCENT),
                ))
                .with_children(|card| {
                    card.spawn((
                        ImageNode::new(server.load(menu_scene(&MapId::Dust2).thumbnail)),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            ..default()
                        },
                    ));
                    card.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(10.),
                            right: Val::Px(10.),
                            padding: UiRect::all(Val::Px(8.)),
                            ..default()
                        },
                        BackgroundColor(SELECTED),
                    ))
                    .with_child(label("SELECTED", 11., WHITE));
                    card.spawn((
                        Node {
                            padding: UiRect::all(Val::Px(18.)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.),
                            ..default()
                        },
                        BackgroundColor(INK),
                    ))
                    .with_children(|name| {
                        name.spawn(label("DUST 2", 28., WHITE));
                        name.spawn(label("DEATHMATCH", 12., ACCENT));
                    });
                });
            });
            root.spawn((
                Node {
                    width: Val::Percent(100.),
                    flex_shrink: 0.,
                    justify_content: JustifyContent::End,
                    padding: UiRect::axes(Val::VMin(4.), Val::Px(16.)),
                    border: UiRect::top(Val::Px(1.)),
                    ..default()
                },
                BorderColor(Color::srgba(1., 1., 1., 0.14)),
            ))
            .with_children(|footer| {
                footer
                    .spawn((
                        StartGameButton,
                        Button,
                        Node {
                            width: Val::Px(260.),
                            max_width: Val::Percent(100.),
                            height: Val::Px(52.),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        MaterialNode(button_materials.add(StartButtonMaterial::default())),
                        BoxShadow::new(
                            start_button::LABEL_COLOR.with_alpha(0.),
                            Val::Px(0.),
                            Val::Px(0.),
                            Val::Px(1.),
                            Val::Px(9.),
                        ),
                    ))
                    .with_child(label("START GAME", 21., start_button::LABEL_COLOR));
            });
        });
}
fn details(config: Res<GameConfig>, mut labels: Query<&mut Text, With<MatchDetails>>) {
    if !config.is_changed() {
        return;
    }
    let time = config
        .match_settings
        .time_limit
        .map(|v| format!("{} min", v.as_secs() / 60))
        .unwrap_or("Unlimited time".into());
    let score = config
        .match_settings
        .score_limit
        .map(|v| format!("First to {v} kills"))
        .unwrap_or("No score limit".into());
    for mut text in &mut labels {
        **text = format!("Local bots · 3v3\n{time} · {score}");
    }
}
fn interact(
    mut config: ResMut<GameConfig>,
    mut pending: ResMut<StartPending>,
    mut next: ResMut<NextState<GameState>>,
    cards: Query<&Interaction, (Changed<Interaction>, With<MapCard>)>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<StartGameButton>)>,
) {
    if cards.iter().any(|i| *i == Interaction::Pressed) {
        LocalMatchOption::select(&mut config);
    }
    if !pending.0
        && LocalMatchOption::selected(&config)
        && buttons.iter().any(|i| *i == Interaction::Pressed)
    {
        pending.0 = true;
        next.set(GameState::Loading);
    }
}
