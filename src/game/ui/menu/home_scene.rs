use debug::*;

use bevy::{
    core_pipeline::bloom::Bloom,
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::camera::Exposure,
};
use bevy_rapier3d::render::DebugRenderContext;
use std::fs::OpenOptions;
use std::io::Write;

use super::scene_definition::{menu_scene, MENU_VERTICAL_FOV};
use crate::game::config::GameConfig;
use crate::game::map::spawn_lighting;
use crate::game::player::animation::SharedAnimations;
use crate::game::player::animation::{CharacterRig, PlayerAnimationController, ShowcaseAnimation};
use crate::game::player::player_model::PlayerModel;
use crate::game::player::skins::SkinRegistry;
use crate::game::ui::menu::PlayerLoadout;
use crate::game::GameState;

pub struct HomeScenePlugin;

impl Plugin for HomeScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugTarget>()
            .init_resource::<DebugPanelState>()
            .init_gizmo_group::<SkeletonGizmos>()
            .add_systems(
                Startup,
                (
                    setup_skeleton_gizmos,
                    setup_debug_ui.run_if(|| std::env::var_os("CSRS_DEBUG").is_some()),
                ),
            )
            .add_systems(
                Update,
                (
                    update_showcase_skin,
                    showcase_visibility,
                    asset_status,
                    fit_camera,
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(
                Update,
                setup_home_scene
                    .run_if(in_state(GameState::MainMenu))
                    .run_if(|cameras: Query<(), With<HomeSceneCamera>>| cameras.is_empty()),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup_home_scene)
            // Home scene specific systems (only in MainMenu)
            .add_systems(
                Update,
                (update_animation_index_display,).run_if(in_state(GameState::MainMenu)),
            )
            // Debug panel systems (run in both MainMenu and Playing)
            .add_systems(
                Update,
                (
                    handle_debug_buttons,
                    update_debug_display,
                    keyboard_debug_controls,
                    handle_debug_toggle,
                    handle_debug_drag,
                    update_debug_panel_position,
                    handle_postprocess_sliders,
                    update_postprocess_displays,
                )
                    .run_if(
                        in_state(GameState::MainMenu)
                            .and(|| std::env::var_os("CSRS_DEBUG").is_some()),
                    ),
            )
            // Debug skeleton gizmos run globally for all player models
            .add_systems(
                Update,
                draw_skeleton_gizmos.run_if(|| std::env::var_os("CSRS_DEBUG").is_some()),
            );
    }
}

/// Marker for home scene entities (cleaned up when leaving menu)
#[derive(Component, Clone)]
pub struct HomeSceneEntity;

/// Marker for debug UI entities (persists across states)
#[derive(Component)]
pub struct DebugUiEntity;

/// Marker for the home scene 3D camera (separate from game camera)
#[derive(Component)]
pub struct HomeSceneCamera;

/// Marker for the rotating player model
#[derive(Component)]
struct HomePlayerModel;

/// Standalone menu background
#[derive(Component)]
struct MenuBackground;
#[derive(Component)]
struct BackgroundLoad {
    started: std::time::Instant,
    reported: bool,
}

/// Marker for the warehouse map (gameplay map preview)
#[derive(Component)]
struct WarehouseScene;

/// Debug UI root
#[derive(Component)]
struct DebugUiRoot;

/// Debug panel content (collapsible part)
#[derive(Component)]
struct DebugPanelContent;

/// Debug panel header (draggable)
#[derive(Component)]
struct DebugPanelHeader;

/// Toggle button for collapse/expand
#[derive(Component)]
struct DebugToggleButton;

/// Toggle button icon (chevron)
#[derive(Component)]
struct DebugToggleIcon;

/// Debug position display text
#[derive(Component)]
struct DebugDisplayText;

/// Animation index display text
#[derive(Component)]
struct AnimationIndexDisplay;

/// Marker for bloom intensity slider
#[derive(Component)]
struct BloomIntensitySlider;

/// Marker for bloom intensity value display
#[derive(Component)]
struct BloomIntensityValue;

/// Marker for contrast slider
#[derive(Component)]
struct ContrastSlider;

/// Marker for contrast value display
#[derive(Component)]
struct ContrastValue;

/// Marker for saturation slider
#[derive(Component)]
struct SaturationSlider;

/// Marker for saturation value display
#[derive(Component)]
struct SaturationValue;

/// State for the debug panel
#[derive(Resource)]
struct DebugPanelState {
    expanded: bool,
    position: Vec2,
    dragging: bool,
    drag_offset: Vec2,
    show_skeleton: bool,
    show_hitboxes: bool,
    current_animation_index: usize,
    // Post-process debug values
    bloom_intensity: f32,
    contrast: f32,
    saturation: f32,
}

impl Default for DebugPanelState {
    fn default() -> Self {
        Self {
            expanded: true,
            position: Vec2::new(10.0, 80.0),
            dragging: false,
            drag_offset: Vec2::ZERO,
            show_skeleton: false,
            show_hitboxes: false,
            current_animation_index: 16, // Home idle animation
            bloom_intensity: 0.05,
            contrast: 1.0,
            saturation: 1.0,
        }
    }
}

/// Which entity is being controlled
#[derive(Resource, Default, PartialEq, Clone, Copy)]
enum DebugTarget {
    #[default]
    Character,
    Scene,
    Camera,
    // Retained only for the existing debug position serializer; no scene is loaded.
    #[allow(dead_code)]
    Warehouse,
}

/// Debug button actions
#[derive(Component, Clone, Copy)]
enum DebugButton {
    // Target selection
    SelectCharacter,
    SelectScene,
    SelectCamera,

