use super::player::LocalPlayer;
use crate::game::{
    config::{PlayerSettings, WeaponId},
    matchplay::{ActorIntent, Combatant},
    weapons::WeaponSelection,
};
use bevy::{input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};
use bevy_fps_controller::controller::FpsControllerInput;

pub fn human_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    settings: Res<PlayerSettings>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut FpsControllerInput, &mut ActorIntent, &Combatant), With<LocalPlayer>>,
) {
    let delta: Vec2 = motion.read().map(|e| e.delta).sum();
    let Ok((mut input, mut intent, actor)) = query.single_mut() else {
        return;
    };
    if !actor.alive() || !window.single().is_ok_and(|w| w.focused) {
        input.movement = Vec3::ZERO;
        input.jump = false;
        input.crouch = false;
        input.sprint = false;
        intent.fire = false;
        intent.reload = false;
        intent.selection = None;
        return;
    }
    input.yaw -= delta.x * settings.sensitivity * 0.001;
    input.pitch = (input.pitch - delta.y * settings.sensitivity * 0.001).clamp(-1.52, 1.52);
    input.movement = Vec3::new(
        (keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32) as f32,
        0.0,
        (keys.pressed(KeyCode::KeyW) as i32 - keys.pressed(KeyCode::KeyS) as i32) as f32,
    );
    input.sprint = keys.pressed(KeyCode::ShiftLeft);
    input.crouch = keys.pressed(KeyCode::ControlLeft);
    input.jump = keys.pressed(KeyCode::Space);
    input.fly = false;
    intent.fire = mouse.pressed(MouseButton::Left);
    intent.reload |= keys.just_pressed(KeyCode::KeyR);
    if keys.just_pressed(KeyCode::Digit1) {
        intent.selection = Some(WeaponSelection::Select(WeaponId::AK47));
    } else if keys.just_pressed(KeyCode::Digit3) {
        intent.selection = Some(WeaponSelection::Select(WeaponId::DefaultKnife));
    } else if keys.just_pressed(KeyCode::KeyF) {
        intent.selection = Some(WeaponSelection::Previous);
    }
}
