use super::skins::HitboxZoneType;
use crate::game::matchplay::Combatant;
use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, LogicalPlayer};
use bevy_rapier3d::prelude::Collider;
pub const COLLIDER_HEIGHT: f32 = super::player::BODY_HEIGHT;
#[derive(Component)]
pub struct PlayerModel {
    pub logical_entity: Entity,
    pub is_local_player: bool,
}
#[derive(Component)]
pub struct HitboxZoneMarker {
    pub zone_type: HitboxZoneType,
    pub player_entity: Entity,
}
impl HitboxZoneMarker {
    pub fn damage_multiplier(&self) -> f32 {
        self.zone_type.damage_multiplier()
    }
}
pub fn sync_player_model(
    time: Res<Time<Fixed>>,
    actors: Query<
        (
            &Transform,
            &FpsController,
            &Combatant,
            &super::presentation::PoseHistory,
        ),
        With<LogicalPlayer>,
    >,
    mut models: Query<(&PlayerModel, &mut Transform, &mut Visibility), Without<LogicalPlayer>>,
) {
    for (model, mut transform, mut visibility) in &mut models {
        if let Ok((body, controller, actor, history)) = actors.get(model.logical_entity) {
            let alpha = time.overstep_fraction();
            transform.translation = super::presentation::position(history, body.translation, alpha)
                - Vec3::Y * (history.height + (controller.height - history.height) * alpha) * 0.5;
            transform.rotation = Quat::from_rotation_y(history.yaw + std::f32::consts::PI).slerp(
                Quat::from_rotation_y(controller.yaw + std::f32::consts::PI),
                alpha,
            );
            // The imported character faces +Z; camera forward is -Z.
            *visibility = if model.is_local_player && std::env::var_os("CSRS_OBSERVER").is_none() {
                Visibility::Hidden
            } else if actor.alive() || actor.respawn_remaining > 1.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
pub fn sync_hit_zones(
    actors: Query<&FpsController>,
    mut zones: Query<(&HitboxZoneMarker, &mut Transform, &mut Collider)>,
) {
    for (zone, mut transform, mut collider) in &mut zones {
        if let Ok(controller) = actors.get(zone.player_entity) {
            let profile = zone.zone_type.zone();
            let ratio = controller.height / super::player::BODY_HEIGHT;
            let mut offset = profile.offset;
            offset.y *= ratio;
            transform.translation = offset - Vec3::Y * (controller.height * 0.5);
            transform.rotation = Quat::from_rotation_y(controller.yaw);
            let size = profile.half_extents;
            *collider = Collider::cuboid(
                size.x,
                size.y
                    * if zone.zone_type == HitboxZoneType::Head {
                        1.0
                    } else {
                        ratio
                    },
                size.z,
            );
        }
    }
}

/// Imported bounds describe the bind pose, not the animated skin. At close
/// range those bounds can leave the frustum while the actual torso is visible.
/// Apply this to the mesh entities (a root marker does not affect child culling).
pub fn bind_animated_visibility(
    mut commands: Commands,
    meshes: Query<
        Entity,
        (
            With<bevy::render::mesh::skinning::SkinnedMesh>,
            Without<bevy::render::view::NoFrustumCulling>,
        ),
    >,
    parents: Query<&ChildOf>,
    rigs: Query<(), With<super::animation::CharacterRig>>,
) {
    for mesh in &meshes {
        let mut ancestor = mesh;
        while let Ok(parent) = parents.get(ancestor) {
            ancestor = parent.parent();
            if rigs.contains(ancestor) {
                commands
                    .entity(mesh)
                    .insert(bevy::render::view::NoFrustumCulling);
                break;
            }
        }
    }
}
