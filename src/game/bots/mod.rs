//! Local AI and future remote inputs share controller and weapon intent contracts.
use crate::game::{
    game::SimulationSet,
    matchplay::{ActorIntent, Combatant},
    weapons::WeaponState,
    GameState,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::FpsControllerInput;
use bevy_rapier3d::prelude::*;
pub mod navigation;
use navigation::Navigation;
#[derive(Component)]
pub struct BotController {
    pub slot: usize,
    reaction: f32,
    pub target: Option<Entity>,
    pub path: Vec<usize>,
    pub waypoint: usize,
    patrol: usize,
    repath_remaining: f32,
}
impl BotController {
    pub fn new(slot: usize) -> Self {
        Self {
            slot,
            reaction: 0.0,
            target: None,
            path: Vec::new(),
            waypoint: 0,
            patrol: slot,
            repath_remaining: 0.0,
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct NavNode {
    pub position: crate::game::map::Vec3Config,
}
pub struct BotPlugin;
impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>().add_systems(
            FixedUpdate,
            think
                .in_set(SimulationSet::Intent)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn think(
    time: Res<Time>,
    nav: Res<Navigation>,
    context: ReadRapierContext,
    actors: Query<(Entity, &Transform, &Combatant)>,
    mut bots: Query<(
        Entity,
        &Transform,
        &Combatant,
        &mut BotController,
        &mut FpsControllerInput,
        &mut ActorIntent,
        &WeaponState,
    )>,
) {
    if nav.positions.is_empty() || nav.patrol.is_empty() {
        return;
    }
    let Ok(physics) = context.single() else {
        return;
    };
    for (entity, transform, actor, mut bot, mut input, mut intent, weapon) in &mut bots {
        intent.fire = false;
        input.movement = Vec3::ZERO;
        input.jump = false;
        input.crouch = false;
        input.sprint = false;
        if !actor.alive() {
            bot.target = None;
            bot.path.clear();
            continue;
        }
        let eye = transform.translation + Vec3::Y * 0.65;
        let target = actors
            .iter()
            .filter(|(e, _, a)| *e != entity && a.alive() && a.team != actor.team)
            .filter_map(|(e, t, _)| {
                let point = t.translation + Vec3::Y * 0.25;
                let distance = point.distance(eye);
                if distance > 65.0 {
                    return None;
                }
                let visible = physics
                    .cast_ray(
                        eye,
                        (point - eye).normalize_or_zero(),
                        distance,
                        true,
                        QueryFilter::default()
                            .exclude_sensors()
                            .exclude_rigid_body(entity),
                    )
                    .is_none_or(|(hit, _)| hit == e);
                visible.then_some((e, point, distance))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let Some((target, point, _)) = target {
            if bot.target != Some(target) {
                bot.reaction = 0.35 + bot.slot as f32 * 0.025;
                bot.target = Some(target);
            }
            bot.reaction -= time.delta_secs();
            let aim = (point - eye).normalize_or_zero();
            let desired = (-aim.x).atan2(-aim.z);
            let error = (desired - input.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            input.yaw += error.clamp(-3.0 * time.delta_secs(), 3.0 * time.delta_secs());
            input.pitch = aim.y.asin();
            intent.fire = bot.reaction <= 0.0 && error.abs() < 0.06;
            intent.reload = weapon.magazine == 0;
            if weapon.muzzle_blocked {
                input.movement.x = if bot.slot % 2 == 0 { 0.07 } else { -0.07 };
            }
            // Resume route from current ground position after a fight.
            bot.path.clear();
        } else {
            bot.target = None;
            intent.reload = weapon.magazine < 10;
            let feet = transform.translation - Vec3::Y * 0.9;
            bot.repath_remaining -= time.delta_secs();
            let occupants: Vec<_> = actors
                .iter()
                .filter(|(other, _, a)| *other != entity && a.alive())
                .map(|(_, t, _)| t.translation - Vec3::Y * 0.9)
                .collect();
            if bot.repath_remaining <= 0.0 && bot.waypoint < bot.path.len() {
                if let (Some(start), Some(&goal)) = (nav.nearest(feet), bot.path.last()) {
                    let path = nav.path_avoiding(start, goal, &occupants);
                    if path.len() > 1 {
                        bot.path = path;
                        bot.waypoint = 1;
                    }
                }
                bot.repath_remaining = 2.0 + bot.slot as f32 * 0.1;
            }
            if bot.waypoint >= bot.path.len() {
                let Some(start) = nav.nearest(feet) else {
                    continue;
                };
                for _ in 0..nav.destinations.len() {
                    let index = nav.patrol[bot.patrol % nav.patrol.len()];
                    bot.patrol += 1;
                    let path = nav.path_avoiding(start, nav.destinations[index], &occupants);
                    if path.len() > 1 {
                        bot.path = path;
                        bot.waypoint = 1;
                        bot.repath_remaining = 2.0 + bot.slot as f32 * 0.1;
                        break;
                    }
                }
            }
            if let Some(&node) = bot.path.get(bot.waypoint) {
                let delta = nav.positions[node] - feet;
                let distance = delta.xz().length();
                if distance < 0.24 {
                    bot.waypoint += 1;
                    continue;
                }
                let mut direction = delta.with_y(0.0).normalize_or_zero();
                // Local passing rule: both actors pass on their own right. Cast
                // the detour against the map before leaving the graph segment.
                if actors.iter().any(|(other, t, a)| {
                    other != entity
                        && a.alive()
                        && (t.translation - transform.translation).xz().length() < 1.4
                        && (t.translation - transform.translation).dot(direction) > 0.0
                        && (t.translation - transform.translation)
                            .dot(direction.cross(Vec3::Y))
                            .abs()
                            < 0.7
                }) {
                    let right = direction.cross(Vec3::Y);
                    let detour = (direction * 0.35 + right).normalize();
                    let shape = Collider::cylinder(0.75, 0.32);
                    let predicate = |e| !actors.contains(e);
                    if physics
                        .cast_shape(
                            transform.translation,
                            Quat::IDENTITY,
                            detour,
                            &shape,
                            ShapeCastOptions::with_max_time_of_impact(0.65),
                            QueryFilter::default()
                                .exclude_sensors()
                                .predicate(&predicate),
                        )
                        .is_none()
                    {
                        direction = detour;
                    }
                }
                input.yaw = (-direction.x).atan2(-direction.z);
                input.pitch = 0.0;
                input.movement.z = (distance * 3.0).min(4.0) / 30.0;
            }
        }
    }
}
