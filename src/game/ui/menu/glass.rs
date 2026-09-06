//! Blur the scene under menu chrome before the separate UI camera draws sharp text.
use super::{friends_drawer::DrawerRoot, style::HEADER, MenuTab};
use crate::game::{ui::pause_menu::ExitConfirmation, GameState};
use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
        RenderApp,
    },
    window::PrimaryWindow,
};

pub(super) struct MenuGlassPlugin;

impl Plugin for MenuGlassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<MenuGlassSettings>::default(),
            UniformComponentPlugin::<MenuGlassSettings>::default(),
        ))
        .add_systems(
            PostUpdate,
            sync_glass
                .after(bevy::ui::UiSystem::PostLayout)
                .after(TransformSystem::TransformPropagate),
        );
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_graph_node::<ViewNodeRunner<GlassNode>>(Core3d, GlassLabel)
                .add_render_graph_edges(
                    Core3d,
                    (
                        Node3d::Tonemapping,
                        GlassLabel,
                        Node3d::EndMainPassPostProcessing,
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<GlassPipeline>();
        }
    }
}

// ShaderType generates unused field-check helpers; scope their lint allowance here.
#[allow(dead_code)]
mod shader_uniform {
    use super::*;

    #[derive(Component, Clone, Copy, Default, ExtractComponent, ShaderType)]
    pub struct MenuGlassSettings {
        // Logical window size and chrome extents keep the blur consistent across DPI scales.
        pub(super) geometry: Vec4,
    }
}
pub(super) use shader_uniform::MenuGlassSettings;

fn sync_glass(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    drawers: Query<&Node, With<DrawerRoot>>,
    ui_scale: Res<UiScale>,
    game: Res<State<GameState>>,
    tab: Res<State<MenuTab>>,
    confirmation: Res<ExitConfirmation>,
    mut cameras: Query<(Entity, &Camera, Option<&mut MenuGlassSettings>), With<Camera3d>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let menu = *game.get() == GameState::MainMenu;
    let modal = *game.get() == GameState::Paused || (menu && confirmation.open);
    let size = Vec2::new(window.width(), window.height()) / ui_scale.0;
    let drawer_width = if menu {
        drawers.single().map_or(0., |drawer| match drawer.width {
            Val::Px(width) => width,
            _ => 0.,
        })
    } else {
        0.
    };
    let glass = MenuGlassSettings {
        geometry: Vec4::new(
            size.x,
            size.y,
            if modal {
                // Dialogs float above a frosted scene covering the entire viewport.
                size.y
            } else if !menu {
                0.
            } else if matches!(
                *tab.get(),
                MenuTab::Play | MenuTab::Inventory | MenuTab::LoadOut | MenuTab::Settings
            ) {
                size.y
            } else {
                HEADER
            },
            drawer_width,
        ),
    };
    // Frost the final 3D composite, including the first-person weapon, once.
    // Removing the component on resume also skips the post-process entirely.
    let target = if menu || modal {
        cameras
            .iter()
            .filter(|(_, camera, _)| camera.is_active)
            .max_by_key(|(_, camera, _)| camera.order)
            .map(|(entity, _, _)| entity)
    } else {
        None
    };
    for (entity, _, settings) in &mut cameras {
        if Some(entity) == target {
            if let Some(mut settings) = settings {
                *settings = glass;
            } else {
                commands.entity(entity).insert(glass);
            }
        } else if settings.is_some() {
            commands.entity(entity).remove::<MenuGlassSettings>();
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GlassLabel;

#[derive(Default)]
struct GlassNode;

impl ViewNode for GlassNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DynamicUniformIndex<MenuGlassSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        context: &mut RenderContext,
        (target, index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let glass = world.resource::<GlassPipeline>();
        let cache = world.resource::<PipelineCache>();
        let (Some(horizontal), Some(vertical)) = (
            cache.get_render_pipeline(glass.pipelines[0]),
            cache.get_render_pipeline(glass.pipelines[1]),
        ) else {
            return Ok(());
        };
        let uniforms = world.resource::<ComponentUniforms<MenuGlassSettings>>();
        let Some(binding) = uniforms.uniforms().binding() else {
            return Ok(());
        };
        for pipeline in [horizontal, vertical] {
            let output = target.post_process_write();
            let bind_group = context.render_device().create_bind_group(
                "menu_glass_bind_group",
                &glass.layout,
                &BindGroupEntries::sequential((output.source, &glass.sampler, binding.clone())),
            );
            let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("menu_glass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: output.destination,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_render_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[index.index()]);
            pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}

#[derive(Resource)]
struct GlassPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipelines: [CachedRenderPipelineId; 2],
}

impl FromWorld for GlassPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(
            "menu_glass_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<MenuGlassSettings>(true),
                ),
            ),
        );
        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });
        let shader = world.load_asset("shaders/menu_glass.wgsl");
        let pipelines = [true, false].map(|horizontal| {
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("menu_glass_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: shader.clone(),
                        shader_defs: if horizontal {
                            vec!["BLUR_HORIZONTAL".into()]
                        } else {
                            vec![]
                        },
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            // The menu scene camera uses HDR, including after tonemapping.
                            format: ViewTarget::TEXTURE_FORMAT_HDR,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                })
        });
        Self {
            layout,
            sampler,
            pipelines,
        }
    }
}
