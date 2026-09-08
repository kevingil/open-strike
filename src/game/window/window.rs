use bevy::{
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode, WindowResolution},
};

use crate::game::cursor::*;
pub struct WindowSettingsPlugin;
impl Plugin for WindowSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(cursor::CursorPlugin)
            .add_systems(PreStartup, init_window);
    }
}

fn init_window(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        window.title = "Open Strike".into();
        window.resolution = WindowResolution::new(1920., 1080.);
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
        if std::env::var_os("CSRS_CAPTURE").is_some()
            && std::env::var_os("CSRS_FULLSCREEN").is_none()
        {
            window.mode = WindowMode::Windowed;
            let width = std::env::var("CSRS_WIDTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1280.);
            let height = std::env::var("CSRS_HEIGHT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(720.);
            window.resolution =
                WindowResolution::new(width, height).with_scale_factor_override(1.0);
        }
    }
}
