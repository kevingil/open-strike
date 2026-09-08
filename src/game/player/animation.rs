use super::{player_model::PlayerModel, skins::SkinId};
use crate::game::{
    assets::GameAssets, config::WeaponId, matchplay::Combatant, weapons::WeaponState,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, FpsControllerInput};
use bevy_rapier3d::prelude::Velocity;
use std::time::Duration;

pub const CLIP_NAMES: &[&str] = &[
    "idle_rifle",
    "walk_forward",
    "walk_backward",
    "run_forward",
    "strafe_left",
    "strafe_right",
    "fire_rifle",
    "reload_rifle",
    "jump",
    "crouch_idle",
    "crouch_walk",
    "death",
];
#[derive(Component)]
pub struct CharacterRig(pub SkinId);
/// Full-body menu playback, excluded from gameplay masking.
#[derive(Component)]
pub struct ShowcaseAnimation;
#[derive(Resource, Default)]
pub struct SharedAnimations {
    pub graph: Handle<AnimationGraph>,
    pub nodes: Vec<AnimationNodeIndex>,
    pub count: usize,
    profiles: Vec<Handle<AnimationGraph>>,
    showcase: Vec<(Handle<AnimationGraph>, AnimationNodeIndex)>,
    lower: Vec<AnimationNodeIndex>,
    upper: Vec<AnimationNodeIndex>,
}
impl SharedAnimations {
    pub fn get_by_index(&self, index: usize) -> AnimationNodeIndex {
        self.nodes
            .get(index)
            .copied()
            .unwrap_or(AnimationNodeIndex::new(0))
    }
}
#[derive(Component)]
pub struct PlayerAnimationController {
    pub animation_player_entity: Option<Entity>,
    pub state: usize,
    previous: usize,
    pub action: u64,
    previous_action: u64,
    locomotion: usize,
    previous_locomotion: usize,
}
impl Default for PlayerAnimationController {
    fn default() -> Self {
        Self {
            animation_player_entity: None,
            state: 0,
            previous: usize::MAX,
            action: 0,
            previous_action: 0,
            locomotion: 0,
            previous_locomotion: usize::MAX,
        }
    }
}

pub fn load_shared_animations(mut commands: Commands) {
    commands.init_resource::<SharedAnimations>();
}
pub fn prepare_graphs(
    assets: Option<Res<GameAssets>>,
    gltfs: Res<Assets<Gltf>>,
    scenes: Res<Assets<Scene>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut shared: ResMut<SharedAnimations>,
) {
    if shared.count > 0 {
        return;
    }
    let Some(assets) = assets else {
        return;
    };
    let mut profiles = Vec::new();
    let mut showcase = Vec::new();
    let mut nodes = Vec::new();
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    for (profile_index, handle) in assets.skins.iter().enumerate() {
        let Some(gltf) = gltfs.get(handle) else {
            return;
        };
        let Some(clips) = CLIP_NAMES
            .iter()
            .map(|name| gltf.named_animations.get(*name).cloned())
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        let Some(scene) = scenes.get(&gltf.scenes[0]) else {
            return;
        };
        let (mut graph, indices) = AnimationGraph::from_clips(clips.clone());
        nodes = indices;
        for entity in scene.world.iter_entities() {
            let Some(target) = entity.get::<bevy::animation::AnimationTarget>() else {
                continue;
            };
            let mut current = entity.id();
            let mut is_upper = false;
            loop {
                if scene
                    .world
                    .get::<Name>(current)
                    .is_some_and(|n| n.as_str() == "mixamorig:Spine")
                {
                    is_upper = true;
                    break;
                }
                let Some(parent) = scene.world.get::<ChildOf>(current) else {
                    break;
                };
                current = parent.parent();
            }
            graph.add_target_to_mask_group(target.id, if is_upper { 0 } else { 1 });
        }
        lower = clips
            .iter()
            .map(|clip| graph.add_clip_with_mask(clip.clone(), 1, 1.0, graph.root))
            .collect();
        upper = [6, 7, 0]
            .iter()
            .map(|index| graph.add_clip_with_mask(clips[*index].clone(), 2, 1.0, graph.root))
            .collect();
        let Some(knife) = gltfs.get(&assets.knife_poses[profile_index]) else {
            return;
        };
        for name in ["idle_knife", "slash_knife", "draw_knife"] {
            let Some(clip) = knife.named_animations.get(name) else {
                return;
            };
            upper.push(graph.add_clip_with_mask(clip.clone(), 2, 1.0, graph.root));
        }
        let clip = gltf
            .named_animations
            .get("menu_hold_rifle")
            .cloned()
            .unwrap_or_else(|| {
                warn!("Missing menu_hold_rifle: showcase using rifle idle fallback");
                clips[0].clone()
            });
        let (menu_graph, menu_node) = AnimationGraph::from_clip(clip);
        showcase.push((graphs.add(menu_graph), menu_node));
        profiles.push(graphs.add(graph));
    }
    shared.graph = profiles[0].clone();
    shared.nodes = nodes;
    shared.count = CLIP_NAMES.len();
    shared.profiles = profiles;
    shared.showcase = showcase;
    shared.lower = lower;
    shared.upper = upper;
}

