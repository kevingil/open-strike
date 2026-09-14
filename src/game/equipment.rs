//! Fixed inventory equipment, separate from held weapons and character cosmetics.
//! Armor previews never become attachments on a gameplay character.
use crate::game::player::skins::PlayerSide;
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentCategory {
    Grenade,
    Equipment,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSide {
    Both,
    Attacker,
    Defender,
}

#[derive(Deserialize)]
pub struct EquipmentItem {
    pub key: String,
    pub name: String,
    pub category: EquipmentCategory,
    pub side: EquipmentSide,
    pub icon: String,
    /// Standalone asset for inventory presentation, never a character attachment.
    pub model: String,
}

impl EquipmentItem {
    pub fn available_on(&self, side: PlayerSide) -> bool {
        match self.side {
            EquipmentSide::Both => true,
            EquipmentSide::Attacker => side == PlayerSide::Attacker,
            EquipmentSide::Defender => side == PlayerSide::Defender,
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self.side {
            EquipmentSide::Attacker => "Equipment · Attacker",
            EquipmentSide::Defender => "Equipment · Defender",
            EquipmentSide::Both => match self.category {
                EquipmentCategory::Grenade => "Grenade · Both teams",
                EquipmentCategory::Equipment => "Equipment · Both teams",
            },
        }
    }

    pub fn status(&self) -> &'static str {
        match self.key.as_str() {
            "kevlar" | "kevlar_helmet" => "Invisible on character",
            _ => "Fixed selection",
        }
    }
}

#[derive(Deserialize)]
struct EquipmentCatalog {
    items: Vec<EquipmentItem>,
}

pub fn items() -> &'static [EquipmentItem] {
    static CATALOG: LazyLock<EquipmentCatalog> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../assets/config/equipment.json"))
            .expect("Bundled equipment catalog must be valid")
    });
    &CATALOG.items
}
