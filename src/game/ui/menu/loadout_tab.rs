//! Choose each side's buy-menu weapons; this does not equip the player's held weapon.
use super::{
    loadout_scene::{self, CharacterPreviews},
    style::*,
    MenuPage, MenuTab, PlayerLoadout, WeaponId,
};
use crate::game::{
    config::{BuyCategory, BUY_SLOT_COUNT, PRESET_ARMOR, PRESET_GRENADES},
    player::skins::PlayerSide,
    GameState,
};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

pub struct LoadoutTabPlugin;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Slot {
    category: BuyCategory,
    index: usize,
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            category: BuyCategory::Rifles,
            index: 0,
        }
    }
}
#[derive(Resource)]
struct LoadoutEditor {
    side: PlayerSide,
    attacker: Slot,
    defender: Slot,
}
impl Default for LoadoutEditor {
    fn default() -> Self {
        Self {
            side: PlayerSide::Attacker,
            attacker: Slot::default(),
            defender: Slot::default(),
        }
    }
}
impl LoadoutEditor {
    fn slot(&self) -> Slot {
        if self.side == PlayerSide::Defender {
            self.defender
        } else {
            self.attacker
        }
    }
    fn select(&mut self, slot: Slot) {
        if self.side == PlayerSide::Defender {
            self.defender = slot;
        } else {
            self.attacker = slot;
        }
    }
}

#[derive(Component, Clone, Copy)]
pub(super) enum LoadoutButton {
    Side(PlayerSide),
    Slot(Slot),
    Category(BuyCategory),
    Weapon(WeaponId),
    Melee(WeaponId),
    Clear,
}
#[derive(Component)]
pub(super) struct CharacterViewport(pub PlayerSide);
#[derive(Component)]
struct SlotLabel(Slot);
#[derive(Component)]
struct SlotStatus(Slot);
#[derive(Component)]
struct WeaponStatus(WeaponId);
#[derive(Component)]
struct LoadoutHeading;
#[derive(Component)]
struct InventoryHeading;
#[derive(Component)]
struct EmptyInventory;
#[derive(Component)]
struct LoadoutScroll;

impl Plugin for LoadoutTabPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("CSRS_CAPTURE").is_some()
            && std::env::var_os("CSRS_CAPTURE_LOADOUT").is_some()
        {
            app.add_systems(
                OnEnter(GameState::MainMenu),
                (|mut next: ResMut<NextState<MenuTab>>| next.set(MenuTab::LoadOut))
                    .after(super::reset_menu),
            );
        }
        loadout_scene::install(app);
        app.init_resource::<LoadoutEditor>()
            .add_systems(Startup, (loadout_scene::setup_targets, setup).chain())
            .add_systems(
                Update,
                (interact, refresh, scroll)
                    .chain()
                    .run_if(in_state(GameState::MainMenu).and(in_state(MenuTab::LoadOut))),
            );
    }
}

