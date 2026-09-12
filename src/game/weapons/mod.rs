use crate::game::{config::WeaponId, game::SimulationSet, matchplay::ActorIntent, GameState};
use bevy::prelude::*;
pub mod audio;
pub mod combat;
pub mod viewmodel;

/// One authoritative definition shared by human/bot simulation and presentation.
#[derive(Clone, Copy)]
pub struct WeaponDefinition {
    pub magazine: u32,
    pub reserve: u32,
    pub interval: f32,
    pub reload_seconds: f32,
    pub damage: f32,
    pub range: f32,
    pub recoil: f32,
}
pub const AK47: WeaponDefinition = WeaponDefinition {
    magazine: 30,
    reserve: 90,
    interval: 0.1,
    reload_seconds: 2.5,
    damage: 36.0,
    range: 200.0,
    recoil: 0.009,
};

impl crate::game::config::WeaponId {
    pub fn stats(self) -> WeaponDefinition {
        use crate::game::config::WeaponId::*;
        match self {
            Glock18 => WeaponDefinition {
                magazine: 20,
                reserve: 120,
                interval: 0.15,
                reload_seconds: 2.2,
                damage: 26.0,
                range: 80.0,
                recoil: 0.006,
            },
            Usps => WeaponDefinition {
                magazine: 12,
                reserve: 24,
                interval: 0.17,
                reload_seconds: 2.2,
                damage: 33.0,
                range: 90.0,
                recoil: 0.007,
            },
            P2000 => WeaponDefinition {
                magazine: 13,
                reserve: 52,
                interval: 0.17,
                reload_seconds: 2.2,
                damage: 32.0,
                range: 90.0,
                recoil: 0.007,
            },
            DualBerettas => WeaponDefinition {
                magazine: 30,
                reserve: 120,
                interval: 0.12,
                reload_seconds: 3.8,
                damage: 35.0,
                range: 70.0,
                recoil: 0.008,
            },
            P250 => WeaponDefinition {
                magazine: 13,
                reserve: 26,
                interval: 0.15,
                reload_seconds: 2.2,
                damage: 35.0,
                range: 80.0,
                recoil: 0.008,
            },
            Tec9 => WeaponDefinition {
                magazine: 18,
                reserve: 90,
                interval: 0.12,
                reload_seconds: 2.5,
                damage: 30.0,
                range: 70.0,
                recoil: 0.009,
            },
            FiveSeven => WeaponDefinition {
                magazine: 20,
                reserve: 100,
                interval: 0.15,
                reload_seconds: 2.2,
                damage: 29.0,
                range: 90.0,
                recoil: 0.006,
            },
            Cz75 => WeaponDefinition {
                magazine: 12,
                reserve: 12,
                interval: 0.1,
                reload_seconds: 2.7,
                damage: 31.0,
                range: 70.0,
                recoil: 0.01,
            },
            Deagle => WeaponDefinition {
                magazine: 7,
                reserve: 35,
                interval: 0.22,
                reload_seconds: 2.2,
                damage: 53.0,
                range: 120.0,
                recoil: 0.02,
            },
            R8 => WeaponDefinition {
                magazine: 8,
                reserve: 8,
                interval: 0.4,
                reload_seconds: 2.3,
                damage: 84.0,
                range: 120.0,
                recoil: 0.025,
            },
            Mac10 => WeaponDefinition {
                magazine: 30,
                reserve: 100,
                interval: 0.075,
                reload_seconds: 2.5,
                damage: 26.0,
                range: 60.0,
                recoil: 0.008,
            },
            Mp9 => WeaponDefinition {
                magazine: 30,
                reserve: 120,
                interval: 0.07,
                reload_seconds: 2.1,
                damage: 24.0,
                range: 60.0,
                recoil: 0.007,
            },
            Mp7 => WeaponDefinition {
                magazine: 30,
                reserve: 120,
                interval: 0.08,
                reload_seconds: 3.1,
                damage: 27.0,
                range: 70.0,
                recoil: 0.007,
            },
            Mp5sd => WeaponDefinition {
                magazine: 30,
                reserve: 120,
                interval: 0.08,
                reload_seconds: 2.9,
                damage: 25.0,
                range: 70.0,
                recoil: 0.006,
            },
            Ump45 => WeaponDefinition {
                magazine: 25,
                reserve: 100,
                interval: 0.09,
                reload_seconds: 3.3,
                damage: 33.0,
                range: 70.0,
                recoil: 0.008,
            },
            P90 => WeaponDefinition {
                magazine: 50,
                reserve: 100,
                interval: 0.07,
                reload_seconds: 3.3,
                damage: 24.0,
                range: 70.0,
                recoil: 0.006,
            },
            Bizon => WeaponDefinition {
                magazine: 64,
                reserve: 120,
                interval: 0.08,
                reload_seconds: 2.4,
                damage: 25.0,
                range: 60.0,
                recoil: 0.006,
            },
            Nova => WeaponDefinition {
                magazine: 8,
                reserve: 32,
                interval: 0.88,
                reload_seconds: 3.7,
                damage: 52.0,
                range: 40.0,
                recoil: 0.018,
            },
            Xm1014 => WeaponDefinition {
                magazine: 7,
                reserve: 32,
                interval: 0.3,
                reload_seconds: 2.8,
                damage: 28.0,
                range: 40.0,
                recoil: 0.014,
            },
            SawedOff => WeaponDefinition {
                magazine: 8,
                reserve: 16,
                interval: 0.85,
                reload_seconds: 3.4,
                damage: 60.0,
                range: 25.0,
                recoil: 0.022,
            },
            M249 => WeaponDefinition {
                magazine: 100,
                reserve: 200,
                interval: 0.08,
                reload_seconds: 5.7,
                damage: 31.0,
                range: 140.0,
                recoil: 0.01,
            },
            Negev => WeaponDefinition {
                magazine: 150,
                reserve: 300,
                interval: 0.1,
                reload_seconds: 5.7,
                damage: 34.0,
                range: 140.0,
                recoil: 0.012,
            },
            Galil => WeaponDefinition {
                magazine: 35,
                reserve: 90,
                interval: 0.09,
                reload_seconds: 3.0,
                damage: 29.0,
                range: 160.0,
                recoil: 0.009,
            },
            Famas => WeaponDefinition {
                magazine: 25,
                reserve: 90,
                interval: 0.09,
                reload_seconds: 3.3,
                damage: 28.0,
                range: 160.0,
                recoil: 0.008,
            },
            AK47 => crate::game::weapons::AK47,
            M4a4 => WeaponDefinition {
                magazine: 30,
                reserve: 90,
                interval: 0.09,
                reload_seconds: 3.1,
                damage: 31.0,
                range: 180.0,
                recoil: 0.008,
            },
            M4a1s => WeaponDefinition {
                magazine: 20,
                reserve: 80,
                interval: 0.1,
                reload_seconds: 3.1,
                damage: 33.0,
                range: 180.0,
                recoil: 0.007,
            },
            Sg553 => WeaponDefinition {
                magazine: 30,
                reserve: 90,
                interval: 0.11,
                reload_seconds: 2.8,
                damage: 32.0,
                range: 190.0,
                recoil: 0.01,
            },
            Aug => WeaponDefinition {
                magazine: 30,
                reserve: 90,
                interval: 0.11,
                reload_seconds: 3.8,
                damage: 30.0,
                range: 190.0,
                recoil: 0.008,
            },
            Ssg08 => WeaponDefinition {
                magazine: 10,
                reserve: 90,
                interval: 1.25,
                reload_seconds: 3.7,
                damage: 80.0,
                range: 250.0,
                recoil: 0.016,
            },
            Awp => WeaponDefinition {
                magazine: 10,
                reserve: 30,
                interval: 1.5,
                reload_seconds: 3.7,
                damage: 115.0,
                range: 300.0,
                recoil: 0.02,
            },
            G3sg1 => WeaponDefinition {
                magazine: 20,
                reserve: 90,
                interval: 0.25,
                reload_seconds: 4.7,
                damage: 79.0,
                range: 250.0,
                recoil: 0.014,
            },
            Scar20 => WeaponDefinition {
                magazine: 20,
                reserve: 90,
                interval: 0.25,
                reload_seconds: 3.1,
                damage: 79.0,
                range: 250.0,
                recoil: 0.014,
            },
            _ => crate::game::weapons::AK47,
        }
    }
}
#[derive(Clone, Copy)]
pub enum WeaponSelection {
    Select(WeaponId),
    Previous,
}
pub struct KnifeDefinition {
    pub range: f32,
    pub damage: f32,
    pub windup: f32,
    pub recovery: f32,
    pub equip_seconds: f32,
}
pub const KNIFE: KnifeDefinition = KnifeDefinition {
    range: 1.5,
    damage: 40.0,
    windup: 0.15,
    recovery: 0.40,
    equip_seconds: 0.35,
};
#[derive(Component)]
pub struct WeaponState {
    pub active: WeaponId,
    pub gun: WeaponId,
    pub previous: WeaponId,
    pub melee_weapon: WeaponId,
    pub equip_remaining: f32,
    pub knife_remaining: f32,
    pub slashes: u64,
    pub equips: u64,
    pub reload_cancellations: u64,
    pub magazine: u32,
    pub reserve: u32,
    pub cooldown: f32,
    pub reload_remaining: f32,
    pub flash_remaining: f32,
    pub shots: u64,
    pub muzzle_blocked: bool,
}
impl Default for WeaponState {
    fn default() -> Self {
        Self::armed(WeaponId::AK47, WeaponId::DefaultKnife)
    }
}

