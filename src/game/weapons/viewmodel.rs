use super::{WeaponState, KNIFE};
use crate::game::{
    assets::GameAssets,
    config::WeaponId,
    matchplay::Combatant,
    player::{player::PlayerEntity, player_model::PlayerModel, skins::SkinId},
};
use bevy::{
    prelude::*,
    render::{
        camera::{ClearColorConfig, Exposure},
        view::RenderLayers,
    },
};
use bevy_fps_controller::controller::RenderPlayer;
#[derive(Component)]
pub struct ViewModel {
    actor: Entity,
    weapon_id: WeaponId,
    skin_index: usize,
    last_action: u64,
    last_equip: u64,
    was_visible: bool,
    first_person: bool,
    player: Option<Entity>,
    last_reload: bool,
    last_shot: u64,
    nodes: Vec<AnimationNodeIndex>,
    reload_duration: f32,
}
#[derive(Component)]
pub struct LayersBound;
#[derive(Component)]
pub struct WorldWeapon;
#[derive(Component)]
pub struct FlashBound;
#[derive(Component)]
pub struct MuzzleFlash {
    actor: Entity,
}

pub fn bind_muzzles(
    mut commands: Commands,
    models: Query<(Entity, &ViewModel), (With<LayersBound>, Without<FlashBound>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shared: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let (mesh, material) = shared
        .get_or_insert_with(|| {
            (
                meshes.add(Sphere::new(0.04)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.7, 0.15),
                    emissive: LinearRgba::rgb(8.0, 3.0, 0.3),
                    unlit: true,
                    ..default()
                }),
            )
        })
        .clone();
    for (root, model) in &models {
        if model.weapon_id != WeaponId::AK47 {
            commands.entity(root).insert(FlashBound);
            continue;
        }
        if let Some(socket) = children
            .iter_descendants(root)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Muzzle"))
        {
            commands.entity(socket).with_child((
                MuzzleFlash { actor: model.actor },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_scale(Vec3::new(1.0, 1.0, 2.5)),
                Visibility::Hidden,
                RenderLayers::layer(if model.first_person { 1 } else { 0 }),
                bevy::render::view::NoFrustumCulling,
            ));
            commands.entity(root).insert(FlashBound);
        }
    }
}
pub fn animate_flashes(
    weapons: Query<(&WeaponState, &Combatant)>,
    mut flashes: Query<(&MuzzleFlash, &mut Visibility)>,
) {
    for (flash, mut visibility) in &mut flashes {
        *visibility = if weapons
            .get(flash.actor)
            .is_ok_and(|(w, a)| a.alive() && w.flash_remaining > 0.035)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Framing is independent of world FOV and hit-query origins.
fn profile(id: WeaponId) -> Transform {
    let offset = match id {
        WeaponId::AK47 => Vec3::new(0.06, -1.65, -0.30),
        WeaponId::DefaultKnife => Vec3::new(0.0, -1.56, -0.14),
    };
    Transform::from_translation(offset).with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
}
pub fn spawn(
    commands: &mut Commands,
    assets: &GameAssets,
    gltfs: &Assets<Gltf>,
    actor: Entity,
    _world_camera: Entity,
    skin: SkinId,
) {
    let skin_index = match skin {
        SkinId::Soldier => 0,
        SkinId::Police => 1,
    };
    let mut roots = Vec::new();
    for weapon_id in [WeaponId::AK47, WeaponId::DefaultKnife] {
        let handle = match weapon_id {
            WeaponId::AK47 => &assets.arms[skin_index],
            WeaponId::DefaultKnife => &assets.knife_view[skin_index],
        };
        let framing = profile(weapon_id);
        roots.push(
            commands
                .spawn((
                    SceneRoot(gltfs.get(handle).unwrap().scenes[0].clone()),
                    framing,
                    ViewModel {
                        actor,
                        weapon_id,
                        skin_index,
                        first_person: true,
                        player: None,
                        last_action: 0,
                        last_equip: 0,
                        was_visible: false,
                        last_reload: false,
                        last_shot: 0,
                        nodes: Vec::new(),
                        reload_duration: 0.0,
                    },
                    Visibility::Hidden,
                ))
                .id(),
        );
    }
    let light = commands
        .spawn((
            PointLight {
                intensity: 1000.0,
                range: 4.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(-0.3, 0.5, -0.2),
            RenderLayers::layer(1),
        ))
        .id();
    commands
        .spawn((
            PlayerEntity,
            Camera3d::default(),
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                hdr: true,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 75_f32.to_radians(),
                near: 0.01,
                far: 10.0,
                ..default()
            }),
            Exposure { ev100: 12.0 },
            RenderPlayer {
                logical_entity: actor,
            },
            RenderLayers::layer(1),
        ))
        .add_children(&roots)
        .add_child(light);
}
pub fn bind_scenes(
    mut commands: Commands,
    assets: Res<GameAssets>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    clips_store: Res<Assets<AnimationClip>>,
    descendants: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    names: Query<&Name>,
    mut models: Query<(Entity, &mut ViewModel), Without<LayersBound>>,
    characters: Query<
        (
            Entity,
            Option<&PlayerModel>,
            Option<&crate::game::player::animation::ShowcaseAnimation>,
        ),
        (
            With<crate::game::player::animation::CharacterRig>,
            Without<WorldWeapon>,
        ),
    >,
) {
    let Some(_) = gltfs.get(&assets.gun) else {
        return;
    };
    for (character, model, showcase) in &characters {
        let socket = if showcase.is_some() {
            "MenuWeaponSocket"
        } else {
            "mixamorig:RightHand"
        };
        for entity in descendants.iter_descendants(character) {
            if names.get(entity).is_ok_and(|n| n.as_str() == socket) {
                for weapon_id in [WeaponId::AK47, WeaponId::DefaultKnife] {
                    if showcase.is_some() && weapon_id == WeaponId::DefaultKnife {
                        continue;
                    }
                    let handle = if weapon_id == WeaponId::AK47 {
                        &assets.gun
                    } else {
                        &assets.knife_world
                    };
                    let Some(asset) = gltfs.get(handle) else {
                        continue;
                    };
                    commands.entity(entity).with_child((
                        SceneRoot(asset.scenes[0].clone()),
                        Transform::from_scale(Vec3::splat(100.0)),
                        ViewModel {
                            actor: model.map(|m| m.logical_entity).unwrap_or(character),
                            weapon_id,
                            skin_index: 0,
                            first_person: false,
                            player: None,
                            last_action: 0,
                            last_equip: 0,
                            was_visible: false,
                            last_reload: false,
                            last_shot: 0,
                            nodes: Vec::new(),
                            reload_duration: 0.0,
                        },
                        if weapon_id == WeaponId::AK47 {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        },
                    ));
                }
                commands.entity(character).insert(WorldWeapon);
                break;
            }
        }
    }
    for (root, mut model) in &mut models {
        for entity in descendants.iter_descendants(root) {
            commands
                .entity(entity)
                .insert(RenderLayers::layer(if model.first_person { 1 } else { 0 }));
        }
        // World knife motion belongs to the character's upper-body animation.
        if model.weapon_id == WeaponId::DefaultKnife && !model.first_person {
            if descendants.iter_descendants(root).next().is_some() {
                commands
                    .entity(root)
                    .insert((LayersBound, RenderLayers::layer(0)));
            }
            continue;
        }
        let handle = match (model.weapon_id, model.first_person) {
            (WeaponId::AK47, true) => &assets.arms[model.skin_index],
            (WeaponId::AK47, false) => &assets.gun,
            (WeaponId::DefaultKnife, _) => &assets.knife_view[model.skin_index],
        };
        let Some(gltf) = gltfs.get(handle) else {
            continue;
        };
        let names = if model.weapon_id == WeaponId::AK47 {
            ["idle_rifle", "fire_rifle", "reload_rifle"]
        } else {
            ["idle_knife", "slash_knife", "draw_knife"]
        };
        let Some(clips) = names
            .iter()
            .map(|name| gltf.named_animations.get(*name).cloned())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let Some(player) = descendants
            .iter_descendants(root)
            .find(|e| players.contains(*e))
        else {
            continue;
        };
        model.reload_duration = clips_store
            .get(&clips[2])
            .map(AnimationClip::duration)
            .unwrap_or(0.0);
        let (graph, nodes) = AnimationGraph::from_clips(clips);
        let mut transitions = AnimationTransitions::new();
        if let Ok(mut animation) = players.get_mut(player) {
            // Register idle with the transition owner; starting it directly
            // leaves an untracked loop blending over every fire/reload action.
            transitions
                .play(&mut animation, nodes[0], std::time::Duration::ZERO)
                .repeat();
        }
        commands
            .entity(player)
            .insert((AnimationGraphHandle(graphs.add(graph)), transitions));
        for entity in descendants.iter_descendants(root) {
            commands
                .entity(entity)
                .insert(RenderLayers::layer(if model.first_person { 1 } else { 0 }));
            if model.first_person {
                commands
                    .entity(entity)
                    .insert(bevy::render::view::NoFrustumCulling);
            }
        }
        commands.entity(root).insert((
            RenderLayers::layer(if model.first_person { 1 } else { 0 }),
            LayersBound,
        ));
        model.player = Some(player);
        model.nodes = nodes;
    }
}
pub fn animate_viewmodel(
    mut models: Query<(
        &mut ViewModel,
        &mut Visibility,
        &mut Transform,
        Option<&LayersBound>,
    )>,
    weapons: Query<(&WeaponState, &Combatant)>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (mut model, mut visibility, mut transform, bound) in &mut models {
        if bound.is_none() {
            *visibility = Visibility::Hidden;
            continue;
        }
        let Ok((weapon, actor)) = weapons.get(model.actor) else {
            continue;
        };
        let visible = actor.alive() && weapon.active == model.weapon_id;
        if model.first_person && model.weapon_id == WeaponId::AK47 {
            *transform = profile(WeaponId::AK47);
            transform.translation.y -=
                0.22 * (weapon.equip_remaining / KNIFE.equip_seconds).clamp(0.0, 1.0);
        }
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let Some(entity) = model.player else {
            continue;
        };
        let Ok((mut player, mut transitions)) = players.get_mut(entity) else {
            continue;
        };
        if !visible {
            player.stop_all();
            model.was_visible = false;
            model.last_reload = false;
            model.last_shot = weapon.shots;
            continue;
        }
        if model.weapon_id == WeaponId::DefaultKnife {
            if !model.was_visible || model.last_equip != weapon.equips {
                transitions
                    .play(&mut player, model.nodes[2], std::time::Duration::ZERO)
                    .set_speed(model.reload_duration / KNIFE.equip_seconds)
                    .replay();
            } else if model.last_action != weapon.slashes {
                transitions
                    .play(&mut player, model.nodes[1], std::time::Duration::ZERO)
                    .set_speed((17.0 / 30.0) / (KNIFE.windup + KNIFE.recovery))
                    .replay();
            } else if player.all_finished() {
                transitions
                    .play(
                        &mut player,
                        model.nodes[0],
                        std::time::Duration::from_secs_f32(0.08),
                    )
                    .repeat();
            }
            model.last_action = weapon.slashes;
            model.last_equip = weapon.equips;
            model.was_visible = true;
            continue;
        }
        if !model.was_visible {
            transitions
                .play(&mut player, model.nodes[0], std::time::Duration::ZERO)
                .repeat();
            model.was_visible = true;
        }
        let reloading = weapon.reload_remaining > 0.0;
        if reloading && !model.last_reload {
            let active = transitions.play(
                &mut player,
                model.nodes[2],
                std::time::Duration::from_secs_f32(0.08),
            );
            active.replay();
            active.set_speed(model.reload_duration / super::AK47.reload_seconds);
        } else if !reloading && model.last_shot < weapon.shots {
            transitions
                .play(&mut player, model.nodes[1], std::time::Duration::ZERO)
                .replay();
        } else if !reloading
            && (model.last_reload || weapon.shots < model.last_shot || player.all_finished())
        {
            transitions
                .play(
                    &mut player,
                    model.nodes[0],
                    std::time::Duration::from_secs_f32(0.1),
                )
                .repeat();
        }
        model.last_reload = reloading;
        model.last_shot = weapon.shots;
    }
}
