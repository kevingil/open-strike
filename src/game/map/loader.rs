//! Map loading utilities for spawning scenes from MapConfig.

use bevy::core_pipeline::bloom::Bloom;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::camera::Exposure;

use super::config::*;

pub struct MapLoaderPlugin;

impl Plugin for MapLoaderPlugin {
    fn build(&self, _app: &mut App) {
        // Plugin is a placeholder for now - systems are added by specific scenes
    }
}

/// Marker component for entities spawned by the map loader
#[derive(Component)]
pub struct MapEntity;

/// Spawns lighting entities based on map config
pub fn spawn_lighting(
    commands: &mut Commands,
    config: &LightingConfig,
    marker: impl Component + Clone,
) {
    commands.insert_resource(AmbientLight {
        color: config.ambient_color.to_color(),
        brightness: config.ambient_brightness,
        affects_lightmapped_meshes: true,
    });

    // Point lights
    for light in &config.point_lights {
        commands.spawn((
            marker.clone(),
            PointLight {
                color: light.color.to_color(),
                intensity: light.intensity,
                range: light.range,
                shadows_enabled: light.shadows,
                ..default()
            },
            Transform::from_xyz(light.position.x, light.position.y, light.position.z),
        ));
    }

    // Directional lights
    for light in &config.directional_lights {
        commands.spawn((
            marker.clone(),
            DirectionalLight {
                color: light.color.to_color(),
                illuminance: light.illuminance,
                shadows_enabled: light.shadows,
                ..default()
            },
            Transform::from_xyz(
                light.direction.x * 100.0,
                light.direction.y * 100.0,
                light.direction.z * 100.0,
            )
            .looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }
}

/// Creates camera components based on map config
/// Returns a tuple of components to add to the camera entity
pub fn create_camera_components(
    config: &MapConfig,
) -> (
    Exposure,
    Bloom,
    bevy::core_pipeline::tonemapping::Tonemapping,
    Option<DistanceFog>,
) {
    let exposure = Exposure {
        ev100: config.camera.exposure_ev100,
    };

    let bloom = Bloom {
        intensity: config.post_process.bloom_intensity,
        low_frequency_boost: 0.2,
        low_frequency_boost_curvature: 0.7,
        high_pass_frequency: 1.0,
        ..default()
    };

    let tonemapping = config.post_process.tonemapping.to_bevy();

    let fog = config.fog.as_ref().map(|f| DistanceFog {
        color: f.color.to_color(),
        falloff: FogFalloff::Exponential { density: f.density },
        ..default()
    });

    (exposure, bloom, tonemapping, fog)
}

/// Spawns a 3D camera with settings from the map config
pub fn spawn_camera_with_config(
    commands: &mut Commands,
    config: &MapConfig,
    transform: Transform,
    marker: impl Component + Clone,
) -> Entity {
    let (exposure, bloom, tonemapping, fog) = create_camera_components(config);

    let mut entity_commands = commands.spawn((
        marker,
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: config.camera.fov.to_radians(),
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
        Camera {
            order: 0,
            hdr: true,
            ..default()
        },
        transform,
        exposure,
        bloom,
        tonemapping,
    ));

    if let Some(fog) = fog {
        entity_commands.insert(fog);
    }

    entity_commands.id()
}

/// Spawns the map model
pub fn spawn_map_model(
    commands: &mut Commands,
    asset_server: &AssetServer,
    config: &MapConfig,
    base_path: &str,
    marker: impl Component + Clone,
) -> Entity {
    let model_path = format!("{}/{}", base_path, config.model);

    commands
        .spawn((
            marker,
            SceneRoot(asset_server.load(format!("{}#Scene0", model_path))),
            config.transform.to_transform(),
        ))
        .id()
}
