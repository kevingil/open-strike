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

/// Shared by all bots. Local matches read this from player settings.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotDifficulty {
    #[default]
    Easy,
    Normal,
    Hard,
}

impl BotDifficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Normal, Self::Hard];

    pub fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Normal => "Normal",
            Self::Hard => "Hard",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "easy" => Some(Self::Easy),
            "normal" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
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
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MapId {
    Warehouse,
    #[default]
    Dust2,
    Mirage,
}

impl MapId {
    pub const PLAYABLE: [Self; 2] = [Self::Dust2, Self::Mirage];

    pub fn name(&self) -> &'static str {
        match self {
            MapId::Warehouse => "Warehouse",
            MapId::Dust2 => "Dust 2",
            MapId::Mirage => "Mirage",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            MapId::Warehouse => "warehouse",
            MapId::Dust2 => "dust2",
            MapId::Mirage => "mirage",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "warehouse" => Some(MapId::Warehouse),
            "dust2" => Some(MapId::Dust2),
            "mirage" => Some(MapId::Mirage),
            _ => None,
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            MapId::Warehouse => 0,
            MapId::Dust2 => 1,
            MapId::Mirage => 2,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            0 => MapId::Warehouse,
            2 => MapId::Mirage,
            _ => MapId::Dust2,
        }
    }

    /// Get the config file path for this map
    pub fn config_path(&self) -> &'static str {
        match self {
            MapId::Warehouse => "maps/warehouse/config.map.ron",
            MapId::Dust2 => "maps/de_dust_2/config.map.ron",
            MapId::Mirage => "maps/de_mirage/config.map.ron",
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
#[derive(Resource, Clone)]
pub struct PlayerSettings {
    pub sensitivity: f32,
    pub fov: f32,
    pub master_volume: f32,
    pub bot_difficulty: BotDifficulty,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            fov: 103.0,
            master_volume: 1.0,
            bot_difficulty: BotDifficulty::default(),
        }
    }
}

/// Available weapons for loadout
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum WeaponId {
    Glock18,
    Usps,
    P2000,
    DualBerettas,
    P250,
    Tec9,
    FiveSeven,
    Cz75,
    Deagle,
    R8,
    Mac10,
    Mp9,
    Mp7,
    Mp5sd,
    Ump45,
    P90,
    Bizon,
    Nova,
    Xm1014,
    SawedOff,
    M249,
    Negev,
    Galil,
    Famas,
    AK47,
    M4a4,
    M4a1s,
    Sg553,
    Aug,
    Ssg08,
    Awp,
    G3sg1,
    Scar20,
    DefaultKnife,
    DefaultKnifeStained,
    DefaultKnifeForest,
    DefaultKnifeCt,
    DefaultKnifeT,
    ReferenceKnife,
    Karambit,
}

impl WeaponId {
    pub const ALL: [Self; 40] = [
        Self::Glock18,
        Self::Usps,
        Self::P2000,
        Self::DualBerettas,
        Self::P250,
        Self::Tec9,
        Self::FiveSeven,
        Self::Cz75,
        Self::Deagle,
        Self::R8,
        Self::Mac10,
        Self::Mp9,
        Self::Mp7,
        Self::Mp5sd,
        Self::Ump45,
        Self::P90,
        Self::Bizon,
        Self::Nova,
        Self::Xm1014,
        Self::SawedOff,
        Self::M249,
        Self::Negev,
        Self::Galil,
        Self::Famas,
        Self::AK47,
        Self::M4a4,
        Self::M4a1s,
        Self::Sg553,
        Self::Aug,
        Self::Ssg08,
        Self::Awp,
        Self::G3sg1,
        Self::Scar20,
        Self::DefaultKnife,
        Self::DefaultKnifeStained,
        Self::DefaultKnifeForest,
        Self::DefaultKnifeCt,
        Self::DefaultKnifeT,
        Self::ReferenceKnife,
        Self::Karambit,
    ];

