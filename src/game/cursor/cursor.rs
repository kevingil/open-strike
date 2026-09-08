use crate::game::{ui::pause_menu::ExitConfirmation, GameState};
use bevy::{
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (manage_cursor, update_cursor));
    }
}
fn update_cursor(
    state: Res<State<GameState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let active = *state.get() == GameState::Playing && window.focused;
    window.cursor_options.grab_mode = if active {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    window.cursor_options.visible = !active;
}
fn manage_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut confirmation: ResMut<ExitConfirmation>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if *state.get() == GameState::Playing
        && windows.single().is_ok_and(|w| !w.focused)
        && std::env::var_os("CSRS_CAPTURE").is_none()
    {
        next.set(GameState::Paused);
    }
    if keys.just_pressed(KeyCode::Escape) {
        if confirmation.open {
            confirmation.open = false;
            return;
        }
        match state.get() {
            GameState::Playing => next.set(GameState::Paused),
            GameState::Paused => next.set(GameState::Playing),
            GameState::Loading | GameState::LoadFailed | GameState::Finished => {
                next.set(GameState::MainMenu)
            }
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::Enter)
        && matches!(state.get(), GameState::LoadFailed | GameState::Finished)
    {
        next.set(GameState::Loading);
    }
}