fn setup(mut commands: Commands, previews: Res<CharacterPreviews>, server: Res<AssetServer>) {
    commands
        .spawn((
            MenuPage(MenuTab::LoadOut),
            LoadoutScroll,
            ScrollPosition::default(),
            Node {
                padding: UiRect::ZERO,
                row_gap: Val::Px(0.),
                overflow: Overflow::scroll_y(),
                ..page()
            },
            BackgroundColor(Color::srgba(0.16, 0.19, 0.22, 0.68)),
            GlobalZIndex(150),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: Val::Percent(100.),
                height: Val::Vh(57.),
                min_height: Val::Px(410.),
                flex_shrink: 0.,
                padding: UiRect::all(Val::VMin(2.)),
                column_gap: Val::VMin(1.5),
                ..default()
            })
            .with_children(|loadout| {
                spawn_side(loadout, &previews, PlayerSide::Defender);
                loadout
                    .spawn(Node {
                        flex_grow: 1.,
                        flex_basis: Val::Px(0.),
                        min_width: Val::Px(460.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.),
                        ..default()
                    })
                    .with_children(|center| {
                        center.spawn((LoadoutHeading, label("", 18., WHITE)));
                        center.spawn(label("Choose the weapons available to buy.", 13., MUTED));
                        center
                            .spawn(Node {
                                flex_grow: 1.,
                                min_height: Val::Px(0.),
                                column_gap: Val::Px(8.),
                                ..default()
                            })
                            .with_children(|columns| {
                                columns.spawn(column()).with_children(|equipment| {
                                    equipment.spawn(label("EQUIPMENT", 13., WHITE));
                                    equipment.spawn(label("Armor · Fixed", 11., MUTED));
                                    for name in PRESET_ARMOR {
                                        spawn_preset(equipment, name);
                                    }
                                    equipment.spawn(label("Knife · Both teams", 11., MUTED));
                                    for knife in [WeaponId::DefaultKnife, WeaponId::ReferenceKnife]
                                    {
                                        equipment
                                            .spawn((
                                                LoadoutButton::Melee(knife),
                                                Button,
                                                cell(),
                                                BackgroundColor(Color::NONE),
                                                BorderColor(Color::NONE),
                                            ))
                                            .with_child(label(knife.name(), 12., WHITE));
                                    }
                                });
                                for category in BuyCategory::ALL {
                                    columns.spawn(column()).with_children(|weapons| {
                                        weapons.spawn(label(
                                            category.name().to_uppercase(),
                                            13.,
                                            WHITE,
                                        ));
                                        weapons.spawn(label(category.description(), 11., MUTED));
                                        for index in 0..BUY_SLOT_COUNT {
                                            let slot = Slot { category, index };
                                            weapons
                                                .spawn((
                                                    LoadoutButton::Slot(slot),
                                                    Button,
                                                    cell(),
                                                    BackgroundColor(Color::NONE),
                                                    BorderColor(Color::NONE),
                                                ))
                                                .with_children(|entry| {
                                                    entry.spawn((
                                                        SlotLabel(slot),
                                                        label("", 14., WHITE),
                                                    ));
                                                    entry.spawn((
                                                        SlotStatus(slot),
                                                        label("", 10., MUTED),
                                                    ));
                                                });
                                        }
                                    });
                                }
                                columns.spawn(column()).with_children(|grenades| {
                                    grenades.spawn(label("GRENADES", 13., WHITE));
                                    grenades.spawn(label("Preset · Fixed", 11., MUTED));
                                    for name in PRESET_GRENADES {
                                        spawn_preset(grenades, name);
                                    }
                                });
                            });
                    });
                spawn_side(loadout, &previews, PlayerSide::Attacker);
            });
            root.spawn((
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(230.),
                    flex_grow: 1.,
                    flex_shrink: 0.,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::VMin(3.), Val::Px(16.)),
                    row_gap: Val::Px(12.),
                    border: UiRect::top(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.10, 0.25)),
                BorderColor(Color::srgba(1., 1., 1., 0.14)),
            ))
            .with_children(|inventory| {
                inventory.spawn((InventoryHeading, label("", 18., WHITE)));
                inventory
                    .spawn(Node {
                        column_gap: Val::Px(8.),
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(8.),
                        ..default()
                    })
                    .with_children(|filters| {
                        for category in BuyCategory::ALL {
                            filters
                                .spawn((
                                    LoadoutButton::Category(category),
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(14.), Val::Px(7.)),
                                        border: UiRect::bottom(Val::Px(2.)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    BorderColor(Color::NONE),
                                ))
                                .with_child(label(category.name(), 14., WHITE));
                        }
                        filters
                            .spawn((
                                LoadoutButton::Clear,
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(14.), Val::Px(7.)),
                                    border: UiRect::bottom(Val::Px(2.)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                BorderColor(Color::NONE),
                            ))
                            .with_child(label("Remove from slot", 14., WHITE));
                    });
                inventory
                    .spawn(Node {
                        column_gap: Val::Px(16.),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|items| {
                        // The current inventory has one buy-menu weapon. The knife is not a purchase selection.
                        items
                            .spawn((
                                LoadoutButton::Weapon(WeaponId::AK47),
                                Button,
                                Node {
                                    width: Val::Px(200.),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(8.)),
                                    row_gap: Val::Px(5.),
                                    border: UiRect::bottom(Val::Px(2.)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                BorderColor(Color::NONE),
                            ))
                            .with_children(|weapon| {
                                weapon.spawn((
                                    ImageNode::new(server.load("generated/ui/inventory/ak47.png")),
                                    Node {
                                        width: Val::Px(128.),
                                        height: Val::Px(76.),
                                        align_self: AlignSelf::Center,
                                        ..default()
                                    },
                                ));
                                weapon.spawn(label("AK-47", 16., WHITE));
                                weapon.spawn((WeaponStatus(WeaponId::AK47), label("", 12., MUTED)));
                            });
                        items.spawn((
                            EmptyInventory,
                            label("No weapons available in this category yet.", 18., MUTED),
                            Node {
                                margin: UiRect::vertical(Val::Px(28.)),
                                ..default()
                            },
                        ));
                    });
            });
        });
}

