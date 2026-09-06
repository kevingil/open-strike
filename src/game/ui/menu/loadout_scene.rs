//! Live, independently lit character scenes for the loadout's two selectors.
use super::{loadout_tab::CharacterViewport, MenuTab};
use crate::game::{
    player::{
        animation::{CharacterRig, PlayerAnimationController, ShowcaseAnimation},
        skins::{PlayerSide, SkinId, SkinRegistry},
    },
    GameState,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        camera::{ClearColorConfig, Exposure, RenderTarget},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

#[derive(Resource)]
pub(super) struct CharacterPreviews {
    attacker: Handle<Image>,
    defender: Handle<Image>,
}

impl CharacterPreviews {
    pub fn target(&self, side: PlayerSide) -> Handle<Image> {
        if side == PlayerSide::Defender {
            self.defender.clone()
        } else {
            self.attacker.clone()
        }
    }
}

#[derive(Component)]
struct LoadoutScene;
#[derive(Component)]
struct PreviewCamera(PlayerSide);
#[derive(Component)]
struct PreviewCharacter(RenderLayers);

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        spawn
            .run_if(in_state(GameState::MainMenu))
            .run_if(|cameras: Query<(), With<PreviewCamera>>| cameras.is_empty()),
    )
    .add_systems(OnExit(GameState::MainMenu), cleanup)
    .add_systems(Update, visibility.run_if(in_state(GameState::MainMenu)))
    .add_systems(
        PostUpdate,
        (
            bind_layers.before(bevy::render::view::VisibilitySystems::CheckVisibility),
            resize.after(bevy::ui::UiSystem::PostLayout),
        )
            .run_if(in_state(GameState::MainMenu)),
    );
}

pub(super) fn setup_targets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut target = || {
        let mut image = Image::new_fill(
            Extent3d {
                width: 512,
                height: 768,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        images.add(image)
    };
    commands.insert_resource(CharacterPreviews {
        attacker: target(),
        defender: target(),
    });
}

fn spawn(
    mut commands: Commands,
    previews: Res<CharacterPreviews>,
    server: Res<AssetServer>,
    skins: Res<SkinRegistry>,
) {
    for (side, skin, layer, order, yaw) in [
        (PlayerSide::Defender, SkinId::Police, 2, -2, 0.18),
        (PlayerSide::Attacker, SkinId::Soldier, 3, -1, -0.18),
    ] {
        let layers = RenderLayers::layer(layer);
        commands.spawn((
            LoadoutScene,
            PreviewCamera(side),
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(previews.target(side).into()),
                // Render before the main scene; menu glass only processes the window camera.
                order,
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 45_f32.to_radians(),
                ..default()
            }),
            Exposure { ev100: 13.0 },
            Transform::from_xyz(0., 1.05, 2.65).looking_at(Vec3::new(0., 0.96, 0.), Vec3::Y),
            layers.clone(),
        ));
        commands.spawn((
            LoadoutScene,
            PreviewCharacter(layers.clone()),
            CharacterRig(skin),
            ShowcaseAnimation,
            PlayerAnimationController::default(),
            SceneRoot(
                server.load(
                    skins
                        .get(skin)
                        .expect("Default character is registered")
                        .model_path,
                ),
            ),
            Transform::from_rotation(Quat::from_rotation_y(yaw)),
            Visibility::Hidden,
            layers.clone(),
        ));
        commands.spawn((
            LoadoutScene,
            DirectionalLight {
                illuminance: 65000.,
                color: Color::srgb(1., 0.94, 0.84),
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(-2., 4., 4.).looking_at(Vec3::ZERO, Vec3::Y),
            layers,
        ));
    }
}

fn visibility(
    tab: Res<State<MenuTab>>,
    mut cameras: Query<&mut Camera, With<PreviewCamera>>,
    mut characters: Query<&mut Visibility, With<PreviewCharacter>>,
) {
    let active = *tab.get() == MenuTab::LoadOut;
    for mut camera in &mut cameras {
        camera.is_active = active;
    }
    for mut visibility in &mut characters {
        *visibility = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn bind_layers(
    mut commands: Commands,
    roots: Query<(Entity, &PreviewCharacter)>,
    children: Query<&Children>,
    layers: Query<&RenderLayers>,
) {
    // Scene children and socket-mounted weapons arrive asynchronously. Bind both after
    // the normal weapon systems so neither preview leaks into the main scene or gameplay.
    for (root, character) in &roots {
        for entity in children.iter_descendants(root) {
            if layers.get(entity).ok() != Some(&character.0) {
                commands.entity(entity).insert(character.0.clone());
            }
        }
    }
}

fn resize(
    viewports: Query<(&CharacterViewport, &ComputedNode)>,
    previews: Res<CharacterPreviews>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<(&PreviewCamera, &mut Projection)>,
) {
    for (viewport, node) in &viewports {
        let size = node.size().as_uvec2();
        if size.x == 0 || size.y == 0 {
            continue;
        }
        let target = previews.target(viewport.0);
        if images
            .get(&target)
            .is_some_and(|image| image.size() != size)
        {
            if let Some(image) = images.get_mut(&target) {
                image.resize(Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                });
            }
        }
        for (camera, mut projection) in &mut cameras {
            if camera.0 == viewport.0 {
                if let Projection::Perspective(p) = &mut *projection {
                    let ratio = size.x as f32 / size.y as f32;
                    p.fov = 45_f32.to_radians().max(2. * (0.55 / (2.65 * ratio)).atan());
                }
            }
        }
    }
}

fn cleanup(mut commands: Commands, entities: Query<Entity, With<LoadoutScene>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
