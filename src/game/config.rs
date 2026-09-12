use crate::game::player::skins::{PlayerSide, SkinId};
use bevy::prelude::*;
use std::time::Duration;

/// Main game configuration resource
#[derive(Resource, Clone)]
pub struct GameConfig {
    pub mode: GameMode,
    pub map: MapId,
    pub match_settings: MatchSettings,
    pub bot_difficulty: BotDifficulty,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            mode: GameMode::default(),
            map: MapId::default(),
            match_settings: MatchSettings::default(),
            bot_difficulty: BotDifficulty::default(),
        }
    }
}

/// Shared by all bots; a future settings selector can update this configuration.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotDifficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

/// Available game modes
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash)]
pub enum GameMode {
    Freemode,
    Deathmatch,
    #[default]
    TeamDeathmatch,
}

impl GameMode {
    pub fn name(&self) -> &'static str {
        match self {
            GameMode::Freemode => "Freemode",
            GameMode::Deathmatch => "Deathmatch",
            GameMode::TeamDeathmatch => "Team Deathmatch",
        }
    }

    /// Whether actors are grouped into two sides; free-for-all treats everyone as hostile.
    pub fn teams(&self) -> bool {
        matches!(self, GameMode::TeamDeathmatch)
    }

    /// Whether the match keeps score and ends on time or score limits.
    pub fn scored(&self) -> bool {
        matches!(self, GameMode::Deathmatch | GameMode::TeamDeathmatch)
    }

    /// Short identifier used by the hub and the server command line.
    pub fn key(&self) -> &'static str {
        match self {
            GameMode::Freemode => "free",
            GameMode::Deathmatch => "dm",
            GameMode::TeamDeathmatch => "tdm",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "free" => Some(GameMode::Freemode),
            "dm" => Some(GameMode::Deathmatch),
            "tdm" => Some(GameMode::TeamDeathmatch),
            _ => None,
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            GameMode::Freemode => 0,
            GameMode::Deathmatch => 1,
            GameMode::TeamDeathmatch => 2,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => GameMode::Deathmatch,
            2 => GameMode::TeamDeathmatch,
            _ => GameMode::Freemode,
        }
    }
}

/// Available maps
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash)]
pub enum MapId {
    Warehouse,
    #[default]
    Dust2,
}

impl MapId {
    pub fn name(&self) -> &'static str {
        match self {
            MapId::Warehouse => "Warehouse",
            MapId::Dust2 => "Dust 2",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            MapId::Warehouse => "warehouse",
            MapId::Dust2 => "dust2",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "warehouse" => Some(MapId::Warehouse),
            "dust2" => Some(MapId::Dust2),
            _ => None,
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            MapId::Warehouse => 0,
            MapId::Dust2 => 1,
        }
    }

    pub fn from_code(code: u8) -> Self {
        if code == 0 {
            MapId::Warehouse
        } else {
            MapId::Dust2
        }
    }

    /// Get the config file path for this map
    pub fn config_path(&self) -> &'static str {
        match self {
            MapId::Warehouse => "maps/warehouse/config.map.ron",
            MapId::Dust2 => "maps/de_dust_2/config.map.ron",
        }
    }
}

/// Match-specific settings
#[derive(Clone, Debug)]
pub struct MatchSettings {
    pub time_limit: Option<Duration>, // None = unlimited
    pub score_limit: Option<u32>,     // None = unlimited
    pub respawn_time: Duration,       // 0 = instant
}

impl Default for MatchSettings {
    fn default() -> Self {
        Self {
            time_limit: Some(Duration::from_secs(600)),
            score_limit: Some(50),
            respawn_time: Duration::from_secs(3),
        }
    }
}

/// Resource for player settings
#[derive(Resource)]
pub struct PlayerSettings {
    pub sensitivity: f32,
    pub fov: f32,
    pub master_volume: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            fov: 103.0,
            master_volume: 1.0,
        }
    }
}

/// Available weapons for loadout
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponId {
    AK47,
    DefaultKnife,
    ReferenceKnife,
}

impl WeaponId {
    pub fn is_knife(self) -> bool {
        matches!(self, Self::DefaultKnife | Self::ReferenceKnife)
    }

