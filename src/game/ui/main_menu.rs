use bevy::prelude::*;

/// This plugin now just spawns the UI camera.
/// The actual menu UI is handled by the menu module.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui_camera);
    }
}

/// Marker for the UI camera (persists across all states)
#[derive(Component)]
pub struct UiCamera;

fn setup_ui_camera(mut commands: Commands) {
    // Spawn a 2D camera for UI rendering (always present)
    // Order 100 means it renders AFTER 3D cameras, and doesn't clear the framebuffer
    commands.spawn((
        UiCamera,
        Camera2d,
        Camera {
            hdr: true,
            order: 100,
            clear_color: bevy::render::camera::ClearColorConfig::None,
            ..default()
        },
    ));
}
