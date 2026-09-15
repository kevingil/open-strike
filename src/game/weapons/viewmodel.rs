use super::{WeaponState, KNIFE};
use crate::game::{
    assets::GameAssets,
    config::WeaponId,
    matchplay::Combatant,
    player::{player::PlayerEntity, player_model::PlayerModel, skins::SkinId},
};
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        camera::{ClearColorConfig, Exposure},
        view::RenderLayers,
    },
};
use bevy_fps_controller::controller::RenderPlayer;

// Stretch the native 1.33-second holding loop to four seconds.
const KNIFE_IDLE_SPEED: f32 = 1.0 / 3.0;
/// The reference arms' knife socket and the blade it was authored with.
const KNIFE_SOCKET: &str = "KnifeGrip";
const REFERENCE_BLADE: &str = "knife_knife_0";
/// Karambit draw: two turns around the finger ring, easing out.
const KARAMBIT_SPIN_SECONDS: f32 = 0.55;
const KARAMBIT_SPIN_TURNS: f32 = 2.0;
/// Ring centre in the catalog karambit's own metres (blade on +Y, ring on -Y).
const KARAMBIT_RING: Vec3 = Vec3::new(0.0, -0.11, -0.03);

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
    slash_duration: f32,
    /// Catalog blade attached to the reference arms' knife socket.
    attachment: Option<Handle<Gltf>>,
}
#[derive(Component)]
pub struct LayersBound;
/// A catalog knife parented to the first-person arms' `KnifeGrip` socket.
#[derive(Component)]
pub struct KnifeAttachment {
    actor: Entity,
    weapon_id: WeaponId,
    grip: Transform,
    last_equip: u64,
    spin_elapsed: f32,
}
#[derive(Component)]
pub struct AttachmentBound;
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
        if model.weapon_id.is_knife() {
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
    if id.is_knife() {
        // Native reference centimetres, viewed from (0, 10, 32) in Blender.
        let tilt = Quat::from_rotation_x((13.0_f32 / 40.0).atan());
        // The karambit ring sits on the index finger, so turn the fist a
        // little toward the player to bring that finger into view.
        let yaw = Quat::from_rotation_y(if id == WeaponId::Karambit {
            8_f32.to_radians()
        } else {
            0.0
        });
        // Frame the karambit lower and nearer the centre, like its reference.
        let offset = if id == WeaponId::Karambit {
            Vec3::new(-0.03, -0.035, 0.0)
        } else {
            Vec3::ZERO
        };
        Transform {
            translation: yaw * (tilt * Vec3::new(0.0, -0.32, -0.10) + Vec3::Y * 0.075) + offset,
            rotation: yaw * tilt * Quat::from_rotation_y(std::f32::consts::PI),
            scale: Vec3::splat(0.01),
        }
    } else if id.has_viewmodel() {
        // The AKM reference rig is authored in source units around its camera.
        Transform::from_xyz(-0.005, -0.05, 0.0)
            .with_rotation(
                Quat::from_rotation_x(2_f32.to_radians())
                    * Quat::from_rotation_y(std::f32::consts::PI),
            )
            .with_scale(Vec3::splat(0.025))
    } else {
        // Catalog guns are exported in meters with the barrel on +Y.
        Transform::from_xyz(0.18, -0.16, -0.38).with_rotation(
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                * Quat::from_rotation_y(std::f32::consts::PI),
        )
    }
}

/// Every knife is held by the reference arms; only the AK has its own rig.
fn has_arms(id: WeaponId) -> bool {
    id == WeaponId::AK47 || id.is_knife()
}

/// The first-person scene for a weapon: dedicated arms where they exist,
/// the reference knife arms for every other knife, and the bare world
/// model for catalog guns.
fn arms_handle(assets: &GameAssets, id: WeaponId, skin_index: usize) -> &Handle<Gltf> {
    match id {
        WeaponId::AK47 => &assets.arms[skin_index],
        WeaponId::DefaultKnife => &assets.knife_view[skin_index],
        WeaponId::ReferenceKnife => &assets.reference_knife_view[skin_index],
        other if other.is_knife() => &assets.reference_knife_view[skin_index],
        other => assets.world(other),
    }
}

