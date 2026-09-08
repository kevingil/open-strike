use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::*;
use rngs::ThreadRng;

use super::level::LevelEntity;
use crate::game::player::player_shooting::Shootable;
use crate::game::GameState;

pub struct TargetsPlugin;
impl Plugin for TargetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_targets
                .run_if(in_state(GameState::Playing))
                .run_if(resource_exists::<GridShot>),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            init_grid_shot.run_if(
                |config: Res<crate::game::config::GameConfig>,
                 existing: Query<(), With<Target>>| {
                    config.mode == crate::game::config::GameMode::Freemode && existing.is_empty()
                },
            ),
        );
    }
}
#[derive(Component)]
pub struct Target;

#[derive(Component)]
pub struct DeadTarget;

#[derive(Resource, Clone, Copy)]
pub struct GridShot {
    pub grid_size: i32,
    pub cell_size: f32,
    pub max_targets: i32,
}

impl GridShot {
    pub fn generate_new_position(&self, rand: &mut ThreadRng) -> Vec2 {
        return (Vec2::new(
            rand.gen_range(0..self.grid_size) as f32,
            rand.gen_range(0..self.grid_size) as f32,
        ) - Vec2::new(self.grid_size as f32 / 2., 0.)
            + (Vec2::Y * 0.5))
            * self.cell_size;
    }
}

fn init_grid_shot(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let grid_shot = GridShot {
        grid_size: 5,
        cell_size: 5.0,
        max_targets: 5,
    };

    let target_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 0., 0.),
        ..default()
    });

    commands.insert_resource(grid_shot);

    let target_radius = grid_shot.cell_size / 8.;

    let collider_radius = target_radius * f32::sin(PI / 4.);

    for _i in 0..grid_shot.max_targets {
        commands.spawn((
            LevelEntity,
            Collider::cuboid(collider_radius, collider_radius, collider_radius),
            Mesh3d(meshes.add(Sphere::new(target_radius))),
            MeshMaterial3d(target_material.clone()),
            Transform::from_xyz(0., 0., -40.),
            Target {},
            Shootable,
            DeadTarget,
        ));
    }
}

fn update_targets(
    gridshot: Res<GridShot>,
    mut commands: Commands,
    mut dead_targets: Query<(Entity, &mut Transform), (With<DeadTarget>, With<Target>)>,
    alive_targets: Query<&Transform, (With<Target>, Without<DeadTarget>)>,
) {
    let mut alive_target_positions = Vec::new();
    let mut rand = thread_rng();

    for transform in alive_targets.iter() {
        alive_target_positions.push(transform.translation.xy());
    }
    for (entity, mut transform) in dead_targets.iter_mut() {
        let mut found_spot = false;
        let old_position = transform.translation.xy();
        let mut new_position = gridshot.generate_new_position(&mut rand);

        while !found_spot {
            found_spot = true;
            //make sure we don't reset to same position
            while new_position == old_position {
                new_position = gridshot.generate_new_position(&mut rand);
            }
            for position in alive_target_positions.iter() {
                if *position == new_position {
                    found_spot = false;
                    new_position = gridshot.generate_new_position(&mut rand);
                    break;
                }
            }
        }
        commands.entity(entity).remove::<DeadTarget>();
        transform.translation.x = new_position.x;
        transform.translation.y = new_position.y;
        alive_target_positions.push(new_position);
    }
}
