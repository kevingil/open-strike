//! Player skin definitions and shared hitbox configuration.
//!
//! Each skin has motions baked for its own rest skeleton. Damage zones are shared.
//! Cosmetic dimensions never determine collision or damage.

use bevy::prelude::*;

/// Team side - determines which team can use this skin
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerSide {
    Attacker,
    Defender,
    Any, // Custom skins usable by either team
}

impl PlayerSide {
    pub fn name(&self) -> &'static str {
        match self {
            PlayerSide::Attacker => "Attacker",
            PlayerSide::Defender => "Defender",
            PlayerSide::Any => "Any",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            PlayerSide::Attacker => Color::srgb(0.9, 0.3, 0.2), // Red
            PlayerSide::Defender => Color::srgb(0.2, 0.5, 0.9), // Blue
            PlayerSide::Any => Color::srgb(0.6, 0.6, 0.6),      // Gray
        }
    }
}

/// Unique identifier for a skin
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum SkinId {
    #[default]
    Soldier,
    Police,
}

impl SkinId {
    pub fn all() -> Vec<SkinId> {
        vec![SkinId::Soldier, SkinId::Police]
    }
}

/// Hitbox zone type for damage calculation
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitboxZoneType {
    Head,
    Torso,
    Legs,
}

impl HitboxZoneType {
    /// Get the damage multiplier for this zone from the standard hitbox
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            HitboxZoneType::Head => STANDARD_HITBOX.head.damage_multiplier,
            HitboxZoneType::Torso => STANDARD_HITBOX.torso.damage_multiplier,
            HitboxZoneType::Legs => STANDARD_HITBOX.legs.damage_multiplier,
        }
    }

    /// Get the hitbox zone for this type from the standard hitbox
    pub fn zone(&self) -> &'static HitboxZone {
        match self {
            HitboxZoneType::Head => &STANDARD_HITBOX.head,
            HitboxZoneType::Torso => &STANDARD_HITBOX.torso,
            HitboxZoneType::Legs => &STANDARD_HITBOX.legs,
        }
    }
}

/// Hitbox zone with damage multiplier
#[derive(Clone, Debug)]
pub struct HitboxZone {
    /// Offset relative to model root
    pub offset: Vec3,
    /// Box half-size (for AABB collision)
    pub half_extents: Vec3,
    /// Damage multiplier for hits to this zone
    pub damage_multiplier: f32,
}

impl HitboxZone {
    pub const fn new(offset: Vec3, half_extents: Vec3, damage_multiplier: f32) -> Self {
        Self {
            offset,
            half_extents,
            damage_multiplier,
        }
    }
}

/// Hitbox configuration - defines damage zones for a player model
#[derive(Clone, Debug)]
pub struct HitboxConfig {
    /// High damage zone (headshots)
    pub head: HitboxZone,
    /// Normal damage zone
    pub torso: HitboxZone,
    /// Reduced damage zone
    pub legs: HitboxZone,
}

/// Standard hitbox configuration used by ALL players.
/// This ensures competitive fairness - all skins have identical hitboxes.
pub const STANDARD_HITBOX: HitboxConfig = HitboxConfig {
    head: HitboxZone {
        offset: Vec3::new(0.0, 1.6, 0.0),          // Head at top
        half_extents: Vec3::new(0.15, 0.15, 0.15), // Small head hitbox
        damage_multiplier: 2.5,                    // Headshot bonus
    },
    torso: HitboxZone {
        offset: Vec3::new(0.0, 1.1, 0.0),          // Torso in middle
        half_extents: Vec3::new(0.25, 0.35, 0.15), // Larger torso
        damage_multiplier: 1.0,                    // Standard damage
    },
    legs: HitboxZone {
        offset: Vec3::new(0.0, 0.45, 0.0),        // Legs at bottom
        half_extents: Vec3::new(0.2, 0.45, 0.12), // Leg hitbox
        damage_multiplier: 0.75,                  // Reduced damage
    },
};

/// Skin definition - purely cosmetic, no gameplay differences.
/// Named animation semantics and damage zones are shared; baked rig poses differ.
#[derive(Clone, Debug)]
pub struct SkinDefinition {
    /// Unique identifier
    pub id: SkinId,
    /// Display name
    pub name: &'static str,
    /// Path to the GLB model file
    pub model_path: &'static str,
    /// Which team can use this skin
    pub side: PlayerSide,
}

/// Registry of all available skins
#[derive(Resource)]
pub struct SkinRegistry {
    pub skins: Vec<SkinDefinition>,
}

impl Default for SkinRegistry {
    fn default() -> Self {
        Self {
            skins: vec![
                // Soldier - Attacker
                SkinDefinition {
                    id: SkinId::Soldier,
                    name: "Soldier",
                    model_path: "generated/attacker.glb#Scene0",
                    side: PlayerSide::Attacker,
                },
                // Police - Defender
                SkinDefinition {
                    id: SkinId::Police,
                    name: "Police Defender",
                    model_path: "generated/defender.glb#Scene0",
                    side: PlayerSide::Defender,
                },
            ],
        }
    }
}

impl SkinRegistry {
    /// Get a skin definition by ID
    pub fn get(&self, id: SkinId) -> Option<&SkinDefinition> {
        self.skins.iter().find(|s| s.id == id)
    }

    /// Get all skins for a specific side (includes Any)
    pub fn get_for_side(&self, side: PlayerSide) -> Vec<&SkinDefinition> {
        self.skins
            .iter()
            .filter(|s| s.side == side || s.side == PlayerSide::Any)
            .collect()
    }
}
