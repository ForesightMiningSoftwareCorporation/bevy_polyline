use bevy::{
    core::{bytes_of, cast_slice, Bytes},
    ecs::{reflect::ReflectComponent, system::IntoSystem},
    math::{Vec2, Vec3, Vec4},
    prelude::{
        AddAsset, Assets, Changed, Color, DetectChanges, Draw, EventReader, GlobalTransform,
        Handle, Msaa, Query, RenderPipelines, Res, ResMut, Shader, Transform, Without, World,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        draw::{DrawContext, OutsideFrustum},
        pipeline::PipelineDescriptor,
        render_graph::{
            base::{self, MainPass},
            AssetRenderResourcesNode, CommandQueue, Node, RenderGraph, ResourceSlots,
        },
        renderer::{
            BufferId, BufferInfo, BufferUsage, RenderContext, RenderResourceBinding,
            RenderResourceBindings, RenderResourceContext, RenderResources,
        },
        shader::ShaderDefs,
        RenderStage,
    },
    utils::HashSet,
    window::{WindowResized, Windows},
};
use bevy::{
    prelude::{Bundle, CoreStage, Plugin, Visible},
    render::shader,
};

mod global_render_resources_node;
mod pipeline;

use global_render_resources_node::GlobalRenderResourcesNode;

pub mod node {
    pub const POLYLINE_MATERIAL_NODE: &str = "polyline_material_node";
    pub const GLOBAL_RENDER_RESOURCES_NODE: &str = "global_render_resources_node";
    pub const POLYLINE_BUFFERS_NODE: &str = "polyline_buffers_node";
}

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_asset::<PolylineMaterial>()
            .register_type::<Polyline>()
            .insert_resource(GlobalResources::default())
            .insert_resource(PolylineBuffers::default())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                shader::asset_shader_defs_system::<PolylineMaterial>.system(),
            )
            .add_system_to_stage(
                RenderStage::RenderResource,
                polyline_resource_provider_system.system(),
            )
            .add_system_to_stage(
                RenderStage::Draw,
                polyline_draw_render_pipelines_system.system(),
            )
            .add_system(update_global_resources_system.system());

        // Setup pipeline
        let world = app.world_mut().cell();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();
        pipeline::build_pipelines(&mut *shaders, &mut *pipelines);

        // Setup rendergraph addition
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();

        let material_node = AssetRenderResourcesNode::<PolylineMaterial>::new(true);
        render_graph.add_system_node(node::POLYLINE_MATERIAL_NODE, material_node);
        render_graph
            .add_node_edge(node::POLYLINE_MATERIAL_NODE, base::node::MAIN_PASS)
            .unwrap();

        let global_render_resources_node = GlobalRenderResourcesNode::<GlobalResources>::new();
        render_graph.add_system_node(
            node::GLOBAL_RENDER_RESOURCES_NODE,
            global_render_resources_node,
        );

        let polyline_buffers_node = PolylineBuffersNode::default();
        render_graph.add_node(node::POLYLINE_BUFFERS_NODE, polyline_buffers_node);
    }
}

#[allow(clippy::too_many_arguments)]
fn polyline_draw_render_pipelines_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    mut query: Query<
        (&mut Draw, &mut RenderPipelines, &Polyline, &Visible),
        Without<OutsideFrustum>,
    >,
) {
    for (mut draw, mut render_pipelines, polyline, visible) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        // set dynamic bindings
        let render_pipelines = &mut *render_pipelines;
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            render_pipeline.specialization.sample_count = msaa.samples;

            if render_pipeline.dynamic_bindings_generation
                != render_pipelines.bindings.dynamic_bindings_generation()
            {
                render_pipeline.specialization.dynamic_bindings = render_pipelines
                    .bindings
                    .iter_dynamic_bindings()
                    .map(|name| name.to_string())
                    .collect::<HashSet<String>>();
                render_pipeline.dynamic_bindings_generation =
                    render_pipelines.bindings.dynamic_bindings_generation();
                for (handle, _) in render_pipelines.bindings.iter_assets() {
                    if let Some(bindings) = draw_context
                        .asset_render_resource_bindings
                        .get_untyped(handle)
                    {
                        for binding in bindings.iter_dynamic_bindings() {
                            render_pipeline
                                .specialization
                                .dynamic_bindings
                                .insert(binding.to_string());
                        }
                    }
                }
            }
        }

        // draw for each pipeline
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            draw_context
                .set_pipeline(
                    &mut draw,
                    &render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();

            // TODO really only need to bind these once per entity, but not currently possible
            // due to a limitation in pass_node::DrawState
            let render_resource_bindings = &mut [
                &mut render_pipelines.bindings,
                &mut render_resource_bindings,
            ];
            draw_context
                .set_bind_groups_from_bindings(&mut draw, render_resource_bindings)
                .unwrap();

            // TODO handle striped and non-striped line
            let num_line_segments = polyline.vertices.len().max(1) as u32 - 1;

            draw.draw(0..num_line_segments * 6, 0..1)
        }
    }
}

#[derive(Default)]
pub struct PolylineBuffers {
    staging_buffer: Option<BufferId>,
    buffer: Option<BufferId>,
    queue: CommandQueue,
}