    pub fn name(&self) -> &'static str {
        match self {
            WeaponId::AK47 => "AK-47",
            WeaponId::DefaultKnife => "Default Knife",
            WeaponId::ReferenceKnife => "Reference Knife",
        }
    }

    pub fn all() -> Vec<WeaponId> {
        vec![
            WeaponId::AK47,
            WeaponId::DefaultKnife,
            WeaponId::ReferenceKnife,
        ]
    }
}

/// Resource for player loadout
#[derive(Resource)]
pub struct PlayerLoadout {
    pub primary_weapon: WeaponId,
    pub selected_skin: SkinId,
    pub melee_weapon: WeaponId,
    /// Weapons offered for purchase, independent of the currently held weapon.
    pub buy_weapons: BuyLoadouts,
}

impl Default for PlayerLoadout {
    fn default() -> Self {
        Self {
            primary_weapon: WeaponId::AK47,
            melee_weapon: WeaponId::DefaultKnife,
            selected_skin: SkinId::default(),
            buy_weapons: BuyLoadouts::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuyCategory {
    Pistols,
    MidTier,
    Rifles,
}

impl BuyCategory {
    pub const ALL: [Self; 3] = [Self::Pistols, Self::MidTier, Self::Rifles];

    pub fn name(self) -> &'static str {
        match self {
            Self::Pistols => "Pistols",
            Self::MidTier => "Mid-Tier",
            Self::Rifles => "Rifles",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Pistols => "Secondary weapons",
            Self::MidTier => "SMGs and shotguns",
            Self::Rifles => "Rifles and snipers",
        }
    }

    pub fn accepts(self, weapon: WeaponId) -> bool {
        // Expand this catalog when additional weapons are implemented.
        matches!((self, weapon), (Self::Rifles, WeaponId::AK47))
    }
}

pub const BUY_SLOT_COUNT: usize = 5;
pub const PRESET_ARMOR: [&str; 2] = ["Kevlar vest", "Kevlar + helmet"];
pub const PRESET_GRENADES: [&str; 5] = [
    "Flashbang",
    "Smoke grenade",
    "HE grenade",
    "Incendiary grenade",
    "Decoy grenade",
];

#[derive(Clone)]
pub struct SideBuyLoadout {
    pistols: [Option<WeaponId>; BUY_SLOT_COUNT],
    mid_tier: [Option<WeaponId>; BUY_SLOT_COUNT],
    rifles: [Option<WeaponId>; BUY_SLOT_COUNT],
}

impl Default for SideBuyLoadout {
    fn default() -> Self {
        Self {
            pistols: [None; BUY_SLOT_COUNT],
            mid_tier: [None; BUY_SLOT_COUNT],
            rifles: [Some(WeaponId::AK47), None, None, None, None],
        }
    }
}

impl SideBuyLoadout {
    pub fn slots(&self, category: BuyCategory) -> &[Option<WeaponId>; BUY_SLOT_COUNT] {
        match category {
            BuyCategory::Pistols => &self.pistols,
            BuyCategory::MidTier => &self.mid_tier,
            BuyCategory::Rifles => &self.rifles,
        }
    }

    pub fn set(&mut self, category: BuyCategory, index: usize, weapon: Option<WeaponId>) {
        if index >= BUY_SLOT_COUNT || weapon.is_some_and(|weapon| !category.accepts(weapon)) {
            return;
        }
        let slots = match category {
            BuyCategory::Pistols => &mut self.pistols,
            BuyCategory::MidTier => &mut self.mid_tier,
            BuyCategory::Rifles => &mut self.rifles,
        };
        // A weapon occupies one purchase slot per category, even when moved.
        if weapon.is_some() {
            for slot in slots.iter_mut() {
                if *slot == weapon {
                    *slot = None;
                }
            }
        }
        slots[index] = weapon;
    }
}

#[derive(Default)]
pub struct BuyLoadouts {
    pub attacker: SideBuyLoadout,
    pub defender: SideBuyLoadout,
}

impl BuyLoadouts {
    pub fn side(&self, side: PlayerSide) -> &SideBuyLoadout {
        if side == PlayerSide::Defender {
            &self.defender
        } else {
            &self.attacker
        }
    }

    pub fn side_mut(&mut self, side: PlayerSide) -> &mut SideBuyLoadout {
        if side == PlayerSide::Defender {
            &mut self.defender
        } else {
            &mut self.attacker
        }
    }
}