    // Position adjustments
    PosXPlus,
    PosXMinus,
    PosYPlus,
    PosYMinus,
    PosZPlus,
    PosZMinus,
    // Scale adjustments
    ScalePlus,
    ScaleMinus,
    // Rotation adjustments
    RotXPlus,
    RotXMinus,
    RotYPlus,
    RotYMinus,
    // Actions
    SavePositions,
    ResetPositions,
    // Debug visualization toggles
    ToggleSkeleton,
    ToggleHitboxes,
    // Animation cycling
    AnimPrev,
    AnimNext,
}

// Button colors (also available in debug_widgets module)
const BTN_NORMAL: Color = Color::srgba(0.2, 0.2, 0.3, 0.9);
const BTN_HOVER: Color = Color::srgba(0.3, 0.3, 0.5, 0.9);
const BTN_ACTIVE: Color = Color::srgba(0.2, 0.6, 0.3, 0.9);

macro_rules! spawn_debug_button {
    ($parent:expr, $label:expr, $action:expr) => {
        $parent
            .spawn((
                $action,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BTN_NORMAL),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_child((
                Text::new($label),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
    };
}

mod debug;

fn setup_home_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loadout: Res<PlayerLoadout>,
    skins: Res<SkinRegistry>,
    config: Res<GameConfig>,
) {
    let definition = menu_scene(&config.map);
    commands.insert_resource(ClearColor(Color::srgb(0.60, 0.72, 0.80)));
    commands.spawn((
        HomeSceneEntity,
        HomeSceneCamera,
        super::glass::MenuGlassSettings::default(),
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: MENU_VERTICAL_FOV.to_radians(),
            near: 0.1,
            far: 150.,
            ..default()
        }),
        Camera {
            order: 0,
            hdr: true,
            ..default()
        },
        definition.camera,
        Exposure { ev100: 13.0 },
        Bloom {
            intensity: 0.015,
            ..default()
        },
        bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
        DistanceFog {
            color: Color::srgb(0.73, 0.73, 0.65),
            falloff: FogFalloff::Exponential { density: 0.006 },
            ..default()
        },
    ));
    let path = if std::env::var_os("CSRS_MENU_MISSING").is_some() {
        "generated/menu/missing.glb#Scene0"
    } else {
        definition.scene
    };
    commands.spawn((
        HomeSceneEntity,
        MenuBackground,
        BackgroundLoad {
            started: std::time::Instant::now(),
            reported: false,
        },
        SceneRoot(asset_server.load(path)),
        Transform::default(),
    ));
    commands.spawn((
        HomeSceneEntity,
        HomePlayerModel,
        CharacterRig(loadout.selected_skin),
        ShowcaseAnimation,
        PlayerAnimationController::default(),
        SceneRoot(asset_server.load(skins.get(loadout.selected_skin).unwrap().model_path)),
        definition.character,
    ));
    spawn_lighting(&mut commands, &definition.lighting, HomeSceneEntity);
    commands.spawn((
        HomeSceneEntity,
        MenuAssetStatus,
        super::style::label("", 13., super::style::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.),
            bottom: Val::Px(18.),
            ..default()
        },
        GlobalZIndex(190),
    ));
}
#[derive(Component)]
struct MenuAssetStatus;
fn asset_status(
    server: Res<AssetServer>,
    mut backgrounds: Query<(&SceneRoot, &mut BackgroundLoad), With<MenuBackground>>,
    mut labels: Query<&mut Text, With<MenuAssetStatus>>,
) {
    for (scene, mut load) in &mut backgrounds {
        if !load.reported && server.is_loaded_with_dependencies(scene.0.id()) {
            info!(
                "Menu background ready in {:.3}s",
                load.started.elapsed().as_secs_f32()
            );
            load.reported = true;
        }
    }
    let message = if backgrounds.iter().any(|(h, _)| {
        matches!(
            server.get_load_state(h.0.id()),
            Some(bevy::asset::LoadState::Failed(_))
        )
    }) {
        "Menu scenery unavailable · Play is still available"
    } else {
        ""
    };
    for mut text in &mut labels {
        if text.0 != message {
            **text = message.into();
        }
    }
}
fn fit_camera(windows: Query<&Window>, mut cameras: Query<&mut Projection, With<HomeSceneCamera>>) {
    let Ok(window) = windows.single() else {
        return;
    };
    // Vertical framing is stable; narrow windows widen FOV to preserve body width.
    let ratio = window.width() / window.height().max(1.);
    for mut projection in &mut cameras {
        if let Projection::Perspective(p) = &mut *projection {
            p.fov = MENU_VERTICAL_FOV
                .to_radians()
                .max(2. * (0.36 / ratio).atan());
        }
    }
}
fn cleanup_home_scene(mut commands: Commands, query: Query<Entity, With<HomeSceneEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
fn showcase_visibility(
    tab: Res<State<super::MenuTab>>,
    mut models: Query<&mut Visibility, With<HomePlayerModel>>,
) {
    for mut visibility in &mut models {
        *visibility = if matches!(*tab.get(), super::MenuTab::Play | super::MenuTab::LoadOut) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
fn update_showcase_skin(
    mut commands: Commands,
    loadout: Res<PlayerLoadout>,
    skins: Res<SkinRegistry>,
    server: Res<AssetServer>,
    models: Query<(Entity, &Transform, &CharacterRig), With<HomePlayerModel>>,
) {
    if !loadout.is_changed() {
        return;
    }
    for (entity, transform, rig) in &models {
        if rig.0 != loadout.selected_skin {
            commands.entity(entity).despawn();
            commands.spawn((
                HomeSceneEntity,
                HomePlayerModel,
                CharacterRig(loadout.selected_skin),
                ShowcaseAnimation,
                PlayerAnimationController::default(),
                SceneRoot(server.load(skins.get(loadout.selected_skin).unwrap().model_path)),
                *transform,
            ));
        }
    }
}
