use bevy::{
    core::cast_slice,
    ecs::{reflect::ReflectComponent, system::IntoSystem},
    math::{Vec2, Vec3},
    prelude::{
        AddAsset, Assets, Bundle, Changed, Color, CoreStage, Draw, EventReader, GlobalTransform,
        Handle, Msaa, Plugin, Query, RenderPipelines, Res, ResMut, Shader, Transform, Visible,
        Without,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        draw::{DrawContext, OutsideFrustum},
        pipeline::PipelineDescriptor,
        render_graph::{
            base::{self, MainPass},
            AssetRenderResourcesNode, RenderGraph,
        },
        renderer::{
            BufferInfo, BufferUsage, RenderResourceBindings, RenderResourceContext, RenderResources,
        },
        shader::{self, ShaderDefs},
        texture::Texture,
        RenderStage,
    },
    utils::HashSet,
    window::{WindowResized, Windows},
};

mod global_render_resources_node;
pub mod pipeline;

use global_render_resources_node::GlobalRenderResourcesNode;

pub mod node {
    pub const POLYLINE_MATERIAL_NODE: &str = "polyline_material_node";
    pub const POLYLINE_PBR_MATERIAL_NODE: &str = "polyline_pbr_material_node";
    pub const GLOBAL_RENDER_RESOURCES_NODE: &str = "global_render_resources_node";
}

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_asset::<PolylineMaterial>()
            .add_asset::<PolylinePbrMaterial>()
            .register_type::<Polyline>()
            .insert_resource(GlobalResources::default())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                shader::asset_shader_defs_system::<PolylineMaterial>.system(),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                shader::asset_shader_defs_system::<PolylinePbrMaterial>.system(),
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

        let pbr_material_node = AssetRenderResourcesNode::<PolylinePbrMaterial>::new(true);
        render_graph.add_system_node(node::POLYLINE_PBR_MATERIAL_NODE, pbr_material_node);
        render_graph
            .add_node_edge(node::POLYLINE_PBR_MATERIAL_NODE, base::node::MAIN_PASS)
            .unwrap();

        let global_render_resources_node = GlobalRenderResourcesNode::<GlobalResources>::new();
        render_graph.add_system_node(
            node::GLOBAL_RENDER_RESOURCES_NODE,
            global_render_resources_node,
        );
        render_graph
            .add_node_edge(node::GLOBAL_RENDER_RESOURCES_NODE, base::node::MAIN_PASS)
            .unwrap();
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
            draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();

            // calculate how many instances this shader needs to render
            let num_vertices = polyline.vertices.len() as u32;
            let stride = render_pipeline.specialization.vertex_buffer_layout.stride as u32;
            let num_attributes = render_pipeline
                .specialization
                .vertex_buffer_layout
                .attributes
                .len() as u32;
            if (num_attributes - 1) > num_vertices / (stride / 12) {
                continue;
            }
            let num_instances = num_vertices / (stride / 12) - (num_attributes - 1);

            draw.draw(0..6, 0..num_instances)
        }
    }
}

pub fn polyline_resource_provider_system(
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut query: Query<(&Polyline, &mut RenderPipelines), Changed<Polyline>>,
) {
    // let mut changed_meshes = HashSet::default();
    let render_resource_context = &**render_resource_context;

    query.for_each_mut(|(polyline, mut render_pipelines)| {
        // remove previous buffer
        if let Some(buffer_id) = render_pipelines.bindings.vertex_attribute_buffer {
            render_resource_context.remove_buffer(buffer_id);
        }

        if polyline.vertices.is_empty() {
            return;
        }

        let buffer_id = render_resource_context.create_buffer_with_data(
            BufferInfo {
                size: std::mem::size_of_val(&polyline.vertices),
                buffer_usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
                mapped_at_creation: false,
            },
            cast_slice(polyline.vertices.as_slice()),
        );

        render_pipelines
            .bindings
            .vertex_attribute_buffer
            .replace(buffer_id);
    });
}

#[derive(Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Polyline {
    pub vertices: Vec<Vec3>,
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
    pub polyline_material: Handle<PolylineMaterial>,
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
            polyline_material: Default::default(),
            polyline: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visible: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![
                pipeline::new_polyline_pipeline(true),
                pipeline::new_miter_join_pipeline(),
            ]),
            main_pass: MainPass,
        }
    }
}

#[derive(Bundle)]
pub struct PolylinePbrBundle {
    pub polyline_pbr_material: Handle<PolylinePbrMaterial>,
    pub polyline: Polyline,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visible: Visible,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
}

impl Default for PolylinePbrBundle {
    fn default() -> Self {
        Self {
            polyline_pbr_material: Default::default(),
            polyline: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visible: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![
                pipeline::new_polyline_pipeline(true),
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

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here https://google.github.io/filament/Material%20Properties.pdf
#[derive(Debug, RenderResources, ShaderDefs, TypeUuid)]
#[uuid = "aa1438e6-9876-48cb-aaa4-d3d836f16eb0"]
pub struct PolylinePbrMaterial {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between If used together with a base_color_texture, this is factored into the final
    /// base color as `base_color * base_color_texture_value`
    pub base_color: Color,
    #[shader_def]
    pub base_color_texture: Option<Handle<Texture>>,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `roughness * roughness_texture_value`
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `metallic * metallic_texture_value`
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    #[shader_def]
    pub metallic_roughness_texture: Option<Handle<Texture>>,
    pub reflectance: f32,
    #[shader_def]
    pub normal_map: Option<Handle<Texture>>,
    #[render_resources(ignore)]
    #[shader_def]
    pub double_sided: bool,
    #[shader_def]
    pub occlusion_texture: Option<Handle<Texture>>,
    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    pub emissive: Color,
    #[shader_def]
    pub emissive_texture: Option<Handle<Texture>>,
    #[render_resources(ignore)]
    #[shader_def]
    pub unlit: bool,
    pub width: f32,
    #[render_resources(ignore)]
    #[shader_def]
    pub perspective: bool,
}

impl Default for PolylinePbrMaterial {
    fn default() -> Self {
        PolylinePbrMaterial {
            base_color: Color::rgb(1.0, 1.0, 1.0),
            base_color_texture: None,
            // This is the minimum the roughness is clamped to in shader code
            // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/
            // It's the minimum floating point value that won't be rounded down to 0 in the
            // calculations used. Although technically for 32-bit floats, 0.045 could be
            // used.
            roughness: 0.089,
            // Few materials are purely dielectric or metallic
            // This is just a default for mostly-dielectric
            metallic: 0.01,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see https://google.github.io/filament/Material%20Properties.pdf
            metallic_roughness_texture: None,
            reflectance: 0.5,
            normal_map: None,
            double_sided: false,
            occlusion_texture: None,
            emissive: Color::BLACK,
            emissive_texture: None,
            unlit: false,
            width: 1.0,
            perspective: false,
        }
    }
}