impl WeaponState {
    pub fn armed(gun: WeaponId, melee: WeaponId) -> Self {
        let stats = gun.stats();
        Self {
            active: gun,
            gun,
            previous: melee,
            melee_weapon: melee,
            equip_remaining: 0.0,
            knife_remaining: 0.0,
            slashes: 0,
            equips: 0,
            reload_cancellations: 0,
            magazine: stats.magazine,
            reserve: stats.reserve,
            cooldown: 0.0,
            reload_remaining: 0.0,
            flash_remaining: 0.0,
            shots: 0,
            muzzle_blocked: false,
        }
    }
}
#[derive(Event)]
pub struct ShotFired {
    pub actor: Entity,
    pub origin: Vec3,
    pub end: Vec3,
}
/// Clear transient input at the pause boundary without changing weapon timers.
fn clear_intents(mut actors: Query<&mut ActorIntent>) {
    for mut intent in &mut actors {
        *intent = ActorIntent::default();
    }
}
pub struct WeaponPlugin;
impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(audio::WeaponAudioPlugin)
            .add_event::<ShotFired>()
            .add_systems(OnEnter(GameState::Paused), clear_intents)
            .add_systems(OnEnter(GameState::Finished), clear_intents)
            .add_systems(
                FixedUpdate,
                combat::simulate_weapons
                    .in_set(SimulationSet::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    viewmodel::bind_scenes,
                    viewmodel::bind_muzzles,
                    viewmodel::animate_viewmodel.run_if(in_state(GameState::Playing)),
                    viewmodel::animate_flashes,
                    viewmodel::frame_camera,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                combat::shot_effects.run_if(in_state(GameState::Playing)),
            );
    }
}
