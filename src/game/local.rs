//! Client SQLite store: hub session, settings, and loadout.
use crate::game::{
    config::{
        BotDifficulty, BuyCategory, GameConfig, PlayerLoadout, PlayerSettings, SideBuyLoadout,
        WeaponId, BUY_SLOT_COUNT,
    },
    player::skins::{PlayerSide, SkinId},
};
use bevy::prelude::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub const KEY_HUB: &str = "hub";
const KEY_SETTINGS: &str = "settings";
const KEY_LOADOUT: &str = "loadout";
const BUY_DEFAULTS_VERSION: u32 = 1;

#[derive(Clone, Resource)]
pub struct LocalDb {
    conn: Arc<Mutex<Connection>>,
}

impl LocalDb {
    fn open() -> Self {
        Self::open_path(&db_path())
    }

    fn open_path(path: &std::path::Path) -> Self {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path).unwrap_or_else(|error| {
            warn!("local db open failed ({error}), using memory");
            Connection::open_in_memory().expect("open memory db")
        });
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .expect("local db migrate");
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.import_hub_json();
        db
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let db = self.conn.lock().unwrap();
        db.query_row("SELECT value FROM kv WHERE key = ?1", params![key], |row| {
            row.get(0)
        })
        .optional()
        .ok()
        .flatten()
    }

    pub fn set(&self, key: &str, value: &str) {
        let db = self.conn.lock().unwrap();
        if let Err(error) = db.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ) {
            warn!("local db write {key} failed: {error}");
        }
    }

    fn import_hub_json(&self) {
        if self.get(KEY_HUB).is_some() {
            return;
        }
        let Ok(text) = std::fs::read_to_string(hub_json_path()) else {
            return;
        };
        if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
            self.set(KEY_HUB, &text);
        }
    }
}

pub fn hub_json_path() -> PathBuf {
    if let Some(path) = std::env::var_os("STRIKE_HUB_CONFIG") {
        return path.into();
    }
    config_dir().join("hub.json")
}

fn db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("STRIKE_LOCAL_DB") {
        return path.into();
    }
    config_dir().join("local.db")
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| ".".into())
        .join("open-strike")
}

#[derive(Serialize, Deserialize)]
struct StoredSettings {
    #[serde(default = "default_sensitivity")]
    sensitivity: f32,
    #[serde(default = "default_fov")]
    fov: f32,
    #[serde(default = "default_volume")]
    master_volume: f32,
    #[serde(default)]
    bot_difficulty: String,
}

fn default_sensitivity() -> f32 {
    PlayerSettings::default().sensitivity
}
fn default_fov() -> f32 {
    PlayerSettings::default().fov
}
fn default_volume() -> f32 {
    PlayerSettings::default().master_volume
}

impl StoredSettings {
    fn from_live(settings: &PlayerSettings) -> Self {
        Self {
            sensitivity: settings.sensitivity,
            fov: settings.fov,
            master_volume: settings.master_volume,
            bot_difficulty: settings.bot_difficulty.key().into(),
        }
    }

