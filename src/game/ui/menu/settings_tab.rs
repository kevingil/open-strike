use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::RelativeCursorPosition,
};

use super::{style::*, MenuPage, MenuTab, PlayerSettings};
use crate::game::{config::BotDifficulty, GameState};

pub struct SettingsTabPlugin;

impl Plugin for SettingsTabPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Category>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (select_category, interact, refresh, scroll)
                    .chain()
                    .run_if(in_state(GameState::MainMenu).and(in_state(MenuTab::Settings))),
            );
        if std::env::var_os("CSRS_CAPTURE").is_some()
            && std::env::var_os("CSRS_CAPTURE_SETTINGS").is_some()
        {
            app.add_systems(
                OnEnter(GameState::MainMenu),
                (|mut next: ResMut<NextState<MenuTab>>| next.set(MenuTab::Settings))
                    .after(super::reset_menu),
            );
        }
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum Category {
    #[default]
    All,
    Video,
    Audio,
    Mouse,
    Gameplay,
}

impl Category {
    fn title(self) -> &'static str {
        match self {
            Self::All => "ALL SETTINGS",
            Self::Video => "VIDEO",
            Self::Audio => "AUDIO",
            Self::Mouse => "KEYBOARD / MOUSE",
            Self::Gameplay => "GAMEPLAY",
        }
    }
}

#[derive(Clone, Copy)]
enum Setting {
    Fov,
    Volume,
    Sensitivity,
}

impl Setting {
    fn title(self) -> &'static str {
        match self {
            Self::Fov => "Field of View",
            Self::Volume => "Master Volume",
            Self::Sensitivity => "Mouse Sensitivity",
        }
    }

    fn category(self) -> Category {
        match self {
            Self::Fov => Category::Video,
            Self::Volume => Category::Audio,
            Self::Sensitivity => Category::Mouse,
        }
    }

    fn normalized(self, settings: &PlayerSettings) -> f32 {
        match self {
            Self::Fov => (settings.fov - 60.) / 60.,
            Self::Volume => settings.master_volume,
            Self::Sensitivity => (settings.sensitivity - 0.1) / 2.9,
        }
        .clamp(0., 1.)
    }

    fn value(self, settings: &PlayerSettings) -> String {
        match self {
            Self::Fov => format!("{:.0}°", settings.fov),
            Self::Volume => format!("{:.0}%", settings.master_volume * 100.),
            Self::Sensitivity => format!("{:.2}", settings.sensitivity),
        }
    }

    fn set(self, settings: &mut PlayerSettings, position: f32) {
        let position = position.clamp(0., 1.);
        match self {
            Self::Fov => settings.fov = (60. + position * 60.).round(),
            Self::Volume => settings.master_volume = (position * 100.).round() / 100.,
            Self::Sensitivity => {
                settings.sensitivity = ((0.1 + position * 2.9) * 100.).round() / 100.
            }
        }
    }
}

#[derive(Component)]
struct CategoryButton(Category);
#[derive(Component)]
struct Section(Category);
#[derive(Component)]
struct Slider(Setting);
#[derive(Component)]
struct SliderFill(Setting);
#[derive(Component)]
struct SliderThumb(Setting);
#[derive(Component)]
struct SettingValue(Setting);
#[derive(Component)]
struct DifficultyButton(BotDifficulty);
#[derive(Component)]
struct SettingsScroll;
#[derive(Component)]
struct ResetButton;

const LINE: Color = Color::srgba(1., 1., 1., 0.14);
const TRACK: Color = Color::srgba(1., 1., 1., 0.18);
const FILL: Color = Color::srgb(0.73, 0.76, 0.76);