    pub fn is_knife(self) -> bool {
        matches!(
            self,
            Self::DefaultKnife
                | Self::DefaultKnifeStained
                | Self::DefaultKnifeForest
                | Self::DefaultKnifeCt
                | Self::DefaultKnifeT
                | Self::ReferenceKnife
                | Self::Karambit
        )
    }

    pub fn has_viewmodel(self) -> bool {
        matches!(self, Self::AK47 | Self::DefaultKnife | Self::ReferenceKnife)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Glock18 => "Glock-18",
            Self::Usps => "USP-S",
            Self::P2000 => "P2000",
            Self::DualBerettas => "Dual Berettas",
            Self::P250 => "P250",
            Self::Tec9 => "Tec-9",
            Self::FiveSeven => "Five-SeveN",
            Self::Cz75 => "CZ75-Auto",
            Self::Deagle => "Desert Eagle",
            Self::R8 => "R8 Revolver",
            Self::Mac10 => "MAC-10",
            Self::Mp9 => "MP9",
            Self::Mp7 => "MP7",
            Self::Mp5sd => "MP5-SD",
            Self::Ump45 => "UMP-45",
            Self::P90 => "P90",
            Self::Bizon => "PP-Bizon",
            Self::Nova => "Nova",
            Self::Xm1014 => "XM1014",
            Self::SawedOff => "Sawed-Off",
            Self::M249 => "M249",
            Self::Negev => "Negev",
            Self::Galil => "Galil AR",
            Self::Famas => "FAMAS",
            Self::AK47 => "AK-47",
            Self::M4a4 => "M4A4",
            Self::M4a1s => "M4A1-S",
            Self::Sg553 => "SG 553",
            Self::Aug => "AUG",
            Self::Ssg08 => "SSG 08",
            Self::Awp => "AWP",
            Self::G3sg1 => "G3SG1",
            Self::Scar20 => "SCAR-20",
            Self::DefaultKnife => "Default Knife",
            Self::DefaultKnifeStained => "Default Knife · Stained",
            Self::DefaultKnifeForest => "Default Knife · Forest",
            Self::DefaultKnifeCt => "Default Knife · CT",
            Self::DefaultKnifeT => "Default Knife · T",
            Self::ReferenceKnife => "Reference Knife",
            Self::Karambit => "Karambit",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Glock18 => "glock18",
            Self::Usps => "usps",
            Self::P2000 => "p2000",
            Self::DualBerettas => "dual_berettas",
            Self::P250 => "p250",
            Self::Tec9 => "tec9",
            Self::FiveSeven => "fiveseven",
            Self::Cz75 => "cz75",
            Self::Deagle => "deagle",
            Self::R8 => "r8",
            Self::Mac10 => "mac10",
            Self::Mp9 => "mp9",
            Self::Mp7 => "mp7",
            Self::Mp5sd => "mp5sd",
            Self::Ump45 => "ump45",
            Self::P90 => "p90",
            Self::Bizon => "bizon",
            Self::Nova => "nova",
            Self::Xm1014 => "xm1014",
            Self::SawedOff => "sawedoff",
            Self::M249 => "m249",
            Self::Negev => "negev",
            Self::Galil => "galil",
            Self::Famas => "famas",
            Self::AK47 => "ak47",
            Self::M4a4 => "m4a4",
            Self::M4a1s => "m4a1s",
            Self::Sg553 => "sg553",
            Self::Aug => "aug",
            Self::Ssg08 => "ssg08",
            Self::Awp => "awp",
            Self::G3sg1 => "g3sg1",
            Self::Scar20 => "scar20",
            Self::DefaultKnife => "knife",
            Self::DefaultKnifeStained => "knife_stained",
            Self::DefaultKnifeForest => "knife_forest",
            Self::DefaultKnifeCt => "knife_ct",
            Self::DefaultKnifeT => "knife_t",
            Self::ReferenceKnife => "reference_knife",
            Self::Karambit => "karambit",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|weapon| weapon.key() == key)
    }

    pub fn all() -> Vec<WeaponId> {
        Self::ALL.to_vec()
    }

    pub fn knives() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(|weapon| weapon.is_knife())
    }

    pub fn guns() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(|weapon| !weapon.is_knife())
    }

    pub fn buy_category(self) -> Option<BuyCategory> {
        match self {
            Self::Glock18
            | Self::Usps
            | Self::P2000
            | Self::DualBerettas
            | Self::P250
            | Self::Tec9
            | Self::FiveSeven
            | Self::Cz75
            | Self::Deagle
            | Self::R8 => Some(BuyCategory::Pistols),
            Self::Mac10
            | Self::Mp9
            | Self::Mp7
            | Self::Mp5sd
            | Self::Ump45
            | Self::P90
            | Self::Bizon
            | Self::Nova
            | Self::Xm1014
            | Self::SawedOff
            | Self::M249
            | Self::Negev => Some(BuyCategory::MidTier),
            Self::Galil
            | Self::Famas
            | Self::AK47
            | Self::M4a4
            | Self::M4a1s
            | Self::Sg553
            | Self::Aug
            | Self::Ssg08
            | Self::Awp
            | Self::G3sg1
            | Self::Scar20 => Some(BuyCategory::Rifles),
            _ => None,
        }
    }

    pub fn side(self) -> PlayerSide {
        match self {
            Self::Glock18
            | Self::Tec9
            | Self::Mac10
            | Self::SawedOff
            | Self::Galil
            | Self::AK47
            | Self::Sg553
            | Self::G3sg1 => PlayerSide::Attacker,
            Self::Usps
            | Self::P2000
            | Self::FiveSeven
            | Self::Mp9
            | Self::Famas
            | Self::M4a4
            | Self::M4a1s
            | Self::Aug
            | Self::Scar20 => PlayerSide::Defender,
            _ => PlayerSide::Any,
        }
    }

    pub fn available_on(self, side: PlayerSide) -> bool {
        matches!(self.side(), PlayerSide::Any) || self.side() == side
    }

    pub fn inventory_path(self) -> &'static str {
        match self {
            Self::AK47 => "generated/ui/inventory/ak47.png",
            Self::DefaultKnife => "generated/ui/inventory/knife.png",
            Self::ReferenceKnife => "generated/ui/inventory/reference_knife.png",
            _ => self.generated_icon(),
        }
    }

    fn generated_icon(self) -> &'static str {
        match self {
            Self::Glock18 => "generated/ui/inventory/glock18.png",
            Self::Usps => "generated/ui/inventory/usps.png",
            Self::P2000 => "generated/ui/inventory/p2000.png",
            Self::DualBerettas => "generated/ui/inventory/dual_berettas.png",
            Self::P250 => "generated/ui/inventory/p250.png",
            Self::Tec9 => "generated/ui/inventory/tec9.png",
            Self::FiveSeven => "generated/ui/inventory/fiveseven.png",
            Self::Cz75 => "generated/ui/inventory/cz75.png",
            Self::Deagle => "generated/ui/inventory/deagle.png",
            Self::R8 => "generated/ui/inventory/r8.png",
            Self::Mac10 => "generated/ui/inventory/mac10.png",
            Self::Mp9 => "generated/ui/inventory/mp9.png",
            Self::Mp7 => "generated/ui/inventory/mp7.png",
            Self::Mp5sd => "generated/ui/inventory/mp5sd.png",
            Self::Ump45 => "generated/ui/inventory/ump45.png",
            Self::P90 => "generated/ui/inventory/p90.png",
            Self::Bizon => "generated/ui/inventory/bizon.png",
            Self::Nova => "generated/ui/inventory/nova.png",
            Self::Xm1014 => "generated/ui/inventory/xm1014.png",
            Self::SawedOff => "generated/ui/inventory/sawedoff.png",
            Self::M249 => "generated/ui/inventory/m249.png",
            Self::Negev => "generated/ui/inventory/negev.png",
            Self::Galil => "generated/ui/inventory/galil.png",
            Self::Famas => "generated/ui/inventory/famas.png",
            Self::M4a4 => "generated/ui/inventory/m4a4.png",
            Self::M4a1s => "generated/ui/inventory/m4a1s.png",
            Self::Sg553 => "generated/ui/inventory/sg553.png",
            Self::Aug => "generated/ui/inventory/aug.png",
            Self::Ssg08 => "generated/ui/inventory/ssg08.png",
            Self::Awp => "generated/ui/inventory/awp.png",
            Self::G3sg1 => "generated/ui/inventory/g3sg1.png",
            Self::Scar20 => "generated/ui/inventory/scar20.png",
            Self::DefaultKnifeStained => "generated/ui/inventory/knife_stained.png",
            Self::DefaultKnifeForest => "generated/ui/inventory/knife_forest.png",
            Self::DefaultKnifeCt => "generated/ui/inventory/knife_ct.png",
            Self::DefaultKnifeT => "generated/ui/inventory/knife_t.png",
            Self::Karambit => "generated/ui/inventory/karambit.png",
            Self::AK47 | Self::DefaultKnife | Self::ReferenceKnife => unreachable!(),
        }
    }

    pub fn world_path(self) -> &'static str {
        match self {
            Self::AK47 => "generated/ak_world.glb",
            Self::DefaultKnife => "generated/knife_world.glb",
            Self::ReferenceKnife => "generated/reference_knife_world.glb",
            _ => self.generated_world(),
        }
    }

    fn generated_world(self) -> &'static str {
        match self {
            Self::Glock18 => "generated/weapons/glock18.glb",
            Self::Usps => "generated/weapons/usps.glb",
            Self::P2000 => "generated/weapons/p2000.glb",
            Self::DualBerettas => "generated/weapons/dual_berettas.glb",
            Self::P250 => "generated/weapons/p250.glb",
            Self::Tec9 => "generated/weapons/tec9.glb",
            Self::FiveSeven => "generated/weapons/fiveseven.glb",
            Self::Cz75 => "generated/weapons/cz75.glb",
            Self::Deagle => "generated/weapons/deagle.glb",
            Self::R8 => "generated/weapons/r8.glb",
            Self::Mac10 => "generated/weapons/mac10.glb",
            Self::Mp9 => "generated/weapons/mp9.glb",
            Self::Mp7 => "generated/weapons/mp7.glb",
            Self::Mp5sd => "generated/weapons/mp5sd.glb",
            Self::Ump45 => "generated/weapons/ump45.glb",
            Self::P90 => "generated/weapons/p90.glb",
            Self::Bizon => "generated/weapons/bizon.glb",
            Self::Nova => "generated/weapons/nova.glb",
            Self::Xm1014 => "generated/weapons/xm1014.glb",
            Self::SawedOff => "generated/weapons/sawedoff.glb",
            Self::M249 => "generated/weapons/m249.glb",
            Self::Negev => "generated/weapons/negev.glb",
            Self::Galil => "generated/weapons/galil.glb",
            Self::Famas => "generated/weapons/famas.glb",
            Self::M4a4 => "generated/weapons/m4a4.glb",
            Self::M4a1s => "generated/weapons/m4a1s.glb",
            Self::Sg553 => "generated/weapons/sg553.glb",
            Self::Aug => "generated/weapons/aug.glb",
            Self::Ssg08 => "generated/weapons/ssg08.glb",
            Self::Awp => "generated/weapons/awp.glb",
            Self::G3sg1 => "generated/weapons/g3sg1.glb",
            Self::Scar20 => "generated/weapons/scar20.glb",
            Self::DefaultKnifeStained => "generated/weapons/knife_stained.glb",
            Self::DefaultKnifeForest => "generated/weapons/knife_forest.glb",
            Self::DefaultKnifeCt => "generated/weapons/knife_ct.glb",
            Self::DefaultKnifeT => "generated/weapons/knife_t.glb",
            Self::Karambit => "generated/weapons/karambit.glb",
            Self::AK47 | Self::DefaultKnife | Self::ReferenceKnife => unreachable!(),
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::DefaultKnife => "Melee · Default finish",
            Self::DefaultKnifeStained => "Melee · Stained finish",
            Self::DefaultKnifeForest => "Melee · Forest finish",
            Self::DefaultKnifeCt => "Melee · CT",
            Self::DefaultKnifeT => "Melee · T",
            Self::ReferenceKnife => "Melee · Reference finish",
            Self::Karambit => "Melee · Karambit",
            other => match other.buy_category() {
                Some(BuyCategory::Pistols) => "Pistol",
                Some(BuyCategory::MidTier) => "Mid-tier",
                Some(BuyCategory::Rifles) => "Rifle",
                None => "Weapon",
            },
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::AK47 => 0,
            Self::DefaultKnife => 1,
            Self::ReferenceKnife => 2,
            other => {
                3 + Self::ALL
                    .iter()
                    .filter(|weapon| {
                        !matches!(
                            weapon,
                            Self::AK47 | Self::DefaultKnife | Self::ReferenceKnife
                        )
                    })
                    .position(|weapon| *weapon == other)
                    .unwrap() as u8
            }
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::AK47,
            1 => Self::DefaultKnife,
            2 => Self::ReferenceKnife,
            other => Self::ALL
                .iter()
                .copied()
                .filter(|weapon| {
                    !matches!(
                        weapon,
                        Self::AK47 | Self::DefaultKnife | Self::ReferenceKnife
                    )
                })
                .nth((other - 3) as usize)
                .unwrap_or(Self::AK47),
        }
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