    fn apply(self, settings: &mut PlayerSettings) {
        settings.sensitivity = self.sensitivity;
        settings.fov = self.fov;
        settings.master_volume = self.master_volume;
        settings.bot_difficulty = BotDifficulty::from_key(&self.bot_difficulty).unwrap_or_default();
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
struct StoredBuySide {
    #[serde(default)]
    pistols: Vec<Option<String>>,
    #[serde(default)]
    mid_tier: Vec<Option<String>>,
    #[serde(default)]
    rifles: Vec<Option<String>>,
}

#[derive(Serialize, Deserialize, Default)]
struct StoredBuy {
    #[serde(default)]
    attacker: StoredBuySide,
    #[serde(default)]
    defender: StoredBuySide,
}

#[derive(Serialize, Deserialize)]
struct StoredLoadout {
    #[serde(default)]
    buy_defaults_version: u32,
    #[serde(default)]
    primary_weapon: String,
    #[serde(default)]
    melee_weapon: String,
    #[serde(default)]
    selected_skin: String,
    #[serde(default)]
    buy_weapons: StoredBuy,
}

impl StoredLoadout {
    fn from_live(loadout: &PlayerLoadout) -> Self {
        Self {
            buy_defaults_version: BUY_DEFAULTS_VERSION,
            primary_weapon: loadout.primary_weapon.key().into(),
            melee_weapon: loadout.melee_weapon.key().into(),
            selected_skin: loadout.selected_skin.key().into(),
            buy_weapons: StoredBuy {
                attacker: encode_side(loadout.buy_weapons.side(PlayerSide::Attacker)),
                defender: encode_side(loadout.buy_weapons.side(PlayerSide::Defender)),
            },
        }
    }

    fn apply(self, loadout: &mut PlayerLoadout) {
        if let Some(weapon) = WeaponId::from_key(&self.primary_weapon) {
            loadout.primary_weapon = weapon;
        }
        if let Some(weapon) = WeaponId::from_key(&self.melee_weapon) {
            loadout.melee_weapon = weapon;
        }
        if let Some(skin) = SkinId::from_key(&self.selected_skin) {
            loadout.selected_skin = skin;
        }
        let upgrade_defaults = self.buy_defaults_version < BUY_DEFAULTS_VERSION;
        *loadout.buy_weapons.side_mut(PlayerSide::Attacker) = decode_side(
            &self.buy_weapons.attacker,
            PlayerSide::Attacker,
            upgrade_defaults,
        );
        *loadout.buy_weapons.side_mut(PlayerSide::Defender) = decode_side(
            &self.buy_weapons.defender,
            PlayerSide::Defender,
            upgrade_defaults,
        );
    }
}

fn encode_side(side: &SideBuyLoadout) -> StoredBuySide {
    StoredBuySide {
        pistols: encode_slots(side.slots(BuyCategory::Pistols)),
        mid_tier: encode_slots(side.slots(BuyCategory::MidTier)),
        rifles: encode_slots(side.slots(BuyCategory::Rifles)),
    }
}

fn encode_slots(slots: &[Option<WeaponId>; BUY_SLOT_COUNT]) -> Vec<Option<String>> {
    slots
        .iter()
        .map(|weapon| weapon.map(|weapon| weapon.key().to_string()))
        .collect()
}

fn decode_side(
    stored: &StoredBuySide,
    player_side: PlayerSide,
    upgrade_defaults: bool,
) -> SideBuyLoadout {
    // Older saves without buy slots inherit the current preset. Explicitly
    // cleared slots are serialized as five nulls and remain empty.
    if stored.pistols.is_empty() && stored.mid_tier.is_empty() && stored.rifles.is_empty() {
        return SideBuyLoadout::for_side(player_side);
    }
    let mut side = SideBuyLoadout::empty();
    apply_slots(&mut side, BuyCategory::Pistols, &stored.pistols, player_side);
    apply_slots(&mut side, BuyCategory::MidTier, &stored.mid_tier, player_side);
    apply_slots(&mut side, BuyCategory::Rifles, &stored.rifles, player_side);
    if upgrade_defaults {
        let mut legacy = SideBuyLoadout::empty();
        let (pistol, rifle) = if player_side == PlayerSide::Defender {
            (WeaponId::Usps, WeaponId::M4a4)
        } else {
            (WeaponId::Glock18, WeaponId::AK47)
        };
        legacy.set(BuyCategory::Pistols, 0, Some(pistol), player_side);
        legacy.set(BuyCategory::Rifles, 0, Some(rifle), player_side);
        // Upgrade only the exact former preset, independently for each team.
        // Versioned saves retain even a deliberately recreated sparse preset.
        if *stored == encode_side(&legacy) {
            return SideBuyLoadout::for_side(player_side);
        }
    }
    side
}

fn apply_slots(
    side: &mut SideBuyLoadout,
    category: BuyCategory,
    keys: &[Option<String>],
    player_side: PlayerSide,
) {
    for (index, key) in keys.iter().take(BUY_SLOT_COUNT).enumerate() {
        side.set(
            category,
            index,
            key.as_deref().and_then(WeaponId::from_key),
            player_side,
        );
    }
}

pub struct LocalStorePlugin;

impl Plugin for LocalStorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load)
            .add_systems(Update, (sync_difficulty, persist).chain());
    }
}

fn load(world: &mut World) {
    let db = LocalDb::open();
    if let Some(stored) = db
        .get(KEY_SETTINGS)
        .and_then(|text| serde_json::from_str::<StoredSettings>(&text).ok())
    {
        if let Some(mut settings) = world.get_resource_mut::<PlayerSettings>() {
            stored.apply(&mut settings);
        }
        let difficulty = world.resource::<PlayerSettings>().bot_difficulty;
        if let Some(mut config) = world.get_resource_mut::<GameConfig>() {
            config.bot_difficulty = difficulty;
        }
    }
    if let Some(stored) = db
        .get(KEY_LOADOUT)
        .and_then(|text| serde_json::from_str::<StoredLoadout>(&text).ok())
    {
        if let Some(mut loadout) = world.get_resource_mut::<PlayerLoadout>() {
            stored.apply(&mut loadout);
        }
    }
    world.insert_resource(db);
}

fn sync_difficulty(settings: Res<PlayerSettings>, mut config: ResMut<GameConfig>) {
    if config.bot_difficulty != settings.bot_difficulty {
        config.bot_difficulty = settings.bot_difficulty;
    }
}

fn persist(db: Res<LocalDb>, settings: Res<PlayerSettings>, loadout: Res<PlayerLoadout>) {
    if settings.is_changed() {
        if let Ok(text) = serde_json::to_string(&StoredSettings::from_live(&settings)) {
            db.set(KEY_SETTINGS, &text);
        }
    }
    if loadout.is_changed() {
        if let Ok(text) = serde_json::to_string(&StoredLoadout::from_live(&loadout)) {
            db.set(KEY_LOADOUT, &text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "open-strike-local-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        let db = LocalDb::open_path(&path);
        db.set(KEY_SETTINGS, r#"{"fov":90.0}"#);
        assert_eq!(db.get(KEY_SETTINGS).as_deref(), Some(r#"{"fov":90.0}"#));
        let _ = std::fs::remove_file(&path);
    }
}