/// Blade placed in the reference arms' `KnifeGrip` socket, in the socket's
/// native centimetres. The reference blade points along +Y with its flat on
/// Z, and the fist closes around roughly y = -7 .. 0.
fn knife_grip(id: WeaponId) -> Option<Transform> {
    use std::f32::consts::{FRAC_PI_2, PI};
    match id {
        WeaponId::Karambit => Some(
            // Reverse grip: the ring sits on the index finger at the top of the
            // fist and the blade leaves the little-finger side, angled up across
            // the fist so it clears the sleeve, with the hook curving back down
            // at the tip. Scaled to a hand-sized 22 cm knife.
            Transform::from_xyz(-6.0, 3.0, -3.6)
                .with_rotation(
                    Quat::from_rotation_z(120_f32.to_radians()) * Quat::from_rotation_y(PI),
                )
                .with_scale(Vec3::splat(70.0)),
        ),
        WeaponId::DefaultKnifeCt | WeaponId::DefaultKnifeT => Some(
            // Catalog knives already point +Y with the flat on Z; slide the
            // handle into the fist.
            Transform::from_xyz(0.0, 5.5, 0.0).with_scale(Vec3::splat(100.0)),
        ),
        WeaponId::DefaultKnifeStained | WeaponId::DefaultKnifeForest => Some(
            // The authored knife points -Z with its flat on Y.
            Transform::from_xyz(0.0, -3.5, 0.0)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_x(FRAC_PI_2))
                .with_scale(Vec3::splat(80.0)),
        ),
        _ => None,
    }
}