fn column() -> Node {
    Node {
        flex_grow: 1.,
        flex_basis: Val::Px(0.),
        min_width: Val::Px(0.),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.),
        ..default()
    }
}
fn cell() -> Node {
    Node {
        height: Val::Percent(16.),
        min_height: Val::Px(42.),
        flex_shrink: 0.,
        padding: UiRect::all(Val::Px(5.)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(4.),
        border: UiRect::bottom(Val::Px(1.)),
        ..default()
    }
}
fn spawn_preset(parent: &mut ChildSpawnerCommands, name: &str) {
    // No Button or interaction component: armor and grenades cannot be edited.
    parent
        .spawn((
            cell(),
            BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.12)),
            BorderColor(Color::srgba(1., 1., 1., 0.14)),
        ))
        .with_child(label(name, 12., MUTED));
}
fn spawn_side(parent: &mut ChildSpawnerCommands, previews: &CharacterPreviews, side: PlayerSide) {
    parent
        .spawn((
            LoadoutButton::Side(side),
            Button,
            Node {
                width: Val::Percent(18.),
                min_width: Val::Px(130.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Px(12.)),
                row_gap: Val::Px(12.),
                border: UiRect::top(Val::Px(2.)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor(Color::NONE),
        ))
        .with_children(|character| {
            character.spawn(label(side.name().to_uppercase(), 21., WHITE));
            character.spawn((
                CharacterViewport(side),
                ImageNode::new(previews.target(side)),
                Node {
                    width: Val::Percent(100.),
                    flex_grow: 1.,
                    min_height: Val::Px(0.),
                    flex_basis: Val::Px(0.),
                    ..default()
                },
            ));
        });
}
fn interact(
    mut editor: ResMut<LoadoutEditor>,
    mut loadout: ResMut<PlayerLoadout>,
    buttons: Query<(&LoadoutButton, &Interaction), Changed<Interaction>>,
) {
    for (button, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *button {
            LoadoutButton::Side(side) => editor.side = side,
            LoadoutButton::Slot(slot) => editor.select(slot),
            LoadoutButton::Category(category) => editor.select(Slot { category, index: 0 }),
            LoadoutButton::Weapon(weapon) => {
                let slot = editor.slot();
                loadout.buy_weapons.side_mut(editor.side).set(
                    slot.category,
                    slot.index,
                    Some(weapon),
                );
            }
            LoadoutButton::Melee(knife) => loadout.melee_weapon = knife,
            LoadoutButton::Clear => {
                let slot = editor.slot();
                loadout
                    .buy_weapons
                    .side_mut(editor.side)
                    .set(slot.category, slot.index, None);
            }
        }
    }
}
fn refresh(
    editor: Res<LoadoutEditor>,
    loadout: Res<PlayerLoadout>,
    mut buttons: Query<(
        &LoadoutButton,
        &Interaction,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut empty: Query<&mut Node, (With<EmptyInventory>, Without<LoadoutButton>)>,
    mut labels: Query<
        (
            &mut Text,
            Option<&SlotLabel>,
            Option<&SlotStatus>,
            Option<&WeaponStatus>,
            Has<LoadoutHeading>,
            Has<InventoryHeading>,
        ),
        Or<(
            With<SlotLabel>,
            With<SlotStatus>,
            With<WeaponStatus>,
            With<LoadoutHeading>,
            With<InventoryHeading>,
        )>,
    >,
) {
    let selected = editor.slot();
    let side = loadout.buy_weapons.side(editor.side);
    let accent = editor.side.color();
    for (button, interaction, mut node, mut background, mut border) in &mut buttons {
        let visible = match *button {
            LoadoutButton::Weapon(weapon) => selected.category.accepts(weapon),
            LoadoutButton::Clear => side.slots(selected.category)[selected.index].is_some(),
            _ => true,
        };
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        let active = match *button {
            LoadoutButton::Side(value) => editor.side == value,
            LoadoutButton::Slot(slot) => selected == slot,
            LoadoutButton::Category(category) => selected.category == category,
            LoadoutButton::Weapon(weapon) => {
                side.slots(selected.category)[selected.index] == Some(weapon)
            }
            LoadoutButton::Melee(knife) => loadout.melee_weapon == knife,
            LoadoutButton::Clear => false,
        };
        background.0 = if active {
            accent.with_alpha(0.12)
        } else if *interaction == Interaction::Hovered {
            Color::srgba(1., 1., 1., 0.10)
        } else {
            Color::srgba(0.04, 0.06, 0.08, 0.12)
        };
        border.0 = if active {
            accent
        } else {
            Color::srgba(1., 1., 1., 0.18)
        };
    }
    for mut node in &mut empty {
        node.display = if WeaponId::all()
            .into_iter()
            .any(|weapon| selected.category.accepts(weapon))
        {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (mut text, slot_label, slot_status, weapon_status, heading, _) in &mut labels {
        let value = if let Some(SlotLabel(slot)) = slot_label {
            side.slots(slot.category)[slot.index]
                .map_or("Empty", |weapon| weapon.name())
                .into()
        } else if let Some(SlotStatus(slot)) = slot_status {
            if side.slots(slot.category)[slot.index].is_some() {
                "Available to buy"
            } else {
                "Select weapon"
            }
            .into()
        } else if let Some(WeaponStatus(weapon)) = weapon_status {
            if side.slots(selected.category)[selected.index] == Some(*weapon) {
                "Available to buy"
            } else if side.slots(selected.category).contains(&Some(*weapon)) {
                "Move to selected slot"
            } else {
                "Add to selected slot"
            }
            .into()
        } else if heading {
            format!("{} / LOAD OUT", editor.side.name().to_uppercase())
        } else {
            format!(
                "INVENTORY / {} / {}",
                editor.side.name().to_uppercase(),
                selected.category.name().to_uppercase()
            )
        };
        if text.0 != value {
            **text = value;
        }
    }
}
fn scroll(
    mut events: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut views: Query<(&ComputedNode, &GlobalTransform, &mut ScrollPosition), With<LoadoutScroll>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        events.clear();
        return;
    };
    for event in events.read() {
        for (node, transform, mut scroll) in &mut views {
            if !Rect::from_center_size(transform.translation().truncate(), node.size())
                .contains(cursor)
            {
                continue;
            }
            let delta = event.y
                * if event.unit == MouseScrollUnit::Line {
                    32.
                } else {
                    1.
                };
            let maximum =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.);
            scroll.offset_y = (scroll.offset_y - delta).clamp(0., maximum);
        }
    }
}
