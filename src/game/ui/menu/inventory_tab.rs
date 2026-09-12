use super::{style::*, MenuPage, MenuTab, PlayerLoadout, WeaponId};
use crate::game::{
    config::PRESET_GRENADES,
    player::skins::{SkinId, SkinRegistry},
    GameState,
};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

pub struct InventoryTabPlugin;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum Category {
    #[default]
    Everything,
    Weapons,
    Characters,
    Grenades,
}

#[derive(Component)]
struct CategoryButton(Category);
#[derive(Component)]
struct InventoryCard(Category);
#[derive(Component)]
struct ItemCount;
#[derive(Component)]
struct InventoryScroll;
#[derive(Component, Clone, Copy)]
enum Item {
    Weapon(WeaponId),
    Character(SkinId),
}

impl Plugin for InventoryTabPlugin {
    fn build(&self, app: &mut App) {
        // Open this view for a native capture without running the menu scenario.
        if std::env::var_os("CSRS_CAPTURE").is_some()
            && std::env::var_os("CSRS_CAPTURE_INVENTORY").is_some()
        {
            app.add_systems(
                OnEnter(GameState::MainMenu),
                (|mut next: ResMut<NextState<MenuTab>>| next.set(MenuTab::Inventory))
                    .after(super::reset_menu),
            );
        }
        app.init_resource::<Category>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (filter, equipped, scroll)
                    .run_if(in_state(GameState::MainMenu).and(in_state(MenuTab::Inventory))),
            );
    }
}

fn setup(mut commands: Commands, server: Res<AssetServer>, skins: Res<SkinRegistry>) {
    commands
        .spawn((
            MenuPage(MenuTab::Inventory),
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
                    padding: UiRect::all(Val::Px(8.)),
                    column_gap: Val::Px(8.),
                    row_gap: Val::Px(8.),
                    flex_wrap: FlexWrap::Wrap,
                    flex_shrink: 0.,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::bottom(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.10, 0.35)),
                BorderColor(Color::srgba(1., 1., 1., 0.12)),
            ))
            .with_children(|tabs| {
                for (category, title) in [
                    (Category::Everything, "EVERYTHING"),
                    (Category::Weapons, "WEAPONS"),
                    (Category::Characters, "CHARACTERS"),
                    (Category::Grenades, "GRENADES"),
                ] {
                    tabs.spawn((
                        CategoryButton(category),
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(18.), Val::Px(8.)),
                            border: UiRect::bottom(Val::Px(2.)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor(Color::NONE),
                    ))
                    .with_child(label(title, 16., WHITE));
                }
            });
            root.spawn((
                ItemCount,
                label("", 14., MUTED),
                Node {
                    margin: UiRect::new(Val::VMin(5.), Val::VMin(5.), Val::Px(22.), Val::Px(18.)),
                    flex_shrink: 0.,
                    ..default()
                },
            ));
            root.spawn((
                InventoryScroll,
                ScrollPosition::default(),
                Node {
                    flex_grow: 1.,
                    min_height: Val::Px(0.),
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::axes(Val::VMin(5.), Val::Px(0.)),
                    ..default()
                },
            ))
            .with_children(|viewport| {
                viewport
                    .spawn(Node {
                        width: Val::Percent(100.),
                        align_self: AlignSelf::Start,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(22.),
                        row_gap: Val::Px(28.),
                        padding: UiRect::bottom(Val::Px(24.)),
                        ..default()
                    })
                    .with_children(|grid| {
                        let weapons = WeaponId::all().into_iter().map(|weapon| {
                            (
                                Item::Weapon(weapon),
                                Category::Weapons,
                                weapon.name(),
                                weapon.subtitle(),
                                weapon.inventory_path(),
                            )
                        });
                        let characters = skins.skins.iter().map(|skin| {
                            let preview = match skin.id {
                                SkinId::Soldier => "generated/ui/attacker_portrait.png",
                                SkinId::Police => "generated/ui/defender_portrait.png",
                            };
                            (
                                Item::Character(skin.id),
                                Category::Characters,
                                skin.name,
                                skin.side.name(),
                                preview,
                            )
                        });
                        for (item, category, name, subtitle, preview) in weapons.chain(characters) {
                            grid.spawn((
                                InventoryCard(category),
                                Node {
                                    width: Val::Px(200.),
                                    max_width: Val::Percent(100.),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(5.),
                                    ..default()
                                },
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    Node {
                                        width: Val::Percent(100.),
                                        aspect_ratio: Some(4. / 3.),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::bottom(Val::Px(3.)),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.42, 0.45, 0.48, 0.74)),
                                    BorderColor(Color::srgb(0.46, 0.63, 0.71)),
                                ))
                                .with_children(|image| {
                                    let portrait = matches!(item, Item::Character(_));
                                    image.spawn((
                                        ImageNode::new(server.load(preview)),
                                        Node {
                                            width: Val::Percent(if portrait { 68. } else { 100. }),
                                            aspect_ratio: Some(if portrait { 1. } else { 4. / 3. }),
                                            ..default()
                                        },
                                    ));
                                });
                                card.spawn(label(name, 17., WHITE));
                                card.spawn(label(subtitle, 13., MUTED));
                                card.spawn((item, label("", 12., ACCENT)));
                            });
                        }
                        for name in PRESET_GRENADES {
                            grid.spawn((
                                InventoryCard(Category::Grenades),
                                Node {
                                    width: Val::Px(200.),
                                    max_width: Val::Percent(100.),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(5.),
                                    ..default()
                                },
                            ))
                            .with_children(|card| {
                                // Preset information only; no item model or edit action exists yet.
                                card.spawn((
                                    Node {
                                        width: Val::Percent(100.),
                                        aspect_ratio: Some(4. / 3.),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::bottom(Val::Px(3.)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.42, 0.45, 0.48, 0.74)),
                                    BorderColor(Color::srgb(0.46, 0.63, 0.71)),
                                ))
                                .with_child(label("PRESET", 20., MUTED));
                                card.spawn(label(name, 17., WHITE));
                                card.spawn(label("Grenade · Fixed selection", 13., MUTED));
                                card.spawn(label("Not customizable", 12., ACCENT));
                            });
                        }
                    });
            });
            root.spawn((
                label("Choose weapons available to buy in Load Out", 13., MUTED),
                Node {
                    margin: UiRect::axes(Val::VMin(5.), Val::Px(18.)),
                    flex_shrink: 0.,
                    ..default()
                },
            ));
        });
}

