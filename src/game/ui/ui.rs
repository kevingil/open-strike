use bevy::prelude::*;

use super::{crosshair, main_menu, menu, pause_menu};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            super::hud::HudPlugin,
            main_menu::MainMenuPlugin,
            menu::MenuPlugin,
            pause_menu::PauseMenuPlugin,
            crosshair::CrosshairPlugin,
        ));
    }
}