fn setup(mut commands: Commands, settings: Res<PlayerSettings>) {
    commands
        .spawn((
            MenuPage(MenuTab::Settings),
            Node {
                padding: UiRect::ZERO,
                row_gap: Val::Px(0.),
                ..page()
            },
            BackgroundColor(Color::srgba(0.12, 0.14, 0.15, 0.72)),
            GlobalZIndex(150),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(58.),
                    flex_shrink: 0.,
                    padding: UiRect::all(Val::Px(8.)),
                    column_gap: Val::Px(8.),
                    row_gap: Val::Px(4.),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::bottom(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.38)),
                BorderColor(LINE),
            ))
            .with_children(|tabs| {
                for category in [
                    Category::All,
                    Category::Video,
                    Category::Audio,
                    Category::Mouse,
                    Category::Gameplay,
                ] {
                    tabs.spawn((
                        CategoryButton(category),
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(16.), Val::Px(8.)),
                            border: UiRect::bottom(Val::Px(2.)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor(Color::NONE),
                    ))
                    .with_child(label(category.title(), 16., WHITE));
                }
            });
            root.spawn((
                SettingsScroll,
                ScrollPosition::default(),
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(0.),
                    flex_grow: 1.,
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::axes(Val::VMin(5.), Val::Px(28.)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .with_children(|viewport| {
                viewport
                    .spawn(Node {
                        width: Val::Percent(100.),
                        max_width: Val::Px(900.),
                        height: Val::Auto,
                        align_self: AlignSelf::Start,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .with_children(|content| {
                        spawn_section(content, Category::Video, |section| {
                            spawn_slider(section, Setting::Fov, &settings);
                        });
                        spawn_section(content, Category::Audio, |section| {
                            spawn_slider(section, Setting::Volume, &settings);
                        });
                        spawn_section(content, Category::Mouse, |section| {
                            spawn_slider(section, Setting::Sensitivity, &settings);
                        });
                        spawn_section(content, Category::Gameplay, |section| {
                            spawn_difficulty(section, &settings);
                        });
                    });
            });
            root.spawn((
                Node {
                    width: Val::Percent(100.),
                    min_height: Val::Px(66.),
                    flex_shrink: 0.,
                    padding: UiRect::axes(Val::VMin(5.), Val::Px(12.)),
                    column_gap: Val::Px(16.),
                    row_gap: Val::Px(8.),
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::top(Val::Px(1.)),
                    ..default()
                },
                BorderColor(LINE),
                BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.25)),
            ))
            .with_children(|footer| {
                footer.spawn(label(
                    "Saved on this device with your account session and loadout",
                    14.,
                    MUTED,
                ));
                footer
                    .spawn((
                        ResetButton,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(16.), Val::Px(8.)),
                            border: UiRect::all(Val::Px(1.)),
                            ..default()
                        },
                        BorderColor(LINE),
                        BackgroundColor(Color::NONE),
                    ))
                    .with_child(label("RESET TO DEFAULT", 14., WHITE));
            });
        });
}

fn spawn_section(
    parent: &mut ChildSpawnerCommands,
    category: Category,
    spawn_rows: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Section(category),
            Node {
                width: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(Val::Px(30.)),
                ..default()
            },
        ))
        .with_children(|section| {
            section.spawn((
                label(category.title(), 15., MUTED),
                Node {
                    margin: UiRect::bottom(Val::Px(12.)),
                    ..default()
                },
            ));
            spawn_rows(section);
        });
}

fn spawn_slider(parent: &mut ChildSpawnerCommands, setting: Setting, settings: &PlayerSettings) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.),
                min_height: Val::Px(66.),
                padding: UiRect::axes(Val::Px(0.), Val::Px(12.)),
                border: UiRect::bottom(Val::Px(1.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(24.),
                row_gap: Val::Px(8.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BorderColor(LINE),
        ))
        .with_children(|row| {
            row.spawn(label(setting.title(), 19., WHITE));
            row.spawn(Node {
                width: Val::Percent(42.),
                min_width: Val::Px(240.),
                max_width: Val::Percent(100.),
                align_items: AlignItems::Center,
                column_gap: Val::Px(18.),
                ..default()
            })
            .with_children(|control| {
                control
                    .spawn((
                        Slider(setting),
                        Button,
                        RelativeCursorPosition::default(),
                        Node {
                            flex_grow: 1.,
                            height: Val::Px(32.),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_children(|slider| {
                        slider
                            .spawn((
                                Node {
                                    width: Val::Percent(100.),
                                    height: Val::Px(4.),
                                    ..default()
                                },
                                BackgroundColor(TRACK),
                            ))
                            .with_child((
                                SliderFill(setting),
                                Node {
                                    width: Val::Percent(setting.normalized(settings) * 100.),
                                    height: Val::Percent(100.),
                                    ..default()
                                },
                                BackgroundColor(FILL),
                            ));
                        slider.spawn((
                            SliderThumb(setting),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(setting.normalized(settings) * 100.),
                                margin: UiRect::left(Val::Px(-4.)),
                                width: Val::Px(8.),
                                height: Val::Px(14.),
                                ..default()
                            },
                            BackgroundColor(WHITE),
                        ));
                    });
                control
                    .spawn((
                        Node {
                            width: Val::Px(70.),
                            height: Val::Px(32.),
                            flex_shrink: 0.,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(Val::Px(1.)),
                            ..default()
                        },
                        BorderColor(LINE),
                        BackgroundColor(Color::srgba(1., 1., 1., 0.035)),
                    ))
                    .with_child((
                        SettingValue(setting),
                        label(setting.value(settings), 18., WHITE),
                    ));
            });
        });
}

fn spawn_difficulty(parent: &mut ChildSpawnerCommands, settings: &PlayerSettings) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.),
                min_height: Val::Px(66.),
                padding: UiRect::axes(Val::Px(0.), Val::Px(12.)),
                border: UiRect::bottom(Val::Px(1.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(24.),
                row_gap: Val::Px(8.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            BorderColor(LINE),
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.),
                ..default()
            })
            .with_children(|copy| {
                copy.spawn(label("Bot Difficulty", 19., WHITE));
                copy.spawn(label(
                    "How quickly local bots aim and how long they hold fire",
                    14.,
                    MUTED,
                ));
            });
            row.spawn(Node {
                column_gap: Val::Px(8.),
                ..default()
            })
            .with_children(|choices| {
                for difficulty in BotDifficulty::ALL {
                    let selected = settings.bot_difficulty == difficulty;
                    choices
                        .spawn((
                            DifficultyButton(difficulty),
                            Button,
                            Node {
                                min_width: Val::Px(92.),
                                padding: UiRect::axes(Val::Px(16.), Val::Px(8.)),
                                border: UiRect::all(Val::Px(1.)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(if selected {
                                GLASS_SELECTED
                            } else {
                                Color::NONE
                            }),
                            BorderColor(if selected { ACCENT } else { LINE }),
                        ))
                        .with_child(label(difficulty.name().to_uppercase(), 14., WHITE));
                }
            });
        });
}

