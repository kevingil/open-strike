use super::{
    assets::{team_color, HudArt},
    text,
};
use crate::game::{matchplay::KillNotice, player::player::LocalPlayer, GameState};
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Component)]
pub struct FeedRoot;
struct Entry {
    row: Entity,
    remaining: f32,
    local: bool,
}
#[derive(Resource, Default)]
pub struct KillFeed(VecDeque<Entry>);
#[derive(Component)]
pub(super) struct Ink(Color);

pub fn spawn(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        FeedRoot,
        Node {
            position_type: PositionType::Absolute,
            right: Val::VMin(1.4),
            top: Val::VMin(9.0),
            max_width: Val::Percent(32.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: Val::VMin(0.35),
            ..default()
        },
    ));
}
pub fn clear(
    mut commands: Commands,
    mut feed: ResMut<KillFeed>,
    mut kills: EventReader<KillNotice>,
) {
    kills.clear();
    for entry in feed.0.drain(..) {
        commands.entity(entry.row).despawn();
    }
}
fn display_name(name: &str) -> String {
    if name.chars().count() <= 22 {
        name.to_owned()
    } else {
        format!("{}…", name.chars().take(21).collect::<String>())
    }
}
pub fn update(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<GameState>>,
    art: Res<HudArt>,
    root: Query<Entity, With<FeedRoot>>,
    local: Query<Entity, With<LocalPlayer>>,
    mut notices: EventReader<KillNotice>,
    mut feed: ResMut<KillFeed>,
    children: Query<&Children>,
    mut backgrounds: Query<(&mut BackgroundColor, &mut BorderColor)>,
    mut inks: Query<(&Ink, &mut TextColor)>,
    mut images: Query<&mut ImageNode>,
) {
    let Ok(root) = root.single() else {
        return;
    };
    if !matches!(
        state.get(),
        GameState::Playing | GameState::Paused | GameState::Finished
    ) {
        notices.clear();
        return;
    }
    if *state.get() == GameState::Playing {
        for entry in &mut feed.0 {
            entry.remaining -= time.delta_secs();
        }
    }
    while feed.0.front().is_some_and(|e| e.remaining <= 0.0) {
        commands.entity(feed.0.pop_front().unwrap().row).despawn();
    }
    for notice in notices.read() {
        if feed.0.len() == 4 {
            commands.entity(feed.0.pop_front().unwrap().row).despawn();
        }
        let involved = local
            .single()
            .is_ok_and(|id| id == notice.killer_entity || id == notice.victim_entity);
        let row = commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::VMin(0.7), Val::VMin(0.2)),
                    align_items: AlignItems::Center,
                    column_gap: Val::VMin(0.65),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    max_width: Val::VMin(55.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.03, 0.035, 0.66)),
                BorderColor(if involved {
                    Color::srgb(0.85, 0.35, 0.26)
                } else {
                    Color::srgba(0.7, 0.75, 0.8, 0.2)
                }),
                BorderRadius::all(Val::Px(2.0)),
            ))
            .with_children(|row| {
                let color = team_color(notice.team);
                row.spawn((
                    text(&display_name(&notice.killer), 14.0, color),
                    Ink(color),
                    TextLayout::new_with_no_wrap(),
                ));
                row.spawn((
                    ImageNode::new(art.weapon(notice.weapon)),
                    Node {
                        width: Val::VMin(6.8),
                        height: Val::VMin(2.3),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
                if notice.headshot {
                    row.spawn((
                        ImageNode::new(art.headshot.clone()),
                        Node {
                            width: Val::VMin(2.2),
                            height: Val::VMin(2.2),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                }
                let color = team_color(notice.victim_team);
                row.spawn((
                    text(&display_name(&notice.victim), 14.0, color),
                    Ink(color),
                    TextLayout::new_with_no_wrap(),
                ));
            })
            .id();
        commands.entity(root).add_child(row);
        feed.0.push_back(Entry {
            row,
            remaining: 6.0,
            local: involved,
        });
    }
    for entry in &feed.0 {
        let alpha = entry.remaining.clamp(0.0, 1.0);
        if let Ok((mut bg, mut border)) = backgrounds.get_mut(entry.row) {
            bg.0.set_alpha(0.66 * alpha);
            border
                .0
                .set_alpha(if entry.local { alpha } else { 0.2 * alpha });
        }
        for child in children.iter_descendants(entry.row) {
            if let Ok((ink, mut color)) = inks.get_mut(child) {
                color.0 = ink.0.with_alpha(alpha);
            }
            if let Ok(mut image) = images.get_mut(child) {
                image.color.set_alpha(alpha);
            }
        }
    }
}
