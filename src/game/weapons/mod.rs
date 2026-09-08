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
    reserve: 120,
    interval: 0.1,
    reload_seconds: 2.5,
    damage: 34.0,
    range: 200.0,
    recoil: 0.009,
};
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
    pub previous: WeaponId,
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
        Self {
            active: WeaponId::AK47,
            previous: WeaponId::DefaultKnife,
            equip_remaining: 0.0,
            knife_remaining: 0.0,
            slashes: 0,
            equips: 0,
            reload_cancellations: 0,
            magazine: AK47.magazine,
            reserve: AK47.reserve,
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
                )
                    .chain(),
            )
            .add_systems(
                Update,
                combat::shot_effects.run_if(in_state(GameState::Playing)),
            );
    }
}
