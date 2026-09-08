//! Shared presentation values for the local menu.
use bevy::prelude::*;
pub const HEADER: f32 = 64.0;
pub const INK: Color = Color::srgba(0.045, 0.055, 0.065, 0.88);
pub const GLASS: Color = Color::srgba(0.055, 0.075, 0.095, 0.84);
pub const GLASS_SELECTED: Color = Color::srgba(0.30, 0.54, 0.63, 0.30);
pub const PANEL: Color = Color::srgba(0.055, 0.067, 0.078, 0.86);
pub const WHITE: Color = Color::srgb(0.97, 0.98, 0.99);
pub const MUTED: Color = Color::srgb(0.80, 0.84, 0.87);
pub const ACCENT: Color = Color::srgb(0.35, 0.80, 0.86);
pub const SELECTED: Color = Color::srgba(0.17, 0.36, 0.40, 0.86);
#[derive(Component)]
pub struct MenuText;
pub fn apply_fonts(server: Res<AssetServer>, mut labels: Query<&mut TextFont, Added<MenuText>>) {
    for mut font in &mut labels {
        font.font = server.load("fonts/RobotoCondensed.ttf");
    }
}
pub fn label(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        MenuText,
        Text::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(1.0),
            color: Color::srgba(0., 0., 0., 0.8),
        },
    )
}
pub fn page() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.),
        right: Val::Px(56.),
        top: Val::Px(HEADER),
        bottom: Val::Px(0.),
        padding: UiRect::all(Val::VMin(4.)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(24.),
        ..default()
    }
}
pub fn button() -> Node {
    Node {
        padding: UiRect::axes(Val::Px(24.), Val::Px(14.)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        border: UiRect::bottom(Val::Px(2.)),
        ..default()
    }
}
