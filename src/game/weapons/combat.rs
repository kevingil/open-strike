use super::{ShotFired, WeaponSelection, WeaponState, AK47, KNIFE};
use crate::game::{
    config::{GameConfig, WeaponId},
    level::targets::{DeadTarget, Target},
    matchplay::{ActorIntent, Combatant, MatchSession},
    player::{
        player::{LocalPlayer, PlayerEntity},
        player_model::HitboxZoneMarker,
    },
    shooting::tracer::BulletTracer,
};
use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, FpsControllerInput};
use bevy_rapier3d::prelude::*;

pub fn simulate_weapons(
    time: Res<Time>,
    context: ReadRapierContext,
    config: Res<GameConfig>,
    mut commands: Commands,
    mut shots: EventWriter<ShotFired>,
    mut session: ResMut<MatchSession>,
    mut kills: EventWriter<crate::game::matchplay::KillNotice>,
    mut actors: Query<(
        Entity,
        &Transform,
        &mut Combatant,
        &mut ActorIntent,
        &mut WeaponState,
        &FpsController,
        &mut FpsControllerInput,
        Option<&LocalPlayer>,
    )>,
    zones: Query<&HitboxZoneMarker>,
    targets: Query<(), With<Target>>,
) {
    let Ok(physics) = context.single() else {
        return;
    };
    let mut damage = Vec::new();
    let dt = time.delta_secs();
    let living: Vec<_> = actors
        .iter()
        .filter(|(_, _, a, ..)| a.alive())
        .map(|(e, _, a, ..)| (e, a.team, a.protection_remaining))
        .collect();
    for (entity, transform, mut actor, mut intent, mut weapon, controller, mut input, local) in
        &mut actors
    {
        weapon.cooldown = (weapon.cooldown - dt).max(0.0);
        weapon.flash_remaining = (weapon.flash_remaining - dt).max(0.0);
        if !actor.alive() {
            weapon.reload_remaining = 0.0;
            weapon.knife_remaining = 0.0;
            weapon.equip_remaining = 0.0;
            *intent = ActorIntent::default();
            continue;
        }
        if let Some(selection) = intent.selection.take() {
            let target = match selection {
                WeaponSelection::Select(id) => id,
                WeaponSelection::Previous => weapon.previous,
            };
            if target != weapon.active && weapon.knife_remaining == 0.0 {
                weapon.previous = weapon.active;
                weapon.active = target;
                weapon.equips += 1;
                weapon.equip_remaining = KNIFE.equip_seconds;
                if weapon.reload_remaining > 0.0 {
                    weapon.reload_cancellations += 1;
                }
                weapon.reload_remaining = 0.0;
                weapon.flash_remaining = 0.0;
                weapon.muzzle_blocked = false;
                intent.reload = false;
            }
        }
        if weapon.equip_remaining > 0.0 {
            weapon.equip_remaining = (weapon.equip_remaining - dt).max(0.0);
            intent.reload = false;
            continue;
        }
        if weapon.active == WeaponId::DefaultKnife {
            intent.reload = false;
            if weapon.knife_remaining > 0.0 {
                let previous = weapon.knife_remaining;
                weapon.knife_remaining = (previous - dt).max(0.0);
                // Exactly one impact at the windup/recovery boundary.
                if previous > KNIFE.recovery && weapon.knife_remaining <= KNIFE.recovery {
                    let rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
                    let origin = transform.translation + Vec3::Y * (controller.height * 0.5 - 0.15);
                    let predicate = |hit: Entity| {
                        if hit == entity {
                            return false;
                        }
                        if let Ok(zone) = zones.get(hit) {
                            return zone.player_entity != entity
                                && living.iter().any(|(e, _, _)| *e == zone.player_entity);
                        }
                        !living.iter().any(|(e, _, _)| *e == hit)
                    };
                    if let Some((hit, _)) = physics.cast_ray(
                        origin,
                        rotation * -Vec3::Z,
                        KNIFE.range,
                        true,
                        QueryFilter::default()
                            .exclude_rigid_body(entity)
                            .predicate(&predicate),
                    ) {
                        if let Ok(zone) = zones.get(hit) {
                            if living.iter().any(|(e, team, protection)| {
                                *e == zone.player_entity
                                    && *team != actor.team
                                    && *protection <= 0.0
                            }) {
                                damage.push((
                                    entity,
                                    zone.player_entity,
                                    KNIFE.damage,
                                    WeaponId::DefaultKnife,
                                    false,
                                ));
                            }
                        } else if targets.contains(hit) {
                            commands.entity(hit).insert(DeadTarget);
                        }
                    }
                }
            } else if intent.fire {
                weapon.knife_remaining = KNIFE.windup + KNIFE.recovery;
                weapon.slashes += 1;
                actor.protection_remaining = 0.0;
            }
            continue;
        }
        if weapon.reload_remaining > 0.0 {
            weapon.reload_remaining = (weapon.reload_remaining - dt).max(0.0);
            if weapon.reload_remaining == 0.0 {
                let count = (AK47.magazine - weapon.magazine).min(weapon.reserve);
                weapon.magazine += count;
                weapon.reserve -= count;
            }
            intent.reload = false;
            continue;
        }
        if intent.reload && weapon.magazine < AK47.magazine && weapon.reserve > 0 {
            weapon.reload_remaining = AK47.reload_seconds;
            intent.reload = false;
            continue;
        }
        intent.reload = false;
        if !intent.fire || weapon.cooldown > 0.00001 {
            continue;
        }
        if weapon.magazine == 0 {
            if weapon.reserve > 0 {
                weapon.reload_remaining = AK47.reload_seconds;
            }
            weapon.cooldown = AK47.interval;
            continue;
        }
        weapon.magazine -= 1;
        weapon.cooldown = AK47.interval;
        weapon.flash_remaining = 0.065;
        weapon.shots += 1;
        actor.protection_remaining = 0.0;
        let rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
        let origin = transform.translation + Vec3::Y * (controller.height * 0.5 - 0.15);
        let direction = rotation * -Vec3::Z;
        let muzzle = origin + rotation * Vec3::new(0.22, -0.20, -0.90);
        let predicate = |hit: Entity| {
            if hit == entity {
                return false;
            }
            if let Ok(zone) = zones.get(hit) {
                return zone.player_entity != entity
                    && living.iter().any(|(e, _, _)| *e == zone.player_entity);
            }
            // Actor bodies are for movement; separate standardized zones resolve damage.
            !living.iter().any(|(e, _, _)| *e == hit)
        };
        let filter = QueryFilter::default()
            .exclude_rigid_body(entity)
            .predicate(&predicate);
        let aim_hit = physics.cast_ray(origin, direction, AK47.range, true, filter);
        let aim = origin + direction * aim_hit.map(|(_, toi)| toi).unwrap_or(AK47.range);
        let muzzle_dir = (aim - muzzle).normalize_or_zero();
        let obstructed = physics.cast_ray(
            muzzle,
            muzzle_dir,
            muzzle.distance(aim) + 0.01,
            true,
            filter,
        );
        let barrel_delta = muzzle - origin;
        let barrel_hit = physics.cast_ray(
            origin,
            barrel_delta.normalize_or_zero(),
            barrel_delta.length(),
            true,
            filter,
        );
        weapon.muzzle_blocked = barrel_hit.is_some()
            || obstructed.is_some_and(|(e, _)| zones.get(e).is_err() && !targets.contains(e));
        let hit = barrel_hit.or(obstructed).or(aim_hit);
        let end = if let Some((_, toi)) = barrel_hit {
            origin + barrel_delta.normalize_or_zero() * toi
        } else {
            obstructed
                .map(|(_, toi)| muzzle + muzzle_dir * toi)
                .unwrap_or(aim)
        };
        if let Some((hit, _)) = hit {
            if let Ok(zone) = zones.get(hit) {
                if let Some((_, team, protection)) =
                    living.iter().find(|(e, _, _)| *e == zone.player_entity)
                {
                    if *team != actor.team && *protection <= 0.0 {
                        damage.push((
                            entity,
                            zone.player_entity,
                            AK47.damage * zone.damage_multiplier(),
                            WeaponId::AK47,
                            zone.zone_type == crate::game::player::skins::HitboxZoneType::Head,
                        ));
                    }
                }
            } else if targets.contains(hit) {
                commands.entity(hit).insert(DeadTarget);
            }
        }
        shots.write(ShotFired {
            actor: entity,
            origin: muzzle,
            end,
        });
        if local.is_some() {
            input.pitch = (input.pitch + AK47.recoil).min(1.52);
        }
    }
    for (shooter, victim, amount, weapon_id, headshot) in damage {
        let mut killed = false;
        let mut victim_name = String::new();
        let mut victim_team = crate::game::matchplay::Team::Attacker;
        if let Ok((_, _, mut actor, _, mut victim_weapon, _, _, _)) = actors.get_mut(victim) {
            if !actor.alive() {
                continue;
            }
            let absorbed = (amount * 0.25).min(actor.armor);
            actor.armor -= absorbed;
            actor.health = (actor.health - (amount - absorbed)).max(0.0);
            if !actor.alive() {
                actor.deaths += 1;
                actor.respawn_remaining = config.match_settings.respawn_time.as_secs_f32();
                killed = true;
                victim_name = actor.name.clone();
                victim_team = actor.team;
                victim_weapon.reload_remaining = 0.0;
                victim_weapon.knife_remaining = 0.0;
                victim_weapon.reload_cancellations += 1;
                commands.entity(victim).insert(ColliderDisabled);
            }
        }
        if killed {
            if let Ok((_, _, mut actor, ..)) = actors.get_mut(shooter) {
                actor.kills += 1;
                session.score[actor.team.index()] += 1;
                kills.write(crate::game::matchplay::KillNotice {
                    killer_entity: shooter,
                    victim_entity: victim,
                    victim_team,
                    weapon: weapon_id,
                    headshot,
                    killer: actor.name.clone(),
                    victim: victim_name,
                    team: actor.team,
                });
            }
            info!(
                "Kill: {:?} -> {:?}; score {:?}",
                shooter, victim, session.score
            );
        }
    }
}
#[derive(Resource)]
pub struct TracerAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
pub fn shot_effects(
    mut commands: Commands,
    mut shots: EventReader<ShotFired>,
    assets: Option<Res<TracerAssets>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (mesh, material) = if let Some(assets) = assets {
        (assets.mesh.clone(), assets.material.clone())
    } else {
        let mesh = meshes.add(Cuboid::new(0.015, 0.015, 0.6));
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.75, 0.25),
            unlit: true,
            ..default()
        });
        commands.insert_resource(TracerAssets {
            mesh: mesh.clone(),
            material: material.clone(),
        });
        (mesh, material)
    };
    for shot in shots.read() {
        if shot.origin.distance_squared(shot.end) < 0.001 {
            continue;
        }
        commands.spawn((
            PlayerEntity,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(shot.origin).looking_at(shot.end, Vec3::Y),
            BulletTracer::new(shot.origin, shot.end, 220.0),
        ));
    }
}
