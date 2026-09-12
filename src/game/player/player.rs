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
                init_player.run_if(
                    (|q: Query<(), With<LocalPlayer>>| q.is_empty())
                        .and(crate::game::net::role_is(crate::game::net::NetRole::Local)),
                ),
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
/// Who drives an actor and which cosmetic/presentation pieces it needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActorKind {
    /// The local human: camera, viewmodel, keyboard and mouse.
    LocalHuman,
    /// Server-side AI.
    Bot,
    /// A connected player simulated on the server from its reported inputs.
    Remote,
    /// A client-side mirror of an actor the server owns.
    Puppet,
}

pub struct ActorSpawn<'a> {
    pub slot: usize,
    pub team: Team,
    pub kind: ActorKind,
    pub name: Option<String>,
    /// Feet position and yaw in radians; taken from the map's spawn points when None.
    pub placement: Option<(Vec3, f32)>,
    pub melee_weapon: crate::game::config::WeaponId,
    pub primary_weapon: crate::game::config::WeaponId,
    pub skin: SkinId,
    pub map: &'a crate::game::map::MapConfig,
    /// Spawn the visible character; the dedicated server has nothing to draw.
    pub cosmetic: bool,
}

/// Spawns a logical body, its cosmetic model and hit zones. Returns the body.
pub fn spawn_actor(
    commands: &mut Commands,
    assets: &GameAssets,
    gltfs: &Assets<Gltf>,
    settings: &PlayerSettings,
    spawn: ActorSpawn,
) -> Entity {
    let map = spawn.map;
    let team = spawn.team;
    let slot = spawn.slot;
    let (feet, yaw) = spawn.placement.unwrap_or_else(|| {
        let point = map
            .spawn_points
            .iter()
            .filter(|s| s.team.is_none() || s.team == Some(team))
            .nth(slot % 3)
            .or_else(|| {
                map.spawn_points
                    .iter()
                    .find(|s| s.team.is_none() || s.team == Some(team))
            })
            .or_else(|| map.spawn_points.first())
            .expect("Validated spawn");
        (
            map.transform
                .to_transform()
                .transform_point(point.position.to_vec3()),
            point.rotation.to_radians(),
        )
    });
    let center = feet + Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03);
    let local = spawn.kind == ActorKind::LocalHuman;
    // Server-owned mirrors and remote players are placed, not integrated.
    let body_type = if matches!(spawn.kind, ActorKind::Remote | ActorKind::Puppet) {
        RigidBody::KinematicPositionBased
    } else {
        RigidBody::Dynamic
    };
    let mut combatant = match spawn.name {
        Some(name) => Combatant::named(team, slot, name),
        None => Combatant::new(team, slot),
    };
    if spawn.kind == ActorKind::Bot && combatant.name == "YOU" {
        combatant.name = crate::game::matchplay::BOT_NAMES[0].into();
    }
    let body = commands
        .spawn((
            PlayerEntity,
            LogicalPlayer,
            Transform::from_translation(center),
            Visibility::default(),
            Collider::cylinder(BODY_HEIGHT * 0.5, BODY_RADIUS),
            body_type,
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
            combatant,
            super::presentation::PoseHistory::new(center, yaw),
            ActorIntent::default(),
            crate::game::weapons::WeaponState::armed(spawn.primary_weapon, spawn.melee_weapon),
            crate::game::weapons::audio::AudioState::default(),
            FpsControllerInput { yaw, ..default() },
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
                yaw,
                ..default()
            },
            CameraConfig {
                height_offset: -0.15,
            },
        ))
        .id();
    match spawn.kind {
        ActorKind::LocalHuman => {
            commands.entity(body).insert(LocalPlayer);
            if std::env::var_os("CSRS_BOT_PLAYER").is_some() {
                commands
                    .entity(body)
                    .insert(crate::game::bots::BotController::new(0));
            }
        }
        ActorKind::Bot => {
            commands
                .entity(body)
                .insert(crate::game::bots::BotController::new(slot));
        }
        ActorKind::Remote | ActorKind::Puppet => {}
    }
    if spawn.cosmetic {
        let gltf = gltfs.get(&assets.skins[team.index()]).unwrap();
        commands.spawn((
            PlayerEntity,
            PlayerModel {
                logical_entity: body,
                is_local_player: local,
            },
            CharacterRig(spawn.skin),
            PlayerAnimationController::default(),
            SceneRoot(gltf.scenes[0].clone()),
            Transform::from_translation(feet),
            Visibility::Inherited,
        ));
    }
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
    if local {
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
                commands, assets, gltfs, body, camera, spawn.skin,
            );
        }
    }
    body
}

pub fn team_skin(team: Team) -> SkinId {
    if team == Team::Attacker {
        SkinId::Soldier
    } else {
        SkinId::Police
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
    let count = match config.mode {
        GameMode::TeamDeathmatch => 6,
        GameMode::Deathmatch => 8,
        GameMode::Freemode => 1,
    };
    for slot in 0..count {
        let team = if config.mode == GameMode::Deathmatch {
            if slot % 2 == 0 {
                human_team
            } else if human_team == Team::Attacker {
                Team::Defender
            } else {
                Team::Attacker
            }
        } else if slot < 3 {
            human_team
        } else if human_team == Team::Attacker {
            Team::Defender
        } else {
            Team::Attacker
        };
        let placement = (config.mode == GameMode::Deathmatch).then(|| {
            let point = &map.spawn_points[slot % map.spawn_points.len()];
            (
                map.transform
                    .to_transform()
                    .transform_point(point.position.to_vec3()),
                point.rotation.to_radians(),
            )
        });
        spawn_actor(
            &mut commands,
            &assets,
            &gltfs,
            &settings,
            ActorSpawn {
                slot,
                team,
                kind: if slot == 0 {
                    ActorKind::LocalHuman
                } else {
                    ActorKind::Bot
                },
                name: None,
                placement,
                melee_weapon: loadout.melee_weapon,
                primary_weapon: loadout.spawn_gun(if team == Team::Defender {
                    crate::game::player::skins::PlayerSide::Defender
                } else {
                    crate::game::player::skins::PlayerSide::Attacker
                }),
                skin: team_skin(team),
                map,
                cosmetic: true,
            },
        );
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