pub fn setup_animation_player(
    mut commands: Commands,
    shared: Res<SharedAnimations>,
    parents: Query<&ChildOf>,
    mut animations: Query<(Entity, &mut AnimationPlayer)>,
    mut roots: Query<(
        Entity,
        &CharacterRig,
        &mut PlayerAnimationController,
        Option<&ShowcaseAnimation>,
    )>,
) {
    if shared.count == 0 {
        return;
    }
    for (root, rig, mut controller, showcase) in &mut roots {
        if controller.animation_player_entity.is_some() {
            continue;
        }
        for (entity, mut player) in &mut animations {
            let mut current = entity;
            while let Ok(parent) = parents.get(current) {
                current = parent.parent();
                if current == root {
                    let profile = if rig.0 == SkinId::Soldier { 0 } else { 1 };
                    commands.entity(entity).insert((
                        AnimationGraphHandle(if showcase.is_some() {
                            shared.showcase[profile].0.clone()
                        } else {
                            shared.profiles[profile].clone()
                        }),
                        AnimationTransitions::new(),
                    ));
                    if showcase.is_some() {
                        player.stop_all();
                        player.play(shared.showcase[profile].1).repeat();
                    }
                    controller.animation_player_entity = Some(entity);
                    break;
                }
            }
            if controller.animation_player_entity.is_some() {
                break;
            }
        }
    }
}

pub fn detect_animation_state(
    mut models: Query<(&mut PlayerAnimationController, Option<&PlayerModel>)>,
    actors: Query<(
        &Combatant,
        &Velocity,
        &FpsController,
        &FpsControllerInput,
        &WeaponState,
    )>,
) {
    for (mut animation, model) in &mut models {
        let Some(model) = model else {
            continue;
        };
        let Ok((actor, velocity, controller, input, weapon)) = actors.get(model.logical_entity)
        else {
            continue;
        };
        let speed = velocity.linvel.xz().length();
        animation.action = if weapon.active == WeaponId::DefaultKnife {
            weapon.slashes
        } else {
            weapon.shots
        };
        animation.locomotion = if controller.ground_tick == 0 && velocity.linvel.y.abs() > 0.7 {
            8
        } else if input.crouch {
            if speed > 0.3 {
                10
            } else {
                9
            }
        } else if speed < 0.3 {
            0
        } else if input.movement.x < -0.2 {
            4
        } else if input.movement.x > 0.2 {
            5
        } else if input.movement.z < -0.2 {
            2
        } else if input.sprint {
            3
        } else {
            1
        };
        animation.state = if !actor.alive() {
            11
        } else if weapon.active == WeaponId::DefaultKnife {
            if weapon.equip_remaining > 0.0 {
                14
            } else if weapon.knife_remaining > 0.0 {
                13
            } else {
                12
            }
        } else if weapon.reload_remaining > 0.0 {
            7
        } else if weapon.flash_remaining > 0.0 {
            6
        } else {
            animation.locomotion
        };
    }
}
pub fn update_player_animations(
    shared: Res<SharedAnimations>,
    mut controllers: Query<&mut PlayerAnimationController, Without<ShowcaseAnimation>>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    if shared.count == 0 {
        return;
    }
    for mut controller in &mut controllers {
        let Some(entity) = controller.animation_player_entity else {
            continue;
        };
        if controller.state == controller.previous
            && controller.locomotion == controller.previous_locomotion
            && (![6, 13].contains(&controller.state)
                || controller.action == controller.previous_action)
        {
            continue;
        }
        let Ok((mut player, mut transitions)) = players.get_mut(entity) else {
            continue;
        };
        let masked = ![8, 11].contains(&controller.state);
        let was_masked =
            controller.previous != usize::MAX && ![8, 11].contains(&controller.previous);
        if masked {
            if !was_masked {
                player.stop_all();
                *transitions = AnimationTransitions::new();
            }
            if !was_masked || controller.locomotion != controller.previous_locomotion {
                if was_masked {
                    player.stop(shared.lower[controller.previous_locomotion]);
                }
                let active = player.play(shared.lower[controller.locomotion]);
                if controller.locomotion != 8 {
                    active.repeat();
                }
            }
            let upper_index = |state| {
                if state >= 12 {
                    state - 9
                } else if state == 6 {
                    0
                } else if state == 7 {
                    1
                } else {
                    2
                }
            };
            let current = upper_index(controller.state);
            if !was_masked || current != upper_index(controller.previous) {
                if was_masked {
                    player.stop(shared.upper[upper_index(controller.previous)]);
                }
                let active = player.play(shared.upper[current]);
                active.replay();
                if current == 2 || current == 3 {
                    active.repeat();
                }
            }
            if controller.state == 6 && controller.action != controller.previous_action {
                player.play(shared.upper[0]).replay();
            }
            if controller.state == 13 && controller.action != controller.previous_action {
                player
                    .play(shared.upper[4])
                    .set_speed((17.0 / 30.0) / 0.55)
                    .replay();
            }
            if controller.state == 14 {
                player.play(shared.upper[5]).set_speed((11.0 / 30.0) / 0.35);
            }
            if controller.state == 7 {
                player
                    .play(shared.upper[1])
                    .set_speed(3.3 / crate::game::weapons::AK47.reload_seconds);
            }
        } else {
            if was_masked {
                player.stop_all();
            }
            transitions
                .play(
                    &mut player,
                    shared.nodes[controller.state],
                    Duration::from_secs_f32(0.12),
                )
                .replay();
        }
        controller.previous_locomotion = controller.locomotion;
        controller.previous = controller.state;
        controller.previous_action = controller.action;
    }
}