impl PlayerLoadout {
    pub fn spawn_gun(&self, side: PlayerSide) -> WeaponId {
        self.buy_weapons
            .side(side)
            .first_gun()
            .filter(|weapon| weapon.available_on(side))
            .unwrap_or(self.primary_weapon)
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
        weapon.buy_category() == Some(self)
    }

    pub fn accepts_for(self, weapon: WeaponId, side: PlayerSide) -> bool {
        self.accepts(weapon) && weapon.available_on(side)
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
        Self::for_side(PlayerSide::Attacker)
    }
}

impl SideBuyLoadout {
    pub fn for_side(side: PlayerSide) -> Self {
        use WeaponId::*;

        if side == PlayerSide::Defender {
            Self {
                pistols: [Usps, DualBerettas, P250, FiveSeven, Deagle].map(Some),
                mid_tier: [Mp9, Mp7, Ump45, P90, Xm1014].map(Some),
                rifles: [M4a4, M4a1s, Famas, Ssg08, Awp].map(Some),
            }
        } else {
            Self {
                pistols: [Glock18, DualBerettas, P250, Tec9, Deagle].map(Some),
                mid_tier: [Mac10, Mp7, Ump45, P90, Xm1014].map(Some),
                rifles: [AK47, Galil, Sg553, Ssg08, Awp].map(Some),
            }
        }
    }

