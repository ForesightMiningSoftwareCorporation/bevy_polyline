use bevy::{
    core::{AsBytes, Bytes},
    ecs::{reflect::ReflectComponent, system::IntoSystem},
    math::{Vec2, Vec3},
    prelude::{
        AddAsset, Assets, Changed, Color, Draw, EventReader, GlobalTransform, Handle,
        HandleUntyped, Msaa, Query, RenderPipelines, Res, ResMut, Shader, Transform, Without,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        draw::{DrawContext, OutsideFrustum},
        pipeline::{InputStepMode, PipelineDescriptor, VertexAttribute, VertexFormat},
        render_graph::{
            base::{self, MainPass},
            AssetRenderResourcesNode, RenderGraph,
        },
        renderer::{
            BufferInfo, BufferUsage, RenderResourceBindings, RenderResourceContext, RenderResources,
        },
        RenderStage,
    },
    utils::HashSet,
    window::{WindowResized, Windows},
};
use bevy::{
    prelude::{Bundle, Plugin, Visible},
    render::pipeline::VertexBufferLayout,
};

mod global_render_resources_node;
mod pipeline;

use global_render_resources_node::GlobalRenderResourcesNode;

pub mod node {
    pub const POLY_LINE_MATERIAL_NODE: &str = "poly_line_material_node";
    pub const GLOBAL_RENDER_RESOURCES_NODE: &str = "global_render_resources_node";
}

pub const POLY_LINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279849);

pub const INSTANCED_POLY_LINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279498);

pub struct PolyLinePlugin;

impl Plugin for PolyLinePlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_asset::<PolyLineMaterial>()
            .register_type::<PolyLine>()
            .insert_resource(GlobalResources::default())
            .add_system_to_stage(
                RenderStage::RenderResource,
                poly_line_resource_provider_system.system(),
            )
            .add_system_to_stage(
                RenderStage::Draw,
                poly_line_draw_render_pipelines_system.system(),
            )
            .add_system(update_global_resources_system.system());

        // Setup pipeline
        let world = app.world_mut().cell();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();
        pipelines.set_untracked(
            POLY_LINE_PIPELINE_HANDLE,
            pipeline::build_poly_line_pipeline(&mut shaders),
        );

        // Setup rendergraph addition
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();

        let material_node = AssetRenderResourcesNode::<PolyLineMaterial>::new(true);
        render_graph.add_system_node(node::POLY_LINE_MATERIAL_NODE, material_node);
        render_graph
            .add_node_edge(node::POLY_LINE_MATERIAL_NODE, base::node::MAIN_PASS)
            .unwrap();

        let global_render_resources_node = GlobalRenderResourcesNode::<GlobalResources>::new();
        render_graph.add_system_node(
            node::GLOBAL_RENDER_RESOURCES_NODE,
            global_render_resources_node,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn poly_line_draw_render_pipelines_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    mut query: Query<
        (&mut Draw, &mut RenderPipelines, &PolyLine, &Visible),
        Without<OutsideFrustum>,
    >,
) {
    for (mut draw, mut render_pipelines, poly_line, visible) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        // set dynamic bindings
        let render_pipelines = &mut *render_pipelines;
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            render_pipeline.specialization.sample_count = msaa.samples;

            // TODO Consider moving to build_poly_line_pipeline
            // Needed to pass compiler check for all vertex buffer attibutes
            render_pipeline.specialization.vertex_buffer_layout = VertexBufferLayout {
                name: "PolyLine".into(),
                stride: match poly_line.style {
                    PolyLineStyle::LineStrip { .. } => 12,
                    PolyLineStyle::LineList { .. } => 24,
                },
                // But this field is overwritten
                step_mode: InputStepMode::Instance,
                attributes: vec![
                    VertexAttribute {
                        name: "Instance_Point0".into(),
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        name: "Instance_Point1".into(),
                        format: VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                ],
            };

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
            let render_resource_bindings = &mut [
                &mut render_pipelines.bindings,
                &mut render_resource_bindings,
            ];
            draw_context
                .set_pipeline(
                    &mut draw,
                    &render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            draw_context
                .set_bind_groups_from_bindings(&mut draw, render_resource_bindings)
                .unwrap();
            draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();

            match poly_line.style {
                PolyLineStyle::LineStrip {
                    join_style,
                    cap_style,
                } => {
                    // TODO can be done in one draw call and 4 triangles for the miter case
                    // if we change index buffer stride to 4 points (p-1, p0, p1 and p2) instead of just p0, p1
                    // and pad the buffer at start/end with a copy of the first/last point or a point along the x-basis
                    draw.draw(0..6, 0..(poly_line.vertices.len() - 1) as u32)
                }
                PolyLineStyle::LineList { cap_style } => {
                    draw.draw(0..6, 0..(poly_line.vertices.len() / 2) as u32)
                }
            }
        }
    }
}

pub fn poly_line_resource_provider_system(
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut query: Query<(&PolyLine, &mut RenderPipelines), Changed<PolyLine>>,
) {
    // let mut changed_meshes = HashSet::default();
    let render_resource_context = &**render_resource_context;

    query.for_each_mut(|(poly_line, mut render_pipelines)| {
        // remove previous buffer
        if let Some(buffer_id) = render_pipelines.bindings.vertex_attribute_buffer {
            render_resource_context.remove_buffer(buffer_id);
        }

        let buffer_id = render_resource_context.create_buffer_with_data(
            BufferInfo {
                size: poly_line.vertices.byte_len(),
                buffer_usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
                mapped_at_creation: false,
            },
            poly_line.vertices.as_bytes(),
        );

        render_pipelines
            .bindings
            .vertex_attribute_buffer
            .replace(buffer_id);
    });
}

#[derive(Copy, Clone, Debug, Reflect)]
pub enum PolyLineJoinStyle {
    None,
    Miter,
}

impl Default for PolyLineJoinStyle {
    fn default() -> Self {
        Self::Miter
    }
}

#[derive(Copy, Clone, Debug, Reflect)]
pub enum PolyLineCapStyle {
    Butt,
}

impl Default for PolyLineCapStyle {
    fn default() -> Self {
        Self::Butt
    }
}

#[derive(Copy, Clone, Debug, Reflect)]
pub enum PolyLineStyle {
    LineStrip {
        join_style: PolyLineJoinStyle,
        cap_style: PolyLineCapStyle,
    },
    LineList {
        cap_style: PolyLineCapStyle,
    },
}

impl Default for PolyLineStyle {
    fn default() -> Self {
        Self::LineStrip {
            join_style: Default::default(),
            cap_style: Default::default(),
        }
    }
}

#[derive(Default, Reflect)]
#[reflect(Component)]
pub struct PolyLine {
    pub vertices: Vec<Vec3>,
    pub style: PolyLineStyle,
}

#[derive(Reflect, RenderResources, TypeUuid)]
#[reflect(Component)]
#[uuid = "0be0c53f-05c9-40d4-ac1d-b56e072e33f8"]
pub struct PolyLineMaterial {
    pub width: f32,
    pub color: Color,
}

impl Default for PolyLineMaterial {
    fn default() -> Self {
        Self {
            width: 10.0,
            color: Color::WHITE,
        }
    }
}

#[derive(Bundle)]
pub struct PolyLineBundle {
    pub material: Handle<PolyLineMaterial>,
    pub poly_line: PolyLine,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visible: Visible,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
}

impl Default for PolyLineBundle {
    fn default() -> Self {
        Self {
            material: Default::default(),
            poly_line: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visible: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines::from_handles(vec![
                &POLY_LINE_PIPELINE_HANDLE.typed()
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