fn filter(
    mut selected: ResMut<Category>,
    mut buttons: Query<(
        &CategoryButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut cards: Query<(&InventoryCard, &mut Node)>,
    mut counts: Query<&mut Text, With<ItemCount>>,
    mut scrolls: Query<&mut ScrollPosition, With<InventoryScroll>>,
) {
    for (button, interaction, _, _) in &mut buttons {
        if *interaction == Interaction::Pressed && *selected != button.0 {
            *selected = button.0;
            for mut scroll in &mut scrolls {
                scroll.offset_y = 0.;
            }
        }
    }
    for (button, interaction, mut background, mut border) in &mut buttons {
        let active = *selected == button.0;
        background.0 = if active {
            GLASS_SELECTED
        } else if *interaction == Interaction::Hovered {
            Color::srgba(1., 1., 1., 0.08)
        } else {
            Color::NONE
        };
        border.0 = if active { ACCENT } else { Color::NONE };
    }
    let mut count = 0;
    for (card, mut node) in &mut cards {
        let visible = *selected == Category::Everything || *selected == card.0;
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        count += usize::from(visible);
    }
    for mut text in &mut counts {
        let value = format!("{count} items");
        if text.0 != value {
            **text = value;
        }
    }
}

fn equipped(loadout: Res<PlayerLoadout>, mut labels: Query<(&Item, &mut Text)>) {
    for (item, mut text) in &mut labels {
        let active = match item {
            Item::Weapon(weapon) => {
                *weapon == loadout.primary_weapon || *weapon == loadout.melee_weapon
            }
            Item::Character(skin) => *skin == loadout.selected_skin,
        };
        let value = if active { "Equipped" } else { "Available" };
        if text.0 != value {
            **text = value.into();
        }
    }
}

fn scroll(
    mut events: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut views: Query<(&ComputedNode, &GlobalTransform, &mut ScrollPosition), With<InventoryScroll>>,
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
            let center = transform.translation().truncate();
            if !Rect::from_center_size(center, node.size()).contains(cursor) {
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
