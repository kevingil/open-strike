use super::targets;
use crate::game::{
    assets::GameAssets,
    config::GameConfig,
    map::{spawn_lighting, MapConfig},
    sound_library::SoundLibrary,
    GameState,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
#[derive(Component, Clone)]
pub struct LevelEntity;
#[derive(Resource)]
pub struct GameplayMapConfigHandle {
    handle: Handle<MapConfig>,
    base_path: String,
}
#[derive(Resource, Default)]
pub struct LoadedGameplayMapConfig {
    pub config: Option<MapConfig>,
    pub base_path: String,
}
#[derive(Resource, Default)]
pub struct LoadingStatus {
    pub message: String,
    scene: Option<Handle<Scene>>,
    spawned: bool,
    ready_tick: Option<u64>,
    started: f32,
    failed: bool,
}
#[derive(Resource, Default)]
pub struct PhysicsTick(pub u64);

pub struct LevelPlugin;
impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(targets::TargetsPlugin)
            .init_resource::<LoadedGameplayMapConfig>()
            .init_resource::<LoadingStatus>()
            .init_resource::<PhysicsTick>()
            .add_systems(
                FixedUpdate,
                (|mut tick: ResMut<PhysicsTick>| tick.0 += 1).after(PhysicsSet::Writeback),
            )
            .add_systems(
                OnEnter(GameState::Loading),
                (cleanup_level, start_loading_map_config).chain(),
            )
            .add_systems(Update, load_level.run_if(in_state(GameState::Loading)))
            .add_systems(OnEnter(GameState::MainMenu), cleanup_level)
            .add_systems(
                OnEnter(GameState::LoadFailed),
                |mut status: ResMut<LoadingStatus>| status.failed = true,
            );
    }
}
fn start_loading_map_config(
    mut commands: Commands,
    server: Res<AssetServer>,
    config: Res<GameConfig>,
    mut loaded: ResMut<LoadedGameplayMapConfig>,
    mut status: ResMut<LoadingStatus>,
    time: Res<Time<bevy::time::Real>>,
    assets: Res<GameAssets>,
    sounds: Res<SoundLibrary>,
    mut animations: ResMut<crate::game::player::animation::SharedAnimations>,
) {
    if status.failed {
        sounds.retry_failed(&server);
        if let Some(path) = status
            .scene
            .as_ref()
            .and_then(|scene| server.get_path(scene.id()))
        {
            server.reload(path.without_label());
        }
        for asset in assets
            .skins
            .iter()
            .chain(assets.arms.iter())
            .chain(assets.knife_view.iter())
            .chain(assets.knife_poses.iter())
            .chain([&assets.gun, &assets.knife_world])
        {
            if let Some(path) = server.get_path(asset.id()) {
                server.reload(path);
            }
        }
        server.reload(config.map.config_path());
        *animations = crate::game::player::animation::SharedAnimations::default();
    }
    *loaded = LoadedGameplayMapConfig::default();
    *status = LoadingStatus {
        message: "Loading map and character assets...".into(),
        started: time.elapsed_secs(),
        ..default()
    };
    let path = config.map.config_path();
    commands.insert_resource(GameplayMapConfigHandle {
        handle: server.load(path),
        base_path: path.rsplit_once('/').unwrap().0.into(),
    });
}
fn load_level(
    gltfs: Res<Assets<Gltf>>,
    animations: Res<crate::game::player::animation::SharedAnimations>,
    runtime: (
        Res<PhysicsTick>,
        Res<Time<bevy::time::Real>>,
        Res<GameConfig>,
        Res<SoundLibrary>,
    ),
    mut commands: Commands,
    server: Res<AssetServer>,
    configs: Res<Assets<MapConfig>>,
    handle: Res<GameplayMapConfigHandle>,
    assets: Res<GameAssets>,
    mut loaded: ResMut<LoadedGameplayMapConfig>,
    mut status: ResMut<LoadingStatus>,
    mut next: ResMut<NextState<GameState>>,
    colliders: Query<Option<&RapierColliderHandle>, With<Collider>>,
    pending: Query<(), With<AsyncSceneCollider>>,
    scene_spawner: Res<SceneSpawner>,
    instances: Query<&bevy::scene::SceneInstance, With<LevelEntity>>,
    context: ReadRapierContext,
) {
    let (tick, time, game, sounds) = runtime;
    if time.elapsed_secs() - status.started > 60.0 {
        status.message =
            "Loading timed out after 60 seconds. Check the required assets and collision geometry."
                .into();
        next.set(GameState::LoadFailed);
        return;
    }
    let error = match server.load_state(handle.handle.id()) {
        bevy::asset::LoadState::Failed(e) => Some(e.to_string()),
        _ => assets.failure(&server).or_else(|| sounds.failure(&server)),
    };
    if let Some(error) = error {
        status.message = error;
        next.set(GameState::LoadFailed);
        return;
    }
    if !status.spawned {
        let Some(config) = configs.get(&handle.handle) else {
            return;
        };
        if config.spawn_points.is_empty() {
            status.message = "Map has no spawn points".into();
            next.set(GameState::LoadFailed);
            return;
        }
        if game.mode == crate::game::config::GameMode::TeamDeathmatch
            && (config.navigation.is_empty()
                || config.patrol_destinations.is_empty()
                || config
                    .patrol_destinations
                    .iter()
                    .any(|&i| i >= config.navigation.len())
                || [
                    crate::game::matchplay::Team::Attacker,
                    crate::game::matchplay::Team::Defender,
                ]
                .iter()
                .any(|team| {
                    config
                        .spawn_points
                        .iter()
                        .filter(|s| s.team == Some(*team))
                        .count()
                        < 3
                }))
        {
            status.message =
                "Team Deathmatch requires navigation and three spawn points per team".into();
            next.set(GameState::LoadFailed);
            return;
        }
        loaded.config = Some(config.clone());
        loaded.base_path = handle.base_path.clone();
        let scene: Handle<Scene> =
            server.load(format!("{}/{}#Scene0", handle.base_path, config.model));
        commands.spawn((
            LevelEntity,
            SceneRoot(scene.clone()),
            config.transform.to_transform(),
            AsyncSceneCollider {
                shape: Some(ComputedColliderShape::TriMesh(
                    TriMeshFlags::MERGE_DUPLICATE_VERTICES,
                )),
                named_shapes: default(),
            },
        ));
        spawn_lighting(&mut commands, &config.lighting, LevelEntity);
        commands.insert_resource(ClearColor(
            config
                .clear_color
                .map(|c| c.to_color())
                .unwrap_or(Color::srgb(0.53, 0.7, 0.85)),
        ));
        status.scene = Some(scene);
        status.spawned = true;
        status.message = "Preparing collision and animation bindings...".into();
        return;
    }
    if let Some(scene) = &status.scene {
        if let bevy::asset::LoadState::Failed(e) = server.load_state(scene.id()) {
            status.message = e.to_string();
            next.set(GameState::LoadFailed);
            return;
        }
        if !server.is_loaded_with_dependencies(scene.id()) {
            return;
        }
    }
    if !assets.ready(&server)
        || !sounds.ready(&server)
        || !pending.is_empty()
        || instances.is_empty()
        || !instances
            .iter()
            .all(|i| scene_spawner.instance_is_ready(**i))
    {
        return;
    }
    for (index, handle) in assets.skins.iter().enumerate() {
        if let Some(gltf) = gltfs.get(handle) {
            if let Some(missing) = crate::game::player::animation::CLIP_NAMES
                .iter()
                .find(|name| !gltf.named_animations.contains_key(**name))
            {
                status.message = format!("Character {} missing required clip {}", index, missing);
                next.set(GameState::LoadFailed);
                return;
            }
        }
    }
    for handle in assets.arms.iter().chain([&assets.gun]) {
        let Some(gltf) = gltfs.get(handle) else {
            return;
        };
        if gltf.scenes.is_empty()
            || ["idle_rifle", "fire_rifle", "reload_rifle"]
                .iter()
                .any(|name| !gltf.named_animations.contains_key(*name))
            || ["Muzzle", "Magazine", "Bolt", "WeaponGrip"]
                .iter()
                .any(|name| !gltf.named_nodes.contains_key(*name))
        {
            status.message =
                "Weapon export is missing a required scene, named action or socket".into();
            next.set(GameState::LoadFailed);
            return;
        }
    }
    for (label, handle, clips) in [
        (
            "Knife Soldier view",
            &assets.knife_view[0],
            &["idle_knife", "draw_knife", "slash_knife"][..],
        ),
        (
            "Knife Police view",
            &assets.knife_view[1],
            &["idle_knife", "draw_knife", "slash_knife"][..],
        ),
        ("Knife world", &assets.knife_world, &[][..]),
        (
            "Attacker knife pose",
            &assets.knife_poses[0],
            &["idle_knife", "draw_knife", "slash_knife"][..],
        ),
        (
            "Defender knife pose",
            &assets.knife_poses[1],
            &["idle_knife", "draw_knife", "slash_knife"][..],
        ),
    ] {
        let Some(gltf) = gltfs.get(handle) else {
            return;
        };
        if gltf.scenes.is_empty()
            || clips
                .iter()
                .any(|name| !gltf.named_animations.contains_key(*name))
        {
            status.message = format!("{label} export is missing a scene or required knife clip");
            next.set(GameState::LoadFailed);
            return;
        }
        if label.starts_with("Knife") && !gltf.named_nodes.contains_key("KnifeGrip") {
            status.message = format!("{label} export is missing KnifeGrip");
            next.set(GameState::LoadFailed);
            return;
        }
    }
    if animations.count == 0
        || colliders.is_empty()
        || colliders.iter().any(|handle| handle.is_none())
    {
        return;
    }
    if let Some(ready_tick) = status.ready_tick {
        if tick.0 <= ready_tick {
            return;
        }
    } else {
        status.ready_tick = Some(tick.0);
        return;
    }
    let Ok(physics) = context.single() else {
        return;
    };
    let config = loaded.config.as_ref().unwrap();
    for spawn in &config.spawn_points {
        let feet = config
            .transform
            .to_transform()
            .transform_point(spawn.position.to_vec3());
        if physics
            .cast_ray(
                feet + Vec3::Y * 0.5,
                -Vec3::Y,
                1.5,
                true,
                QueryFilter::default().exclude_sensors(),
            )
            .is_none()
        {
            status.message = format!("Invalid spawn: no ground below {:?}", feet);
            next.set(GameState::LoadFailed);
            return;
        }
    }
    let navigation = crate::game::bots::navigation::build(config, &physics);
    if game.mode == crate::game::config::GameMode::TeamDeathmatch {
        for spawn in &config.spawn_points {
            let Some(node) = navigation.nearest(
                config
                    .transform
                    .to_transform()
                    .transform_point(spawn.position.to_vec3()),
            ) else {
                status.message = "No walkable navigation at spawn".into();
                next.set(GameState::LoadFailed);
                return;
            };
            if navigation
                .destinations
                .iter()
                .any(|&goal| navigation.path(node, goal).is_empty())
            {
                status.message = "Spawn cannot reach every authored combat destination".into();
                next.set(GameState::LoadFailed);
                return;
            }
        }
    }
    commands.insert_resource(navigation);
    info!("Map ready: {}", config.name);
    next.set(GameState::Playing);
}
fn cleanup_level(mut commands: Commands, query: Query<Entity, With<LevelEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