    pub fn empty() -> Self {
        Self {
            pistols: [None; BUY_SLOT_COUNT],
            mid_tier: [None; BUY_SLOT_COUNT],
            rifles: [None; BUY_SLOT_COUNT],
        }
    }

    pub fn slots(&self, category: BuyCategory) -> &[Option<WeaponId>; BUY_SLOT_COUNT] {
        match category {
            BuyCategory::Pistols => &self.pistols,
            BuyCategory::MidTier => &self.mid_tier,
            BuyCategory::Rifles => &self.rifles,
        }
    }

    pub fn first_gun(&self) -> Option<WeaponId> {
        for category in [BuyCategory::Rifles, BuyCategory::MidTier, BuyCategory::Pistols] {
            if let Some(weapon) = self.slots(category).iter().flatten().next() {
                return Some(*weapon);
            }
        }
        None
    }

    pub fn set(
        &mut self,
        category: BuyCategory,
        index: usize,
        weapon: Option<WeaponId>,
        side: PlayerSide,
    ) {
        if index >= BUY_SLOT_COUNT
            || weapon.is_some_and(|weapon| !category.accepts_for(weapon, side))
        {
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

pub struct BuyLoadouts {
    pub attacker: SideBuyLoadout,
    pub defender: SideBuyLoadout,
}

impl Default for BuyLoadouts {
    fn default() -> Self {
        Self {
            attacker: SideBuyLoadout::for_side(PlayerSide::Attacker),
            defender: SideBuyLoadout::for_side(PlayerSide::Defender),
        }
    }
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
