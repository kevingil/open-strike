use super::{
    animation::{CharacterRig, PlayerAnimationController},
    player_model::{HitboxZoneMarker, PlayerModel},
    skins::{HitboxZoneType, SkinId, STANDARD_HITBOX},
};
use crate::game::{
    assets::GameAssets,
    config::{GameConfig, GameMode, PlayerLoadout, PlayerSettings},
    level::level::LoadedGameplayMapConfig,
    map::create_camera_components,
    matchplay::{ActorIntent, Combatant, Team},
    GameState,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_rapier3d::prelude::*;

pub const BODY_HEIGHT: f32 = 1.8;
pub const BODY_RADIUS: f32 = 0.30;
#[derive(Component)]
pub struct PlayerEntity;
#[derive(Component)]
pub struct LocalPlayer;
#[derive(Component)]
pub struct WorldCamera;
pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::game::shooting::tracer::TracerPlugin)
            .add_systems(Startup, super::animation::load_shared_animations)
            .add_systems(
                Update,
                (
                    super::player_model::bind_animated_visibility,
                    super::animation::prepare_graphs,
                    super::animation::setup_animation_player,
                    super::animation::detect_animation_state,
                    super::animation::update_player_animations,
                )
                    .chain(),
            )
            .add_systems(
                PreUpdate,
                super::input::human_input
                    .after(bevy::input::InputSystem)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                init_player.run_if(|q: Query<(), With<LocalPlayer>>| q.is_empty()),
            )
            .add_systems(OnEnter(GameState::MainMenu), cleanup_player)
            .add_systems(OnEnter(GameState::Loading), cleanup_player)
            .add_systems(Update, update_fov)
            .add_systems(
                FixedUpdate,
                super::player_model::sync_hit_zones
                    .after(crate::game::game::SimulationSet::Movement)
                    .before(PhysicsSet::SyncBackend)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                super::player_model::sync_player_model
                    .before(TransformSystem::TransformPropagate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn init_player(
    mut commands: Commands,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    settings: Res<PlayerSettings>,
    loadout: Res<PlayerLoadout>,
    config: Res<GameConfig>,
    map: Res<LoadedGameplayMapConfig>,
) {
    let map = map
        .config
        .as_ref()
        .expect("Loading guarantees map configuration");
    let human_team = if loadout.selected_skin == SkinId::Police {
        Team::Defender
    } else {
        Team::Attacker
    };
    let count = if config.mode == GameMode::TeamDeathmatch {
        6
    } else {
        1
    };
    for slot in 0..count {
        let team = if slot < 3 {
            human_team
        } else if human_team == Team::Attacker {
            Team::Defender
        } else {
            Team::Attacker
        };
        let spawn = map
            .spawn_points
            .iter()
            .filter(|s| s.team.is_none() || s.team == Some(team))
            .nth(slot % 3)
            .or_else(|| {
                map.spawn_points
                    .iter()
                    .find(|s| s.team.is_none() || s.team == Some(team))
            })
            .expect("Validated team spawn");
        let feet = map
            .transform
            .to_transform()
            .transform_point(spawn.position.to_vec3());
        let skin = if team == Team::Attacker {
            SkinId::Soldier
        } else {
            SkinId::Police
        };
        let body = commands
            .spawn((
                PlayerEntity,
                LogicalPlayer,
                Transform::from_translation(feet + Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03)),
                Visibility::default(),
                Collider::cylinder(BODY_HEIGHT * 0.5, BODY_RADIUS),
                RigidBody::Dynamic,
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                GravityScale(0.0),
                Sleeping::disabled(),
                Ccd { enabled: true },
                Friction {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Min,
                },
                Restitution {
                    coefficient: 0.0,
                    combine_rule: CoefficientCombineRule::Min,
                },
            ))
            .insert((
                Combatant::new(team, slot),
                super::presentation::PoseHistory::new(
                    feet + Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03),
                    spawn.rotation.to_radians(),
                ),
                ActorIntent::default(),
                crate::game::weapons::WeaponState::default(),
                crate::game::weapons::audio::AudioState::default(),
                FpsControllerInput {
                    yaw: spawn.rotation.to_radians(),
                    ..default()
                },
                FpsController {
                    enable_input: false,
                    radius: BODY_RADIUS,
                    height: BODY_HEIGHT,
                    upright_height: BODY_HEIGHT,
                    crouch_height: 1.4,
                    walk_speed: 4.5,
                    run_speed: 6.2,
                    crouched_speed: 2.3,
                    jump_speed: 6.0,
                    gravity: 19.6,
                    step_offset: 0.48,
                    sensitivity: settings.sensitivity * 0.001,
                    ..default()
                },
                CameraConfig {
                    height_offset: -0.15,
                },
            ))
            .id();
        if slot == 0 {
            commands.entity(body).insert(LocalPlayer);
            if std::env::var_os("CSRS_BOT_PLAYER").is_some() {
                commands
                    .entity(body)
                    .insert(crate::game::bots::BotController::new(0));
            }
        } else {
            commands
                .entity(body)
                .insert(crate::game::bots::BotController::new(slot));
        }
        let gltf = gltfs.get(&assets.skins[team.index()]).unwrap();
        commands.spawn((
            PlayerEntity,
            PlayerModel {
                logical_entity: body,
                is_local_player: slot == 0,
            },
            CharacterRig(skin),
            PlayerAnimationController::default(),
            SceneRoot(gltf.scenes[0].clone()),
            Transform::from_translation(feet),
            Visibility::Inherited,
        ));
        for (kind, zone) in [
            (HitboxZoneType::Head, &STANDARD_HITBOX.head),
            (HitboxZoneType::Torso, &STANDARD_HITBOX.torso),
            (HitboxZoneType::Legs, &STANDARD_HITBOX.legs),
        ] {
            commands.spawn((
                Collider::cuboid(
                    zone.half_extents.x,
                    zone.half_extents.y,
                    zone.half_extents.z,
                ),
                Sensor,
                Transform::from_translation(zone.offset - Vec3::Y * (BODY_HEIGHT * 0.5)),
                HitboxZoneMarker {
                    zone_type: kind,
                    player_entity: body,
                },
                ChildOf(body),
            ));
        }
        if slot == 0 {
            let (exposure, bloom, tonemapping, fog) = create_camera_components(map);
            let mut camera = commands.spawn((
                PlayerEntity,
                WorldCamera,
                SpatialListener::new(0.2),
                Camera3d::default(),
                Camera {
                    hdr: true,
                    ..default()
                },
                Projection::Perspective(PerspectiveProjection {
                    fov: 2.0 * ((settings.fov.to_radians() * 0.5).tan() / (16.0 / 9.0)).atan(),
                    near: 0.05,
                    ..default()
                }),
                RenderPlayer {
                    logical_entity: body,
                },
                Transform::default(),
                exposure,
                bloom,
                tonemapping,
            ));
            if let Some(fog) = fog {
                camera.insert(fog);
            }
            let camera = camera.id();
            if std::env::var_os("CSRS_NO_VIEWMODEL").is_none() {
                crate::game::weapons::viewmodel::spawn(
                    &mut commands,
                    &assets,
                    &gltfs,
                    body,
                    camera,
                    loadout.selected_skin,
                );
            }
        }
    }
}
fn cleanup_player(mut commands: Commands, query: Query<Entity, With<PlayerEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn update_fov(
    settings: Res<PlayerSettings>,
    windows: Query<&Window>,
    mut cameras: Query<&mut Projection, With<WorldCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let aspect = window.width() / window.height().max(1.0);
    for mut projection in &mut cameras {
        if let Projection::Perspective(ref mut p) = *projection {
            p.fov =
                2.0 * ((settings.fov.clamp(60.0, 120.0).to_radians() * 0.5).tan() / aspect).atan();
        }
    }
}