pub fn polyline_resource_provider_system(
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut polyline_buffers: ResMut<PolylineBuffers>,
    mut query: Query<(&Polyline, &mut RenderPipelines)>,
) {
    polyline_buffers.queue.clear();

    let render_resource_context = &**render_resource_context;

    let mut buffer_size = query.iter_mut().fold(0, |acc, (polyline, _)| {
        let data: &[u8] = cast_slice(polyline.vertices.as_slice());
        let padded_len = data.len() + 256 - data.len() % 256;
        acc + padded_len
    });

    // Ensure staging buffer
    let staging_buffer_id = if let Some(staging_buffer_id) = polyline_buffers.staging_buffer {
        let buffer_info = render_resource_context
            .get_buffer_info(staging_buffer_id)
            .unwrap();
        if buffer_info.size >= buffer_size {
            staging_buffer_id
        } else {
            buffer_size *= 2;
            render_resource_context.remove_buffer(staging_buffer_id);
            let staging_buffer_id = render_resource_context.create_buffer(BufferInfo {
                size: buffer_size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: false,
            });
            polyline_buffers.staging_buffer.replace(staging_buffer_id);
            staging_buffer_id
        }
    } else {
        let staging_buffer_id = render_resource_context.create_buffer(BufferInfo {
            size: buffer_size,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: false,
        });
        polyline_buffers.staging_buffer.replace(staging_buffer_id);
        staging_buffer_id
    };

    // Ensure buffer
    let buffer_id = if let Some(buffer_id) = polyline_buffers.buffer {
        let buffer_info = render_resource_context.get_buffer_info(buffer_id).unwrap();
        if buffer_info.size >= buffer_size {
            buffer_id
        } else {
            render_resource_context.remove_buffer(buffer_id);
            let buffer_id = render_resource_context.create_buffer(BufferInfo {
                size: buffer_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::STORAGE,
                mapped_at_creation: false,
            });
            polyline_buffers.buffer.replace(buffer_id);
            buffer_id
        }
    } else {
        let buffer_id = render_resource_context.create_buffer(BufferInfo {
            size: buffer_size,
            buffer_usage: BufferUsage::COPY_DST | BufferUsage::STORAGE,
            mapped_at_creation: false,
        });
        polyline_buffers.buffer.replace(buffer_id);
        buffer_id
    };

    render_resource_context.map_buffer(
        staging_buffer_id,
        bevy::render::renderer::BufferMapMode::Write,
    );

    let mut offset = 0u64;

    query.for_each_mut(|(polyline, mut render_pipelines)| {
        if polyline.vertices.is_empty() {
            return;
        }

        let data = cast_slice(polyline.vertices.as_slice());

        let padded_len = data.len() + 256 - data.len() % 256;

        render_resource_context.write_mapped_buffer(
            staging_buffer_id,
            offset..offset + data.len() as u64,
            &mut |buf: &mut [u8], _| {
                buf.copy_from_slice(data);
            },
        );

        render_pipelines.bindings.set(
            "PolyLine_Vertices",
            RenderResourceBinding::Buffer {
                buffer: buffer_id,
                range: offset..offset + data.len() as u64,
                dynamic_index: None,
            },
        );

        offset += padded_len as u64;
    });

    render_resource_context.unmap_buffer(staging_buffer_id);

    polyline_buffers.queue.copy_buffer_to_buffer(
        staging_buffer_id,
        0,
        buffer_id,
        0,
        buffer_size as u64,
    );
}

#[derive(Default)]
pub struct PolylineBuffersNode;

impl Node for PolylineBuffersNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        world
            .get_resource::<PolylineBuffers>()
            .unwrap()
            .queue
            .execute(render_context);
    }
}

#[derive(Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Polyline {
    pub vertices: Vec<Vec4>,
}

#[derive(Reflect, RenderResources, ShaderDefs, TypeUuid)]
#[reflect(Component)]
#[uuid = "0be0c53f-05c9-40d4-ac1d-b56e072e33f8"]
pub struct PolylineMaterial {
    pub width: f32,
    pub color: Color,
    #[render_resources(ignore)]
    #[shader_def]
    pub perspective: bool,
}

impl Default for PolylineMaterial {
    fn default() -> Self {
        Self {
            width: 10.0,
            color: Color::WHITE,
            perspective: false,
        }
    }
}

#[derive(Bundle)]
pub struct PolylineBundle {
    pub material: Handle<PolylineMaterial>,
    pub polyline: Polyline,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visible: Visible,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
}

impl Default for PolylineBundle {
    fn default() -> Self {
        Self {
            material: Default::default(),
            polyline: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visible: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![
                pipeline::new_polyline_pipeline(),
                pipeline::new_miter_join_pipeline(),
            ]),
            main_pass: MainPass,
        }
    }
}

#[derive(Debug, Default, RenderResources)]
struct GlobalResources {
    pub resolution: Vec2,
}

fn update_global_resources_system(
    windows: Res<Windows>,
    mut global_resources: ResMut<GlobalResources>,
    mut events: EventReader<WindowResized>,
) {
    if global_resources.is_added() {
        let window = windows.get_primary().unwrap();
        global_resources.resolution.x = window.width();
        global_resources.resolution.y = window.height();
    }

    for event in events.iter() {
        global_resources.resolution.x = event.width;
        global_resources.resolution.y = event.height;
    }
}
