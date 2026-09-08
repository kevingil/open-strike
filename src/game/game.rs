use super::{
    config::{GameConfig, PlayerLoadout, PlayerSettings},
    level::level,
    player::player,
    ui::ui,
    window::window,
};
use crate::game::{map::MapSystemPlugin, player::skins::SkinRegistry};
use bevy::prelude::*;
use bevy_fps_controller::controller::{
    fps_controller_look, fps_controller_move, fps_controller_render,
};
use bevy_rapier3d::prelude::*;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    Loading,
    LoadFailed,
    Playing,
    Paused,
    Finished,
}

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SimulationSet {
    Intent,
    Movement,
    Combat,
    Match,
}

pub struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Startup, crate::game::assets::load_assets)
            .init_resource::<GameConfig>()
            .init_resource::<PlayerLoadout>()
            .init_resource::<PlayerSettings>()
            .init_resource::<SkinRegistry>()
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .insert_resource(TimestepMode::Fixed {
                dt: 1.0 / 60.0,
                substeps: 1,
            })
            .add_plugins((
                RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule(),
                RapierDebugRenderPlugin::default().disabled(),
                crate::game::debug::DebugPlugin,
                MapSystemPlugin,
                level::LevelPlugin,
                player::PlayerPlugin,
                crate::game::matchplay::MatchPlugin,
                crate::game::weapons::WeaponPlugin,
                crate::game::bots::BotPlugin,
                window::WindowSettingsPlugin,
                ui::UiPlugin,
            ))
            .configure_sets(
                FixedUpdate,
                (
                    SimulationSet::Intent,
                    SimulationSet::Movement,
                    PhysicsSet::SyncBackend,
                    PhysicsSet::StepSimulation,
                    PhysicsSet::Writeback,
                    SimulationSet::Combat,
                    SimulationSet::Match,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    crate::game::player::movement::guard_standing,
                    fps_controller_look,
                    fps_controller_move,
                    crate::game::player::movement::block_actor_motion,
                )
                    .chain()
                    .in_set(SimulationSet::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                crate::game::player::presentation::remember
                    .before(SimulationSet::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                (
                    fps_controller_render,
                    crate::game::player::presentation::cameras,
                )
                    .chain()
                    .before(TransformSystem::TransformPropagate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, pause_physics)
            .add_systems(OnEnter(GameState::Paused), pause_physics)
            .add_systems(OnEnter(GameState::Finished), pause_physics)
            .add_systems(OnEnter(GameState::Playing), pause_physics);
    }
}

fn pause_physics(
    state: Res<State<GameState>>,
    mut configs: Query<&mut RapierConfiguration>,
    mut animations: Query<&mut AnimationPlayer>,
) {
    let paused = matches!(state.get(), GameState::Paused | GameState::Finished);
    for mut config in &mut configs {
        config.physics_pipeline_active = !paused;
    }
    for mut player in &mut animations {
        if paused {
            player.pause_all();
        } else {
            player.resume_all();
        }
    }
}