pub fn spawn(
    commands: &mut Commands,
    assets: &GameAssets,
    gltfs: &Assets<Gltf>,
    actor: Entity,
    world_camera: Entity,
    skin: SkinId,
) {
    let skin_index = match skin {
        SkinId::Soldier => 0,
        SkinId::Police => 1,
    };
    let mut roots = Vec::new();
    for weapon_id in WeaponId::all() {
        let handle = arms_handle(assets, weapon_id, skin_index);
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
                        slash_duration: 0.0,
                        attachment: knife_grip(weapon_id).map(|_| assets.world(weapon_id).clone()),
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
            ViewModelLight(actor),
            Transform::from_xyz(-0.3, 0.5, -0.2),
            RenderLayers::layer(1),
        ))
        .id();
    commands
        .spawn((
            PlayerEntity,
            ViewModelCamera {
                actor,
                world_camera,
            },
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
            Exposure::default(),
            AmbientLight::default(),
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
                // The menu pose is authored for the AK. Other catalog assets
                // must not delay or prevent its attachment.
                let wanted = WeaponId::ALL
                    .into_iter()
                    .filter(|weapon_id| showcase.is_none() || *weapon_id == WeaponId::AK47);
                if wanted
                    .clone()
                    .any(|weapon_id| gltfs.get(assets.world(weapon_id)).is_none())
                {
                    break;
                }
                for weapon_id in wanted {
                    let handle = assets.world(weapon_id);
                    let Some(asset) = gltfs.get(handle) else {
                        continue;
                    };
                    commands.entity(entity).with_child((
                        SceneRoot(asset.scenes[0].clone()),
                        Transform::from_scale(Vec3::splat(if weapon_id.has_viewmodel() {
                            100.0
                        } else {
                            1.0
                        })),
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
                            slash_duration: 0.0,
                            attachment: None,
                        },
                        if showcase.is_some() {
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
        if model.weapon_id.is_knife() && !model.first_person {
            if descendants.iter_descendants(root).next().is_some() {
                commands
                    .entity(root)
                    .insert((LayersBound, RenderLayers::layer(0)));
            }
            continue;
        }
        let handle = if model.first_person {
            arms_handle(&assets, model.weapon_id, model.skin_index)
        } else {
            assets.world(model.weapon_id)
        };
        let Some(gltf) = gltfs.get(handle) else {
            continue;
        };
        if !has_arms(model.weapon_id) {
            commands.entity(root).insert((
                RenderLayers::layer(if model.first_person { 1 } else { 0 }),
                LayersBound,
            ));
            continue;
        }
        let clip_names = if model.weapon_id == WeaponId::AK47 {
            ["idle_rifle", "fire_rifle", "reload_rifle"]
        } else {
            ["idle_knife", "slash_knife", "draw_knife"]
        };
        let Some(clips) = clip_names
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
        model.slash_duration = clips_store
            .get(&clips[1])
            .map(AnimationClip::duration)
            .unwrap_or(0.0);
        model.reload_duration = clips_store
            .get(&clips[2])
            .map(AnimationClip::duration)
            .unwrap_or(0.0);
        if let Some(attachment) = model.attachment.clone() {
            let Some(asset) = gltfs.get(&attachment) else {
                continue;
            };
            let Some(socket) = descendants
                .iter_descendants(root)
                .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == KNIFE_SOCKET))
            else {
                continue;
            };
            // The arms keep their authored blade; hide it and hang the
            // catalog knife from the same animated socket.
            for entity in descendants.iter_descendants(root) {
                if names
                    .get(entity)
                    .is_ok_and(|n| n.as_str() == REFERENCE_BLADE)
                {
                    commands.entity(entity).insert(Visibility::Hidden);
                }
            }
            let grip = knife_grip(model.weapon_id).unwrap_or_default();
            commands.entity(socket).with_child((
                SceneRoot(asset.scenes[0].clone()),
                grip,
                KnifeAttachment {
                    actor: model.actor,
                    weapon_id: model.weapon_id,
                    grip,
                    last_equip: 0,
                    spin_elapsed: KARAMBIT_SPIN_SECONDS,
                },
                RenderLayers::layer(1),
            ));
        }
        let (graph, nodes) = AnimationGraph::from_clips(clips);
        let mut transitions = AnimationTransitions::new();
        if let Ok(mut animation) = players.get_mut(player) {
            // Register idle with the transition owner; starting it directly
            // leaves an untracked loop blending over every fire/reload action.
            transitions
                .play(&mut animation, nodes[0], std::time::Duration::ZERO)
                .set_speed(if model.weapon_id.is_knife() {
                    KNIFE_IDLE_SPEED
                } else {
                    1.0
                })
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
/// Attached blades spawn after their arms; give every node the arms' layer.
pub fn bind_attachments(
    mut commands: Commands,
    attachments: Query<Entity, (With<KnifeAttachment>, Without<AttachmentBound>)>,
    descendants: Query<&Children>,
) {
    for root in &attachments {
        if descendants.iter_descendants(root).next().is_none() {
            continue;
        }
        for entity in descendants.iter_descendants(root) {
            commands
                .entity(entity)
                .insert((RenderLayers::layer(1), bevy::render::view::NoFrustumCulling));
        }
        commands.entity(root).insert(AttachmentBound);
    }
}

/// Draw flourish for attached blades. The karambit spins around its finger
/// ring while the arms play their draw clip; other knives simply ride the
/// socket.
pub fn animate_attachments(
    time: Res<Time>,
    weapons: Query<&WeaponState>,
    mut attachments: Query<(&mut KnifeAttachment, &mut Transform)>,
) {
    for (mut attachment, mut transform) in &mut attachments {
        let Ok(weapon) = weapons.get(attachment.actor) else {
            continue;
        };
        if attachment.weapon_id != WeaponId::Karambit {
            continue;
        }
        if attachment.last_equip != weapon.equips {
            attachment.last_equip = weapon.equips;
            attachment.spin_elapsed = if weapon.active == attachment.weapon_id {
                0.0
            } else {
                KARAMBIT_SPIN_SECONDS
            };
        }
        if attachment.spin_elapsed >= KARAMBIT_SPIN_SECONDS {
            *transform = attachment.grip;
            continue;
        }
        attachment.spin_elapsed =
            (attachment.spin_elapsed + time.delta_secs()).min(KARAMBIT_SPIN_SECONDS);
        let progress = attachment.spin_elapsed / KARAMBIT_SPIN_SECONDS;
        let eased = 1.0 - (1.0 - progress).powi(3);
        let angle = eased * KARAMBIT_SPIN_TURNS * std::f32::consts::TAU;
        // Spin about the ring's axis (the blade's thin X axis) so the ring
        // stays on the finger and the handle whips around it.
        *transform = attachment.grip
            * Transform::from_translation(KARAMBIT_RING)
            * Transform::from_rotation(Quat::from_rotation_x(angle))
            * Transform::from_translation(-KARAMBIT_RING);
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
        if model.first_person && !model.weapon_id.is_knife() {
            *transform = profile(model.weapon_id);
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
        if model.weapon_id.is_knife() {
            if !model.was_visible || model.last_equip != weapon.equips {
                transitions
                    .play(&mut player, model.nodes[2], std::time::Duration::ZERO)
                    .set_speed(model.reload_duration / KNIFE.equip_seconds)
                    .replay();
            } else if model.last_action != weapon.slashes {
                transitions
                    .play(&mut player, model.nodes[1], std::time::Duration::ZERO)
                    .set_speed(model.slash_duration / (KNIFE.windup + KNIFE.recovery))
                    .replay();
            } else if player.all_finished() {
                transitions
                    .play(
                        &mut player,
                        model.nodes[0],
                        std::time::Duration::from_secs_f32(0.08),
                    )
                    .set_speed(KNIFE_IDLE_SPEED)
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
            active.set_speed(model.reload_duration / weapon.active.stats().reload_seconds);
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

#[derive(Component)]
pub struct ViewModelCamera {
    actor: Entity,
    world_camera: Entity,
}

#[derive(Component)]
pub struct ViewModelLight(Entity);

pub fn frame_camera(
    weapons: Query<&WeaponState>,
    ambient: Res<AmbientLight>,
    world_cameras: Query<(&Exposure, &Tonemapping), Without<ViewModelCamera>>,
    mut cameras: Query<(
        &ViewModelCamera,
        &mut Projection,
        &mut AmbientLight,
        &mut Exposure,
        &mut Tonemapping,
    )>,
    mut lights: Query<(&ViewModelLight, &mut PointLight)>,
) {
    for (owner, mut light) in &mut lights {
        if let Ok(weapon) = weapons.get(owner.0) {
            // The rifle's glove seams and receiver need a camera-local key;
            // world sunlight belongs to layer 0 and cannot light these meshes.
            light.intensity = if weapon.active.is_knife() {
                1000.0
            } else if weapon.active.has_viewmodel() {
                70_000.0
            } else {
                8_000.0
            };
            light.radius = if weapon.active.has_viewmodel() && !weapon.active.is_knife() {
                0.25
            } else {
                0.0
            };
        }
    }
    for (camera, mut projection, mut lighting, mut exposure, mut tonemapping) in &mut cameras {
        // Composite the arms with the same exposure and color response as the world.
        if let Ok((world_exposure, world_tonemapping)) = world_cameras.get(camera.world_camera) {
            *exposure = *world_exposure;
            *tonemapping = *world_tonemapping;
        }
        *lighting = ambient.clone();
        if let (Ok(weapon), Projection::Perspective(p)) =
            (weapons.get(camera.actor), &mut *projection)
        {
            p.fov = if weapon.active.is_knife() {
                // Keep both hands framed on narrower windows as well as 16:9.
                let vertical = 55_f32.to_radians();
                vertical.max(2.0 * ((vertical * 0.5).tan() * (16.0 / 9.0) / p.aspect_ratio).atan())
            } else {
                // Hold the same lens throughout the authored reload.
                55_f32.to_radians()
            };
        }
    }
}