fn select_category(
    mut category: ResMut<Category>,
    mut buttons: Query<(
        &CategoryButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut sections: Query<(&Section, &mut Node)>,
    mut viewport: Query<&mut ScrollPosition, With<SettingsScroll>>,
) {
    for (button, interaction, _, _) in &buttons {
        if *interaction == Interaction::Pressed && *category != button.0 {
            *category = button.0;
            for mut position in &mut viewport {
                position.offset_y = 0.;
            }
        }
    }
    for (button, interaction, mut background, mut border) in &mut buttons {
        let selected = button.0 == *category;
        background.0 = if selected {
            GLASS_SELECTED
        } else if *interaction == Interaction::Hovered {
            Color::srgba(1., 1., 1., 0.06)
        } else {
            Color::NONE
        };
        border.0 = if selected { ACCENT } else { Color::NONE };
    }
    for (section, mut node) in &mut sections {
        node.display = if *category == Category::All || section.0 == *category {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn interact(
    category: Res<Category>,
    sliders: Query<(&Slider, &Interaction, &RelativeCursorPosition)>,
    difficulties: Query<(&DifficultyButton, &Interaction)>,
    mut resets: Query<(&Interaction, &mut BackgroundColor), With<ResetButton>>,
    mut settings: ResMut<PlayerSettings>,
) {
    for (slider, interaction, cursor) in &sliders {
        if *interaction == Interaction::Pressed
            && (*category == Category::All || slider.0.category() == *category)
        {
            if let Some(position) = cursor.normalized {
                slider.0.set(&mut settings, position.x);
            }
        }
    }
    if *category == Category::All || *category == Category::Gameplay {
        for (button, interaction) in &difficulties {
            if *interaction == Interaction::Pressed {
                settings.bot_difficulty = button.0;
            }
        }
    }
    for (interaction, mut background) in &mut resets {
        background.0 = if *interaction == Interaction::None {
            Color::NONE
        } else {
            GLASS_SELECTED
        };
        if *interaction == Interaction::Pressed {
            let defaults = PlayerSettings::default();
            for setting in [Setting::Fov, Setting::Volume, Setting::Sensitivity] {
                if *category == Category::All || setting.category() == *category {
                    setting.set(&mut settings, setting.normalized(&defaults));
                }
            }
            if *category == Category::All || *category == Category::Gameplay {
                settings.bot_difficulty = defaults.bot_difficulty;
            }
        }
    }
}

fn refresh(
    settings: Res<PlayerSettings>,
    mut fills: Query<(&SliderFill, &mut Node), Without<SliderThumb>>,
    mut thumbs: Query<(&SliderThumb, &mut Node), Without<SliderFill>>,
    mut values: Query<(&SettingValue, &mut Text)>,
    mut difficulties: Query<(
        &DifficultyButton,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    if settings.is_changed() {
        for (fill, mut node) in &mut fills {
            node.width = Val::Percent(fill.0.normalized(&settings) * 100.);
        }
        for (thumb, mut node) in &mut thumbs {
            node.left = Val::Percent(thumb.0.normalized(&settings) * 100.);
        }
        for (value, mut text) in &mut values {
            **text = value.0.value(&settings);
        }
    }
    for (button, interaction, mut background, mut border) in &mut difficulties {
        let selected = button.0 == settings.bot_difficulty;
        background.0 = if selected {
            GLASS_SELECTED
        } else if *interaction == Interaction::Hovered {
            Color::srgba(1., 1., 1., 0.06)
        } else {
            Color::NONE
        };
        border.0 = if selected { ACCENT } else { LINE };
    }
}

fn scroll(
    mut wheel: EventReader<MouseWheel>,
    mut viewports: Query<(&mut ScrollPosition, &ComputedNode), With<SettingsScroll>>,
) {
    for event in wheel.read() {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * 32.,
            MouseScrollUnit::Pixel => event.y,
        };
        for (mut position, node) in &mut viewports {
            let max = (node.content_size().y - node.size().y).max(0.) * node.inverse_scale_factor();
            position.offset_y = (position.offset_y - delta).clamp(0., max);
        }
    }
}
