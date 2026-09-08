//! Animated contour-map material for the map picker's primary action.
use super::play_tab::StartGameButton;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub(super) const LABEL_COLOR: Color = Color::srgb(0.57, 1.0, 0.12);

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, Default)]
pub(super) struct StartButtonMaterial {
    // Elapsed time, eased hover intensity, eased press intensity, reserved.
    #[uniform(0)]
    animation: Vec4,
}

impl UiMaterial for StartButtonMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/start_button.wgsl".into()
    }
}

pub(super) fn animate(
    time: Res<Time>,
    mut buttons: Query<
        (
            &Interaction,
            &MaterialNode<StartButtonMaterial>,
            &mut BoxShadow,
        ),
        With<StartGameButton>,
    >,
    mut materials: ResMut<Assets<StartButtonMaterial>>,
) {
    let ease = 1.0 - (-12.0 * time.delta_secs()).exp();
    for (interaction, handle, mut shadow) in &mut buttons {
        if let Some(material) = materials.get_mut(&handle.0) {
            let hover = f32::from(*interaction != Interaction::None);
            let pressed = f32::from(*interaction == Interaction::Pressed);
            material.animation.x = time.elapsed_secs();
            material.animation.y += (hover - material.animation.y) * ease;
            material.animation.z += (pressed - material.animation.z) * ease;
            for layer in &mut shadow.0 {
                layer.color = LABEL_COLOR.with_alpha(material.animation.y * 0.30);
            }
        }
    }
}
