//! First-person HUD. Every number is a view of authoritative match/actor state.
use super::radar::HudImages;
use crate::game::{
    config::{GameConfig, GameMode},
    level::level::{LoadedGameplayMapConfig, LoadingStatus},
    matchplay::{Combatant, MatchSession, Team},
    player::player::LocalPlayer,
    weapons::WeaponState,
    GameState,
};
use bevy::prelude::*;
mod assets;
mod kill_feed;
mod ranking;
mod weapon_slots;
use assets::HudArt;
use weapon_slots::AmmoOnly;
const CYAN: Color = Color::srgb(0.03, 0.86, 0.83);
const PANEL: Color = Color::srgba(0.02, 0.03, 0.04, 0.38);
#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct HudFont(f32);
#[derive(Component)]
enum Label {
    Health,
    Armor,
    Magazine,
    Reserve,
    Reload,
    Timer,
    Score,
    AliveT,
    AliveCt,
    Location,
    Mode,
}
#[derive(Component)]
enum Bar {
    Health,
    Armor,
    Reload,
}
#[derive(Component)]
struct StatusText;
pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::radar::RadarPlugin)
            .init_resource::<kill_feed::KillFeed>()
            .add_systems(Startup, assets::prepare)
            .add_systems(
                Startup,
                setup.after(super::radar::prepare).after(assets::prepare),
            )
            .add_systems(
                Update,
                (
                    update,
                    kill_feed::update,
                    ranking::update,
                    weapon_slots::update,
                    scale_fonts,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Loading), kill_feed::clear)
            .add_systems(OnEnter(GameState::MainMenu), kill_feed::clear);
    }
}
fn text(value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        HudFont(size),
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(1.0),
            color: Color::BLACK,
        },
    )
}
fn setup(mut commands: Commands, images: Res<HudImages>, art: Res<HudArt>) {
    commands
        .spawn((
            HudRoot,
            bevy::ui::FocusPolicy::Pass,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(180),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            ranking::spawn(root);
            kill_feed::spawn(root);
            weapon_slots::spawn(root, &art);
            root.spawn((
                Label::Location,
                text("", 14.0, Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.6),
                    top: Val::Percent(0.5),
                    ..default()
                },
            ));
            root.spawn((
                ImageNode::new(images.radar.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(1.0),
                    top: Val::Percent(4.0),
                    width: Val::VMin(25.778),
                    height: Val::VMin(25.778),
                    ..default()
                },
            ));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                ..default()
            })
            .with_children(|score| {
                score
                    .spawn((
                        Node {
                            width: Val::VMin(6.222),
                            padding: UiRect::all(Val::VMin(0.267)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            border: UiRect::top(Val::Px(2.0)),
                            ..default()
                        },
                        ranking::PracticeCounts,
                        BackgroundColor(PANEL),
                        BorderColor(Color::srgb(0.8, 0.68, 0.35)),
                    ))
                    .with_children(|panel| {
                        panel.spawn((Label::AliveT, text("3", 26.0, Color::WHITE)));
                        panel.spawn(text("Alive", 9.0, Color::WHITE));
                    });
                score
                    .spawn((
                        Node {
                            width: Val::VMin(8.000),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::VMin(0.178)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.02, 0.04, 0.06, 0.65)),
                    ))
                    .with_children(|panel| {
                        panel.spawn((Label::Timer, text("10:00", 17.0, Color::WHITE)));
                    });
                score
                    .spawn((
                        Node {
                            width: Val::VMin(6.222),
                            padding: UiRect::all(Val::VMin(0.267)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            border: UiRect::top(Val::Px(2.0)),
                            ..default()
                        },
                        ranking::PracticeCounts,
                        BackgroundColor(PANEL),
                        BorderColor(Color::srgb(0.45, 0.7, 0.85)),
                    ))
                    .with_children(|panel| {
                        panel.spawn((Label::AliveCt, text("3", 26.0, Color::WHITE)));
                        panel.spawn(text("Alive", 9.0, Color::WHITE));
                    });
            });
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::VMin(12.8),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::VMin(1.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((Label::Mode, text("TDM", 10.0, Color::WHITE)));
                row.spawn((Label::Score, text("0 : 0", 12.0, Color::WHITE)));
            });
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(0.5),
                    bottom: Val::Percent(1.0),
                    padding: UiRect::axes(Val::VMin(0.711), Val::VMin(0.267)),
                    align_items: AlignItems::Center,
                    column_gap: Val::VMin(1.156),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|vitals| {
                vitals.spawn(text("+", 27.0, CYAN));
                vitals.spawn((Label::Health, text("100", 27.0, CYAN)));
                vitals
                    .spawn((
                        Node {
                            width: Val::VMin(6.222),
                            height: Val::VMin(0.747),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.3, 0.3, 0.65)),
                    ))
                    .with_child((
                        Bar::Health,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(CYAN),
                    ));
                vitals.spawn((
                    ImageNode::new(images.shield.clone()),
                    Node {
                        width: Val::VMin(2.133),
                        height: Val::VMin(2.667),
                        ..default()
                    },
                ));
                vitals.spawn((Label::Armor, text("100", 27.0, CYAN)));
                vitals
                    .spawn((
                        Node {
                            width: Val::VMin(6.222),
                            height: Val::VMin(0.747),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.3, 0.3, 0.65)),
                    ))
                    .with_child((
                        Bar::Armor,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(CYAN),
                    ));
            });
            root.spawn((
                AmmoOnly,
                Label::Reload,
                text("", 12.0, CYAN),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(1.0),
                    bottom: Val::Percent(7.0),
                    ..default()
                },
            ));
            root.spawn((
                AmmoOnly,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(1.0),
                    bottom: Val::Percent(6.0),
                    width: Val::VMin(17.778),
                    height: Val::VMin(0.356),
                    ..default()
                },
            ))
            .with_child((
                Bar::Reload,
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(CYAN),
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(0.5),
                    bottom: Val::Percent(1.0),
                    padding: UiRect::axes(Val::VMin(1.067), Val::VMin(0.267)),
                    align_items: AlignItems::Baseline,
                    column_gap: Val::VMin(0.533),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .insert(AmmoOnly)
            .with_children(|ammo| {
                ammo.spawn((Label::Magazine, text("30", 28.0, CYAN)));
                ammo.spawn(text("/", 19.0, CYAN));
                ammo.spawn((Label::Reserve, text("120", 19.0, CYAN)));
                ammo.spawn(Node {
                    margin: UiRect::left(Val::VMin(1.067)),
                    column_gap: Val::VMin(0.231),
                    align_items: AlignItems::End,
                    ..default()
                })
                .with_children(|bullets| {
                    for _ in 0..5 {
                        bullets.spawn((
                            Node {
                                width: Val::VMin(0.338),
                                height: Val::VMin(1.778),
                                ..default()
                            },
                            BackgroundColor(CYAN),
                            BorderRadius::top(Val::VMin(0.178)),
                        ));
                    }
                });
            });
        });
    commands.spawn((
        StatusText,
        text("", 25.0, Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(36.0),
            left: Val::Percent(23.0),
            width: Val::Percent(54.0),
            ..default()
        },
        GlobalZIndex(190),
    ));
}
fn scale_fonts(
    windows: Query<&Window>,
    art: Res<HudArt>,
    mut fonts: Query<(&HudFont, &mut TextFont)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = (window.width() / 1280.0)
        .min(window.height() / 720.0)
        .clamp(0.5, 3.0);
    for (size, mut font) in &mut fonts {
        font.font_size = size.0 * scale;
        if font.font != art.font {
            font.font = art.font.clone();
        }
    }
}
fn update(
    state: Res<State<GameState>>,
    config: Res<GameConfig>,
    session: Res<MatchSession>,
    loading: Res<LoadingStatus>,
    map: Res<LoadedGameplayMapConfig>,
    actors: Query<(&Transform, &Combatant, &WeaponState, Option<&LocalPlayer>)>,
    mut root: Query<&mut Visibility, With<HudRoot>>,
    mut labels: Query<(&Label, &mut Text), Without<StatusText>>,
    mut status: Query<&mut Text, (With<StatusText>, Without<Label>)>,
    mut bars: Query<(&Bar, &mut Node)>,
) {
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if matches!(
            state.get(),
            GameState::Playing | GameState::Paused | GameState::Finished
        ) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let player = actors.iter().find(|(_, _, _, local)| local.is_some());
    let mut alive = [0; 2];
    for (_, actor, _, _) in &actors {
        if actor.alive() {
            alive[actor.team.index()] += 1;
        }
    }
    let remaining = config
        .match_settings
        .time_limit
        .map(|d| (d.as_secs_f32() - session.elapsed).max(0.0) as u32)
        .unwrap_or(0);
    if let Some((transform, actor, weapon, _)) = player {
        let location = map
            .config
            .as_ref()
            .and_then(|map| {
                let point = map
                    .transform
                    .to_transform()
                    .compute_matrix()
                    .inverse()
                    .transform_point3(transform.translation);
                map.callouts
                    .iter()
                    .filter(|c| c.position.to_vec3().xz().distance(point.xz()) <= c.radius)
                    .min_by(|a, b| {
                        a.position
                            .to_vec3()
                            .distance_squared(point)
                            .total_cmp(&b.position.to_vec3().distance_squared(point))
                    })
                    .map(|c| c.name.as_str())
            })
            .unwrap_or(config.map.name());
        for (label, mut text) in &mut labels {
            **text = match label {
                Label::Health => format!("{:.0}", actor.health),
                Label::Armor => format!("{:.0}", actor.armor),
                Label::Magazine => weapon.magazine.to_string(),
                Label::Reserve => weapon.reserve.to_string(),
                Label::Reload => {
                    if weapon.reload_remaining > 0.0 {
                        "RELOADING".into()
                    } else {
                        String::new()
                    }
                }
                Label::Timer => {
                    if config.mode == GameMode::TeamDeathmatch {
                        format!("{:02}:{:02}", remaining / 60, remaining % 60)
                    } else {
                        "--:--".into()
                    }
                }
                Label::Score => format!("{} : {}", session.score[0], session.score[1]),
                Label::AliveT => alive[Team::Attacker.index()].to_string(),
                Label::AliveCt => alive[Team::Defender.index()].to_string(),
                Label::Location => location.into(),
                Label::Mode => {
                    if config.mode == GameMode::TeamDeathmatch {
                        "TDM".into()
                    } else {
                        "PRACTICE".into()
                    }
                }
            };
        }
        for (bar, mut node) in &mut bars {
            node.width = Val::Percent(match bar {
                Bar::Health => actor.health,
                Bar::Armor => actor.armor,
                Bar::Reload => {
                    if weapon.reload_remaining > 0.0 {
                        (1.0 - weapon.reload_remaining / crate::game::weapons::AK47.reload_seconds)
                            * 100.0
                    } else {
                        0.0
                    }
                }
            });
        }
    }
    if let Ok(mut text) = status.single_mut() {
        **text = match state.get() {
            GameState::Loading => format!(
                "LOADING {}\n{}\nEsc - cancel",
                config.map.name(),
                loading.message
            ),
            GameState::LoadFailed => format!(
                "COULD NOT START MATCH\n{}\nEnter - retry    Esc - menu",
                loading.message
            ),
            GameState::Finished => format!(
                "{}\n{} : {}\nEnter - play again    Esc - menu",
                session.result, session.score[0], session.score[1]
            ),
            GameState::Playing => player
                .filter(|(_, a, _, _)| !a.alive())
                .map(|(_, a, _, _)| {
                    format!("ELIMINATED\nRespawn in {:.0}", a.respawn_remaining.max(0.0))
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
    }
}
