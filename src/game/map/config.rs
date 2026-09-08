//! Map configuration structs that can be loaded from RON files.

use bevy::asset::Asset;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Main map configuration - loaded from .map.ron files
#[derive(Asset, TypePath, Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MapConfig {
    #[serde(default)]
    pub navigation_bounds: Option<(Vec3Config, Vec3Config)>,
    #[serde(default)]
    pub navigation: Vec<crate::game::bots::NavNode>,
    #[serde(default)]
    pub patrol_destinations: Vec<usize>,
    #[serde(default)]
    pub callouts: Vec<MapCallout>,
    /// Display name of the map
    pub name: String,
    /// Path to the map model file (relative to the map folder)
    pub model: String,
    /// Optional preview image path (relative to the map folder)
    #[serde(default)]
    pub preview_image: Option<String>,
    /// Transform for the map model
    pub transform: MapTransform,
    /// Lighting configuration
    pub lighting: LightingConfig,
    /// Camera settings
    pub camera: CameraConfig,
    /// Optional fog settings
    pub fog: Option<FogConfig>,
    /// Post-processing effects
    pub post_process: PostProcessConfig,
    /// Player spawn points (empty for non-gameplay maps like home)
    #[serde(default)]
    pub spawn_points: Vec<SpawnPoint>,
    /// Sky/clear color
    #[serde(default)]
    pub clear_color: Option<ColorRgb>,
}

/// Transform configuration for map placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapTransform {
    pub position: Vec3Config,
    pub scale: f32,
    /// Rotation as axis-angle: (axis_x, axis_y, axis_z, angle_radians)
    #[serde(default = "default_rotation")]
    pub rotation: (f32, f32, f32, f32),
}

fn default_rotation() -> (f32, f32, f32, f32) {
    (0.0, 1.0, 0.0, 0.0)
}

impl MapTransform {
    pub fn to_transform(&self) -> Transform {
        Transform::from_xyz(self.position.x, self.position.y, self.position.z)
            .with_scale(Vec3::splat(self.scale))
            .with_rotation(Quat::from_axis_angle(
                Vec3::new(self.rotation.0, self.rotation.1, self.rotation.2),
                self.rotation.3,
            ))
    }
}

/// Lighting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingConfig {
    /// Ambient light color
    pub ambient_color: ColorRgb,
    /// Ambient light brightness
    pub ambient_brightness: f32,
    /// Point lights in the scene
    #[serde(default)]
    pub point_lights: Vec<PointLightConfig>,
    /// Directional lights (sun)
    #[serde(default)]
    pub directional_lights: Vec<DirectionalLightConfig>,
}

/// Point light configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointLightConfig {
    pub position: Vec3Config,
    pub color: ColorRgb,
    pub intensity: f32,
    pub range: f32,
    #[serde(default = "default_true")]
    pub shadows: bool,
}

fn default_true() -> bool {
    true
}

/// Directional light configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectionalLightConfig {
    pub direction: Vec3Config,
    pub color: ColorRgb,
    pub illuminance: f32,
    #[serde(default = "default_true")]
    pub shadows: bool,
}

/// Camera configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Exposure EV100 value (higher = darker scene)
    pub exposure_ev100: f32,
    /// Field of view in degrees
    #[serde(default = "default_fov")]
    pub fov: f32,
}

fn default_fov() -> f32 {
    60.0
}

/// Fog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogConfig {
    pub color: ColorRgba,
    /// Exponential fog density
    pub density: f32,
}

/// Post-processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessConfig {
    pub bloom_intensity: f32,
    #[serde(default = "default_tonemapping")]
    pub tonemapping: TonemappingConfig,
}

fn default_tonemapping() -> TonemappingConfig {
    TonemappingConfig::TonyMcMapface
}

/// Tonemapping options (serializable)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum TonemappingConfig {
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    AgX,
    SomewhatBoringDisplayTransform,
    #[default]
    TonyMcMapface,
    BlenderFilmic,
}

impl TonemappingConfig {
    pub fn to_bevy(&self) -> Tonemapping {
        match self {
            TonemappingConfig::None => Tonemapping::None,
            TonemappingConfig::Reinhard => Tonemapping::Reinhard,
            TonemappingConfig::ReinhardLuminance => Tonemapping::ReinhardLuminance,
            TonemappingConfig::AcesFitted => Tonemapping::AcesFitted,
            TonemappingConfig::AgX => Tonemapping::AgX,
            TonemappingConfig::SomewhatBoringDisplayTransform => {
                Tonemapping::SomewhatBoringDisplayTransform
            }
            TonemappingConfig::TonyMcMapface => Tonemapping::TonyMcMapface,
            TonemappingConfig::BlenderFilmic => Tonemapping::BlenderFilmic,
        }
    }
}

/// Spawn point configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    #[serde(default)]
    pub team: Option<crate::game::matchplay::Team>,
    pub position: Vec3Config,
    #[serde(default)]
    pub rotation: f32, // Y rotation in degrees
}

/// RGB color (0.0-1.0 range)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl ColorRgb {
    pub fn to_color(&self) -> Color {
        Color::srgb(self.r, self.g, self.b)
    }
}

/// RGBA color (0.0-1.0 range)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    #[serde(default = "default_alpha")]
    pub a: f32,
}

fn default_alpha() -> f32 {
    1.0
}

impl ColorRgba {
    pub fn to_color(&self) -> Color {
        Color::srgba(self.r, self.g, self.b, self.a)
    }
}

/// Vec3 configuration (serializable)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec3Config {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Config {
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl From<Vec3Config> for Vec3 {
    fn from(v: Vec3Config) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapCallout {
    pub name: String,
    pub position: Vec3Config,
    pub radius: f32,
}
