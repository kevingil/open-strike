use super::{
    assets::{team_color, HudArt},
    text,
};
use crate::game::{
    config::{GameConfig, GameMode},
    matchplay::Combatant,
    player::player::LocalPlayer,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct RankingRoot;
#[derive(Component)]
pub struct PracticeCounts;
#[derive(Component)]
pub(super) struct Card {
    actor: Entity,
    portrait: Entity,
    name: Entity,
    score: Entity,
}

pub fn spawn(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        RankingRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::VMin(3.5),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::VMin(0.4),
            ..default()
        },
    ));
}
pub fn update(
    mut commands: Commands,
    config: Res<GameConfig>,
    art: Res<HudArt>,
    root: Query<Entity, With<RankingRoot>>,
    actors: Query<(Entity, &Combatant, Option<&LocalPlayer>)>,
    cards: Query<(Entity, &Card)>,
    mut texts: Query<&mut Text>,
    mut images: Query<&mut ImageNode>,
    mut borders: Query<&mut BorderColor>,
    mut nodes: Query<&mut Node>,
    children: Query<&Children>,
    practice: Query<Entity, With<PracticeCounts>>,
) {
    let tdm = config.mode == GameMode::TeamDeathmatch;
    for entity in &practice {
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if tdm { Display::None } else { Display::Flex };
        }
    }
    let Ok(root) = root.single() else {
        return;
    };
    if let Ok(mut node) = nodes.get_mut(root) {
        node.display = if tdm { Display::Flex } else { Display::None };
    }
    let mut ranked: Vec<_> = actors.iter().collect();
    ranked.sort_by(|(_, a, _), (_, b, _)| b.kills.cmp(&a.kills).then_with(|| a.slot.cmp(&b.slot)));
    for (entity, card) in &cards {
        if !actors.contains(card.actor) {
            commands.entity(entity).despawn();
        }
    }
    let mut order = Vec::new();
    for (actor_id, actor, local) in ranked {
        let label = if local.is_some() { "YOU" } else { &actor.name };
        if let Some((entity, card)) = cards.iter().find(|(_, c)| c.actor == actor_id) {
            order.push(entity);
            if let Ok(mut image) = images.get_mut(card.portrait) {
                image.color = Color::WHITE.with_alpha(if actor.alive() { 1.0 } else { 0.25 });
            }
            if let Ok(mut value) = texts.get_mut(card.score) {
                let next = actor.kills.to_string();
                if **value != next {
                    **value = next;
                }
            }
            if let Ok(mut value) = texts.get_mut(card.name) {
                if value.0 != label {
                    value.0 = label.to_owned();
                }
            }
            if let Ok(mut border) = borders.get_mut(entity) {
                border.0 = if local.is_some() {
                    Color::WHITE
                } else {
                    team_color(actor.team)
                };
            }
        } else {
            let portrait = commands
                .spawn((
                    ImageNode::new(art.portraits[actor.team.index()].clone()),
                    Node {
                        width: Val::VMin(6.0),
                        height: Val::VMin(5.4),
                        ..default()
                    },
                ))
                .id();
            let name = commands
                .spawn((
                    text(label, 8.0, team_color(actor.team)),
                    TextLayout::new_with_no_wrap(),
                    Node {
                        max_width: Val::VMin(6.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .id();
            let score = commands
                .spawn(text(&actor.kills.to_string(), 15.0, Color::WHITE))
                .id();
            let entity = commands
                .spawn((
                    Card {
                        actor: actor_id,
                        portrait,
                        name,
                        score,
                    },
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if local.is_some() { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.06, 0.07, 0.52)),
                    BorderColor(if local.is_some() {
                        Color::WHITE
                    } else {
                        team_color(actor.team)
                    }),
                ))
                .add_children(&[portrait, name, score])
                .id();
            order.push(entity);
        }
    }
    // Reorder only on a score/roster change, preserving keyed card entities.
    let unchanged = children
        .get(root)
        .is_ok_and(|current| current.iter().eq(order.iter().copied()));
    if !unchanged {
        commands.entity(root).replace_children(&order);
    }
}
