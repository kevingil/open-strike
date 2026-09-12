use super::{assets::HudArt, text};
use crate::game::{config::WeaponId, player::player::LocalPlayer, weapons::WeaponState};
use bevy::prelude::*;
#[derive(Component)]
pub struct AmmoOnly;
#[derive(Component)]
pub(super) struct Slot(WeaponId);
#[derive(Component)]
pub(super) struct SelectedName;

pub fn spawn(parent: &mut ChildSpawnerCommands, art: &HudArt) {
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::VMin(1.4),
            bottom: Val::VMin(8.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: Val::VMin(1.1),
            ..default()
        })
        .with_children(|stack| {
            for (id, number) in [(WeaponId::AK47, "1"), (WeaponId::DefaultKnife, "3")] {
                stack
                    .spawn(Node {
                        align_items: AlignItems::Start,
                        column_gap: Val::VMin(0.7),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Slot(id),
                            ImageNode::new(art.weapon(id)),
                            Node {
                                width: Val::VMin(13.0),
                                height: Val::VMin(4.3),
                                ..default()
                            },
                        ));
                        row.spawn((Slot(id), text(number, 11.0, Color::WHITE)));
                    });
            }
            stack.spawn((SelectedName, text("AK-47", 12.0, Color::WHITE)));
        });
}
pub fn update(
    player: Query<&WeaponState, With<LocalPlayer>>,
    art: Res<HudArt>,
    mut slots: Query<(&Slot, Option<&mut ImageNode>, Option<&mut TextColor>)>,
    mut names: Query<&mut Text, With<SelectedName>>,
    mut ammo: Query<&mut Visibility, With<AmmoOnly>>,
) {
    let Ok(weapon) = player.single() else {
        return;
    };
    for (slot, image, color) in &mut slots {
        let id = if slot.0.is_knife() {
            weapon.melee_weapon
        } else {
            weapon.gun
        };
        let tint = Color::WHITE.with_alpha(if id == weapon.active { 1.0 } else { 0.38 });
        if let Some(mut image) = image {
            image.color = tint;
            image.image = art.weapon(id);
        }
        if let Some(mut color) = color {
            color.0 = tint;
        }
    }
    for mut name in &mut names {
        if name.0 != weapon.active.name() {
            name.0 = weapon.active.name().to_owned();
        }
    }
    for mut visible in &mut ammo {
        *visible = if weapon.active.is_knife() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
