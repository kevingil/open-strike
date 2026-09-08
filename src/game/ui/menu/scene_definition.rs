//! Menu-owned scene metadata. No gameplay map, navigation or collision handles.
use crate::game::{
    config::MapId,
    map::config::{ColorRgb, DirectionalLightConfig, LightingConfig, Vec3Config},
};
use bevy::prelude::*;
pub struct MenuSceneDefinition {
    pub scene: &'static str,
    pub thumbnail: &'static str,
    pub camera: Transform,
    pub character: Transform,
    pub lighting: LightingConfig,
}
pub const MENU_VERTICAL_FOV: f32 = 50.0;
pub fn menu_scene(map: &MapId) -> MenuSceneDefinition {
    // Internal practice maps explicitly fall back to the authored Dust 2 menu.
    match map {
        MapId::Dust2 | MapId::Warehouse => dust2(),
    }
}
fn dust2() -> MenuSceneDefinition {
    MenuSceneDefinition {
        scene: "generated/menu/dust2.glb#Scene0",
        thumbnail: "generated/menu/dust2-card.png",
        // A near-level, full-body portrait: head just below the bar, boots near the footer.
        camera: Transform::from_xyz(0., 1.05, 2.4).looking_at(Vec3::new(0., 0.94, 0.), Vec3::Y),
        character: Transform::from_rotation(Quat::from_rotation_y(-0.32)),
        lighting: LightingConfig {
            ambient_color: ColorRgb {
                r: 0.82,
                g: 0.88,
                b: 1.,
            },
            ambient_brightness: 5000.,
            point_lights: vec![],
            directional_lights: vec![DirectionalLightConfig {
                direction: Vec3Config {
                    x: -0.5,
                    y: 1.,
                    z: 0.6,
                },
                color: ColorRgb {
                    r: 1.,
                    g: 0.94,
                    b: 0.80,
                },
                illuminance: 65000.,
                shadows: true,
            }],
        },
    }
}
