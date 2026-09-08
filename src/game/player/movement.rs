//! Small adapter around the existing controller: standing requires actual headroom.
use bevy::prelude::*;
use bevy_fps_controller::controller::{FpsController, FpsControllerInput};
use bevy_rapier3d::prelude::*;
pub fn guard_standing(
    context: ReadRapierContext,
    mut actors: Query<(Entity, &Transform, &FpsController, &mut FpsControllerInput)>,
) {
    let Ok(physics) = context.single() else {
        return;
    };
    for (entity, transform, controller, mut input) in &mut actors {
        if input.crouch || controller.height >= controller.upright_height - 0.01 {
            continue;
        }
        let center =
            transform.translation + Vec3::Y * (controller.upright_height - controller.height) * 0.5;
        let shape = Collider::cylinder(
            controller.upright_height * 0.5 - 0.04,
            controller.radius * 0.98,
        );
        let mut blocked = false;
        physics.intersections_with_shape(
            center,
            Quat::IDENTITY,
            &shape,
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(entity),
            |_| {
                blocked = true;
                false
            },
        );
        if blocked {
            input.crouch = true;
        }
    }
}

/// Sweep solid actor bodies before Rapier integrates them. Contacts remove only
/// closing motion, so a player stops against a stationary teammate instead of
/// pushing that dynamic body along. Tangential motion and moving apart remain free.
pub fn block_actor_motion(
    time: Res<Time<Fixed>>,
    mut actors: Query<
        (
            &Transform,
            &Collider,
            &crate::game::matchplay::Combatant,
            &mut Velocity,
        ),
        Without<ColliderDisabled>,
    >,
) {
    use bevy_rapier3d::{na, parry::query::cast_shapes};
    let dt = time.delta_secs();
    // Revisit constraints when one actor touches several others. Each pass only
    // removes closing velocity; the six-actor match bounds the solver's work.
    for _ in 0..actors.iter().count() {
        let mut pairs = actors.iter_combinations_mut();
        let mut changed = false;
        while let Some([(a, shape_a, actor_a, mut va), (b, shape_b, actor_b, mut vb)]) =
            pairs.fetch_next()
        {
            if !actor_a.alive() || !actor_b.alive() {
                continue;
            }
            let pos_a =
                na::Isometry3::translation(a.translation.x, a.translation.y, a.translation.z);
            let pos_b =
                na::Isometry3::translation(b.translation.x, b.translation.y, b.translation.z);
            let options = ShapeCastOptions {
                max_time_of_impact: dt,
                target_distance: 0.01,
                stop_at_penetration: false,
                ..default()
            };
            let Ok(Some(hit)) = cast_shapes(
                &pos_a,
                &va.linvel.into(),
                shape_a.raw.as_ref(),
                &pos_b,
                &vb.linvel.into(),
                shape_b.raw.as_ref(),
                options,
            ) else {
                continue;
            };
            // Upright cylinders have locked rotation. Vertical support is owned
            // by Rapier; only their horizontal blocking constraint is added here.
            let normal = Vec3::new(hit.normal1.x, 0.0, hit.normal1.z).normalize_or_zero();
            let inward_a = va.linvel.dot(normal);
            let inward_b = -vb.linvel.dot(normal);
            let closing = inward_a + inward_b;
            if closing <= 0.00001 {
                continue;
            }
            let correction = closing * (1.0 - hit.time_of_impact / dt).clamp(0.0, 1.0);
            let weight_a = inward_a.max(0.0) / (inward_a.max(0.0) + inward_b.max(0.0));
            va.linvel -= normal * correction * weight_a;
            vb.linvel += normal * correction * (1.0 - weight_a);
            changed |= correction > 0.00001;
        }
        if !changed {
            break;
        }
    }
}
