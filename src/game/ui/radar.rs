//! Local rotating radar drawn from the collision-validated walkable map.
use crate::game::{
    bots::navigation::Navigation, matchplay::Combatant, player::player::LocalPlayer, GameState,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_fps_controller::controller::FpsControllerInput;
const SIZE: usize = 256;
#[derive(Resource)]
pub struct HudImages {
    pub radar: Handle<Image>,
    pub shield: Handle<Image>,
}
pub struct RadarPlugin;
impl Plugin for RadarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, prepare)
            .add_systems(Update, update.run_if(in_state(GameState::Playing)));
    }
}
fn texture(width: usize, height: usize, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}
fn icon(width: usize, height: usize, polygons: &[&[(f32, f32)]]) -> Image {
    let mut data = vec![0; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let point = Vec2::new(x as f32 / width as f32, y as f32 / height as f32);
            if polygons.iter().any(|polygon| {
                let mut inside = false;
                let mut previous = polygon.len() - 1;
                for current in 0..polygon.len() {
                    let (ax, ay) = polygon[current];
                    let (bx, by) = polygon[previous];
                    if (ay > point.y) != (by > point.y)
                        && point.x < (bx - ax) * (point.y - ay) / (by - ay) + ax
                    {
                        inside = !inside;
                    }
                    previous = current;
                }
                inside
            }) {
                data[(y * width + x) * 4..(y * width + x) * 4 + 4]
                    .copy_from_slice(&[8, 220, 211, 255]);
            }
        }
    }
    texture(width, height, data)
}
pub(super) fn prepare(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let shield = icon(
        32,
        40,
        &[&[
            (0.16, 0.06),
            (0.84, 0.06),
            (0.90, 0.50),
            (0.76, 0.77),
            (0.50, 0.98),
            (0.24, 0.77),
            (0.10, 0.50),
        ]],
    );
    commands.insert_resource(HudImages {
        radar: images.add(texture(SIZE, SIZE, vec![0; SIZE * SIZE * 4])),
        shield: images.add(shield),
    });
}
fn update(
    time: Res<Time>,
    mut last: Local<f32>,
    nav: Res<Navigation>,
    handles: Res<HudImages>,
    mut images: ResMut<Assets<Image>>,
    player: Query<(&Transform, &Combatant, &FpsControllerInput), With<LocalPlayer>>,
    actors: Query<(&Transform, &Combatant)>,
) {
    if time.elapsed_secs() - *last < 0.1 {
        return;
    }
    *last = time.elapsed_secs();
    let Ok((body, local, input)) = player.single() else {
        return;
    };
    let Some(image) = images.get_mut(&handles.radar) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let center = SIZE as f32 * 0.5;
    let radius = center - 3.0;
    let pixels_per_meter = 3.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = Vec2::new(x as f32 - center, y as f32 - center).length();
            let color = if d > radius + 2.0 {
                [0, 0, 0, 0]
            } else if d > radius {
                [210, 220, 220, 190]
            } else if x % 24 == 0 || y % 24 == 0 {
                [48, 50, 46, 220]
            } else {
                [15, 20, 19, 215]
            };
            data[(y * SIZE + x) * 4..(y * SIZE + x) * 4 + 4].copy_from_slice(&color);
        }
    }
    let rotation = Quat::from_rotation_y(-input.yaw);
    let project = |p: Vec3| {
        let p = rotation * (p - body.translation);
        Vec2::new(
            center + p.x * pixels_per_meter,
            center + p.z * pixels_per_meter,
        )
    };
    let mut dot = |point: Vec2, half: i32, color: [u8; 4]| {
        for dy in -half..=half {
            for dx in -half..=half {
                let x = point.x.round() as i32 + dx;
                let y = point.y.round() as i32 + dy;
                if x < 0
                    || y < 0
                    || x >= SIZE as i32
                    || y >= SIZE as i32
                    || (Vec2::new(x as f32, y as f32) - Vec2::splat(center)).length() > radius - 1.0
                {
                    continue;
                }
                data[(y as usize * SIZE + x as usize) * 4
                    ..(y as usize * SIZE + x as usize) * 4 + 4]
                    .copy_from_slice(&color);
            }
        }
    };
    for position in &nav.positions {
        if (position.y - body.translation.y).abs() > 5.0 {
            continue;
        }
        let point = project(*position);
        if (point - Vec2::splat(center)).length() < radius {
            dot(
                point,
                1,
                if position.y > body.translation.y {
                    [174, 154, 101, 235]
                } else {
                    [131, 116, 73, 235]
                },
            );
        }
    }
    for (transform, actor) in &actors {
        if actor.alive() && actor.team == local.team {
            let point = project(transform.translation);
            dot(point, 4, [20, 24, 20, 255]);
            dot(point, 3, [232, 211, 83, 255]);
        }
    }
    for offset in 0..8 {
        dot(
            Vec2::new(center, center - 8.0 + offset as f32),
            offset / 3,
            [235, 250, 250, 255],
        );
    }
    dot(Vec2::splat(center), 3, [255, 255, 255, 255]);
}
