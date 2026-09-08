//! Interpolate fixed-tick movement while keeping first-person mouse look responsive.
use bevy::prelude::*;
use bevy_fps_controller::controller::{
    FpsController, FpsControllerInput, LogicalPlayer, RenderPlayer,
};
#[derive(Component)]
pub struct PoseHistory {
    pub position: Vec3,
    pub height: f32,
    pub yaw: f32,
}
impl PoseHistory {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            position,
            height: super::player::BODY_HEIGHT,
            yaw,
        }
    }
}
pub fn remember(
    mut actors: Query<(&Transform, &FpsController, &mut PoseHistory), With<LogicalPlayer>>,
) {
    for (transform, controller, mut history) in &mut actors {
        history.position = transform.translation;
        history.height = controller.height;
        history.yaw = controller.yaw;
    }
}
pub fn position(history: &PoseHistory, current: Vec3, alpha: f32) -> Vec3 {
    if history.position.distance_squared(current) > 4.0 {
        current
    } else {
        history.position.lerp(current, alpha)
    }
}
pub fn cameras(
    time: Res<Time<Fixed>>,
    actors: Query<
        (
            &Transform,
            &FpsController,
            &FpsControllerInput,
            &PoseHistory,
        ),
        With<LogicalPlayer>,
    >,
    mut cameras: Query<(&RenderPlayer, &mut Transform), Without<LogicalPlayer>>,
) {
    let alpha = time.overstep_fraction();
    for (render, mut transform) in &mut cameras {
        if let Ok((body, controller, input, history)) = actors.get(render.logical_entity) {
            transform.translation = position(history, body.translation, alpha)
                + Vec3::Y
                    * ((history.height + (controller.height - history.height) * alpha) * 0.5
                        - 0.15);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
        }
    }
}
