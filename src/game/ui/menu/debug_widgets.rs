//! Trait-based widget system for building modular debug UIs.
//!
//! Provides reusable UI components like sliders, button rows, and toggle buttons
//! that can be composed to build debug panels.

use bevy::prelude::*;

/// Button colors used across debug widgets
pub const BTN_NORMAL: Color = Color::srgba(0.2, 0.2, 0.3, 0.9);
pub const BTN_HOVER: Color = Color::srgba(0.3, 0.3, 0.5, 0.9);
pub const BTN_ACTIVE: Color = Color::srgba(0.2, 0.6, 0.3, 0.9);
pub const SLIDER_BG: Color = Color::srgba(0.1, 0.1, 0.15, 1.0);
pub const SLIDER_FILL: Color = Color::srgb(0.2, 0.5, 0.7);

/// Marker component for slider fill elements
#[derive(Component)]
pub struct SliderFill;

/// A simple button with a label and action component
#[derive(Clone)]
pub struct ButtonWidget<T: Component + Clone> {
    pub label: &'static str,
    pub action: T,
}

impl<T: Component + Clone> ButtonWidget<T> {
    pub fn new(label: &'static str, action: T) -> Self {
        Self { label, action }
    }

    /// Spawn this button as a child of the given entity commands
    pub fn spawn_in(self, commands: &mut Commands) -> Entity {
        commands
            .spawn((
                self.action,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_child((
                Text::new(self.label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .id()
    }
}

/// Trait for spawnable debug UI widgets that can add themselves to an entity
pub trait DebugWidget {
    /// Spawn this widget and add it as a child of the given entity
    fn spawn(self, commands: &mut Commands, parent: Entity);
}

/// A horizontal row of buttons
pub struct ButtonRowWidget<T: Component + Clone> {
    pub buttons: Vec<ButtonWidget<T>>,
    pub gap: f32,
}

impl<T: Component + Clone> ButtonRowWidget<T> {
    pub fn new(buttons: Vec<ButtonWidget<T>>) -> Self {
        Self { buttons, gap: 4.0 }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl<T: Component + Clone> DebugWidget for ButtonRowWidget<T> {
    fn spawn(self, commands: &mut Commands, parent: Entity) {
        let row = commands
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(self.gap),
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            })
            .id();

        commands.entity(parent).add_child(row);

        for button in self.buttons {
            let btn_entity = button.spawn_in(commands);
            commands.entity(row).add_child(btn_entity);
        }
    }
}

/// A labeled row with +/- buttons for adjusting a value
pub struct AdjustRowWidget<T: Component + Clone> {
    pub label: &'static str,
    pub minus_action: T,
    pub plus_action: T,
    pub label_width: f32,
}

impl<T: Component + Clone> AdjustRowWidget<T> {
    pub fn new(label: &'static str, minus_action: T, plus_action: T) -> Self {
        Self {
            label,
            minus_action,
            plus_action,
            label_width: 35.0,
        }
    }

    pub fn with_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }
}

impl<T: Component + Clone> DebugWidget for AdjustRowWidget<T> {
    fn spawn(self, commands: &mut Commands, parent: Entity) {
        let row = commands
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .id();

        commands.entity(parent).add_child(row);

        // Label
        let label_entity = commands
            .spawn((
                Text::new(self.label),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    width: Val::Px(self.label_width),
                    ..default()
                },
            ))
            .id();
        commands.entity(row).add_child(label_entity);

        // Minus button
        let minus_entity = ButtonWidget::new("-", self.minus_action).spawn_in(commands);
        commands.entity(row).add_child(minus_entity);

        // Plus button
        let plus_entity = ButtonWidget::new("+", self.plus_action).spawn_in(commands);
        commands.entity(row).add_child(plus_entity);
    }
}

/// A toggle button that displays on/off state
pub struct ToggleWidget<T: Component + Clone> {
    pub label: &'static str,
    pub action: T,
    pub initial_state: bool,
}

impl<T: Component + Clone> ToggleWidget<T> {
    pub fn new(label: &'static str, action: T, initial_state: bool) -> Self {
        Self {
            label,
            action,
            initial_state,
        }
    }
}

impl<T: Component + Clone> DebugWidget for ToggleWidget<T> {
    fn spawn(self, commands: &mut Commands, parent: Entity) {
        let bg_color = if self.initial_state {
            BTN_ACTIVE
        } else {
            BTN_NORMAL
        };
        let btn = commands
            .spawn((
                self.action,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(bg_color),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_child((
                Text::new(self.label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .id();

        commands.entity(parent).add_child(btn);
    }
}

/// A text display widget for showing values
pub struct DisplayWidget<T: Component + Clone> {
    pub marker: T,
    pub initial_text: String,
}

impl<T: Component + Clone> DisplayWidget<T> {
    pub fn new(marker: T, initial_text: impl Into<String>) -> Self {
        Self {
            marker,
            initial_text: initial_text.into(),
        }
    }
}

impl<T: Component + Clone> DebugWidget for DisplayWidget<T> {
    fn spawn(self, commands: &mut Commands, parent: Entity) {
        let display = commands
            .spawn((
                self.marker,
                Text::new(&self.initial_text),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .id();

        commands.entity(parent).add_child(display);
    }
}

/// Helper to calculate slider value from cursor position
pub fn calculate_slider_value(
    cursor_pos: Vec2,
    node: &Node,
    transform: &GlobalTransform,
    min: f32,
    max: f32,
) -> Option<f32> {
    let width = match node.width {
        Val::Px(w) => w,
        Val::Percent(p) => p * 5.0, // Rough estimate
        _ => return None,
    };

    let left = transform.translation().x - width / 2.0;
    let normalized = ((cursor_pos.x - left) / width).clamp(0.0, 1.0);
    Some(min + normalized * (max - min))
}

/// Update a slider's fill width based on a normalized value (0.0 - 1.0)
pub fn update_slider_fill(
    children: &Children,
    fill_query: &mut Query<&mut Node, With<SliderFill>>,
    normalized: f32,
) {
    for child in children.iter() {
        if let Ok(mut fill_node) = fill_query.get_mut(child) {
            fill_node.width = Val::Percent(normalized.clamp(0.0, 1.0) * 100.0);
        }
    }
}
