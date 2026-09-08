//! Opt-in native contact regression scenario; no OS input injection.
use crate::game::{
    game::SimulationSet,
    matchplay::{ActorIntent, Combatant},
    weapons::WeaponState,
    GameState,
};
use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
use bevy_fps_controller::controller::{FpsControllerInput, LogicalPlayer};
use bevy_rapier3d::prelude::Velocity;
#[derive(Resource, Default)]
struct ContactScenario {
    elapsed: f32,
    anchor: Option<Vec3>,
    checked: bool,
    captured: bool,
}
pub fn install(app: &mut App) {
    if std::env::var_os("CSRS_CONTACT").is_none() {
        return;
    }
    app.init_resource::<ContactScenario>()
        .add_systems(
            FixedUpdate,
            drive
                .after(SimulationSet::Intent)
                .before(SimulationSet::Movement)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, verify.run_if(in_state(GameState::Playing)));
}
fn drive(
    time: Res<Time<Fixed>>,
    mut scenario: ResMut<ContactScenario>,
    mut actors: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut FpsControllerInput,
            &mut ActorIntent,
            &mut Combatant,
        ),
        With<LogicalPlayer>,
    >,
) {
    if actors.is_empty() {
        return;
    }
    let initial = scenario.anchor.is_none();
    if initial {
        scenario.anchor = actors
            .iter()
            .find(|(_, _, _, _, a)| a.slot == 0)
            .map(|(t, _, _, _, _)| t.translation);
    }
    let Some(anchor) = scenario.anchor else {
        return;
    };
    scenario.elapsed += time.delta_secs();
    for (mut body, mut velocity, mut input, mut intent, mut actor) in &mut actors {
        input.movement = Vec3::ZERO;
        input.jump = false;
        input.crouch = false;
        input.yaw = 0.;
        input.pitch = 0.;
        intent.fire = false;
        intent.reload = false;
        actor.protection_remaining = 0.;
        if initial {
            body.translation = anchor
                + match actor.slot {
                    0 => Vec3::ZERO,
                    1 => Vec3::new(0., 0., -1.2),
                    3 => Vec3::new(0., 0., -3.),
                    slot => Vec3::new(5. + slot as f32, 0., 0.),
                };
            velocity.linvel = Vec3::ZERO;
        }
        if actor.slot == 0 {
            let t = scenario.elapsed;
            if t < 5.5 {
                input.movement.z = 1.;
            }
            if (3.5..4.5).contains(&t) {
                intent.fire = true;
            }
            if (4.5..5.5).contains(&t) {
                input.crouch = true;
            }
            if (5.5..6.5).contains(&t) {
                input.movement.z = -1.;
            }
            if t >= 6.5 {
                input.movement.x = 1.;
            }
        }
    }
}
fn verify(
    mut commands: Commands,
    mut scenario: ResMut<ContactScenario>,
    actors: Query<(&Transform, &Combatant, &WeaponState), With<LogicalPlayer>>,
    meshes: Query<
        (),
        (
            With<bevy::render::mesh::skinning::SkinnedMesh>,
            With<bevy::render::view::NoFrustumCulling>,
        ),
    >,
    mut exit: EventWriter<AppExit>,
) {
    let Some(anchor) = scenario.anchor else {
        return;
    };
    let Some((local, _, weapon)) = actors.iter().find(|(_, a, _)| a.slot == 0) else {
        return;
    };
    let (mate, friend, _) = actors.iter().find(|(_, a, _)| a.slot == 1).unwrap();
    let (_, enemy, _) = actors.iter().find(|(_, a, _)| a.slot == 3).unwrap();
    if scenario.elapsed > 3. && !scenario.captured {
        info!(
            "CONTACT near: separation {:.4}, teammate displacement {:.4}, protected meshes {}",
            local.translation.xz().distance(mate.translation.xz()),
            mate.translation
                .xz()
                .distance((anchor - Vec3::Z * 1.2).xz()),
            meshes.iter().count()
        );
        assert!(
            local.translation.xz().distance(mate.translation.xz()) >= 0.595,
            "Actor overlap"
        );
        assert!(
            mate.translation
                .xz()
                .distance((anchor - Vec3::Z * 1.2).xz())
                < 0.02,
            "Player pushed stationary teammate"
        );
        assert!(
            meshes.iter().count() >= 6,
            "Missing animated mesh culling protection"
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/private/tmp/csrs-contact-near.png"));
        scenario.captured = true;
    }
    if scenario.elapsed > 5. && !scenario.checked {
        assert!(weapon.shots > 0);
        assert_eq!(friend.health, 100.);
        assert_eq!(friend.armor, 100.);
        assert_eq!(enemy.health, 100., "Shots passed through teammate");
        assert!(
            local.translation.xz().distance(mate.translation.xz()) >= 0.595,
            "Crouching overlapped teammate"
        );
        info!(
            "CONTACT shooting/crouch: {} shots, friend health {}, enemy behind health {}",
            weapon.shots, friend.health, enemy.health
        );
        scenario.checked = true;
    }
    if scenario.elapsed > 8. {
        assert!(
            local.translation.xz().distance(mate.translation.xz()) > 1.5,
            "Cannot move away from teammate"
        );
        info!("CONTACT passed: blocked movement, no pushing, friendly-fire protection, blocked shots, crouch and moving away");
        exit.write(AppExit::Success);
    }
}
