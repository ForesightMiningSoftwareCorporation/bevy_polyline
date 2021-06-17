use std::iter::repeat;

use bevy::{
    core::cast_slice,
    ecs::{reflect::ReflectComponent, system::IntoSystem},
    math::{Vec2, Vec3},
    prelude::{
        AddAsset, AssetEvent, Assets, Bundle, Changed, Color, CoreStage, DetectChanges, Draw,
        Entity, EventReader, GlobalTransform, Handle, Local, Mesh, Msaa, Mut, Plugin, Query,
        QuerySet, RenderPipelines, Res, ResMut, Shader, Transform, Visible, With, Without,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        draw::{DrawContext, OutsideFrustum},
        mesh::{Indices, VertexAttributeValues},
        pipeline::{IndexFormat, PipelineDescriptor, PrimitiveTopology, VertexFormat},
        render_graph::{
            base::{self, MainPass},
            AssetRenderResourcesNode, RenderGraph,
        },
        renderer::{
            BufferInfo, BufferUsage, RenderResourceBinding, RenderResourceBindings,
            RenderResourceContext, RenderResourceId, RenderResources,
        },
        shader::{self, ShaderDefs},
        texture::Texture,
        RenderStage,
    },
    utils::{HashMap, HashSet},
    window::{WindowResized, Windows},
};

mod global_render_resources_node;
pub mod pipeline;

use global_render_resources_node::GlobalRenderResourcesNode;

use crate::mesh::{INDEX_STORAGE_BUFFER_ASSET_INDEX, VERTEX_ATTRIBUTE_STORAGE_BUFFER_ASSET_INDEX};

pub mod node {
    pub const POLYLINE_MATERIAL_NODE: &str = "polyline_material_node";
    pub const POLYLINE_PBR_MATERIAL_NODE: &str = "polyline_pbr_material_node";
    pub const GLOBAL_RENDER_RESOURCES_NODE: &str = "global_render_resources_node";
}

pub mod mesh {
    pub const INDEX_STORAGE_BUFFER_ASSET_INDEX: u64 = 20;
    pub const VERTEX_ATTRIBUTE_STORAGE_BUFFER_ASSET_INDEX: u64 = 30;
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
                RenderStage::RenderResource,
                polyline_mesh_resource_provider_system.system(),
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
    meshes: Res<Assets<Mesh>>,
    msaa: Res<Msaa>,
    mut query: Query<
        (
            &mut Draw,
            &mut RenderPipelines,
            &Polyline,
            &PolylineMesh,
            &Visible,
        ),
        Without<OutsideFrustum>,
    >,
) {
    for (mut draw, mut render_pipelines, polyline, polyline_mesh, visible) in query.iter_mut() {
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
            if let Some(mesh) = meshes.get(polyline_mesh.mesh.as_ref().unwrap()) {
                let num_indices = match mesh.indices().unwrap() {
                    Indices::U16(indices) => indices.len(),
                    Indices::U32(indices) => indices.len(),
                };

                draw.draw(0..num_indices as u32, 0..num_instances)
            }
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

fn remove_resource_save(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
    index: u64,
) {
    if let Some(RenderResourceId::Buffer(buffer)) =
        render_resource_context.get_asset_resource(handle, index)
    {
        render_resource_context.remove_buffer(buffer);
        render_resource_context.remove_asset_resource(handle, index);
    }
}
fn remove_current_mesh_resources(
    render_resource_context: &dyn RenderResourceContext,
    handle: &Handle<Mesh>,
) {
    remove_resource_save(
        render_resource_context,
        handle,
        mesh::VERTEX_ATTRIBUTE_STORAGE_BUFFER_ASSET_INDEX,
    );
    remove_resource_save(
        render_resource_context,
        handle,
        mesh::INDEX_STORAGE_BUFFER_ASSET_INDEX,
    );
}

#[derive(Default)]
pub struct PolylineMeshEntities {
    entities: HashSet<Entity>,
}

#[derive(Default)]
pub struct PolylineMeshResourceProviderState {
    mesh_entities: HashMap<Handle<Mesh>, PolylineMeshEntities>,
}

#[allow(clippy::type_complexity)]
pub fn polyline_mesh_resource_provider_system(
    mut state: Local<PolylineMeshResourceProviderState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    meshes: Res<Assets<Mesh>>,
    mut mesh_events: EventReader<AssetEvent<Mesh>>,
    mut queries: QuerySet<(
        Query<&mut RenderPipelines, With<PolylineMesh>>,
        Query<(Entity, &PolylineMesh, &mut RenderPipelines), Changed<PolylineMesh>>,
    )>,
) {
    let mut changed_meshes = HashSet::default();
    let render_resource_context = &**render_resource_context;
    for event in mesh_events.iter() {
        match event {
            AssetEvent::Created { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
            }
            AssetEvent::Modified { ref handle } => {
                changed_meshes.insert(handle.clone_weak());
                remove_current_mesh_resources(render_resource_context, handle);
            }
            AssetEvent::Removed { ref handle } => {
                remove_current_mesh_resources(render_resource_context, handle);
                // if mesh was modified and removed in the same update, ignore the modification
                // events are ordered so future modification events are ok
                changed_meshes.remove(handle);
            }
        }
    }

    // update changed mesh data
    for changed_mesh_handle in changed_meshes.iter() {
        if let Some(mesh) = meshes.get(changed_mesh_handle) {
            // TODO: check for individual buffer changes in non-interleaved mode
            if let Some(data) = mesh.get_index_buffer_bytes() {
                let index_buffer = render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        buffer_usage: BufferUsage::STORAGE,
                        ..Default::default()
                    },
                    data,
                );

                render_resource_context.set_asset_resource(
                    changed_mesh_handle,
                    RenderResourceId::Buffer(index_buffer),
                    mesh::INDEX_STORAGE_BUFFER_ASSET_INDEX,
                );
            }

            let attributes = vec![
                Mesh::ATTRIBUTE_POSITION,
                Mesh::ATTRIBUTE_NORMAL,
                Mesh::ATTRIBUTE_UV_0,
            ];

            // attributes.iter().map(|&attribute| {
            //     let attribute_values = mesh.attribute(attribute).unwrap();
            //     let vertex_format = VertexFormat::from(attribute_values);
            //     let attribute_size = vertex_format.get_size() as usize;
            //     let attributes_bytes = attribute_values.get_bytes();
            //     attributes_bytes.chunks_exact(attribute_size)
            // });

            let attributes_values = attributes
                .iter()
                .map(|attribute| mesh.attribute(*attribute).unwrap())
                .collect::<Vec<&VertexAttributeValues>>();

            let mut vertex_size = 0;
            for attribute_values in &attributes_values {
                let vertex_format = VertexFormat::from(*attribute_values);
                vertex_size += vertex_format.get_size().max(16) as usize;
            }

            let vertex_count = mesh.count_vertices();
            let mut attributes_interleaved_buffer = vec![0; vertex_count * vertex_size];
            // bundle into interleaved buffers
            let mut attribute_offset = 0;
            for attribute_values in attributes_values {
                let vertex_format = VertexFormat::from(attribute_values);
                let attribute_size = vertex_format.get_size() as usize;
                let attributes_bytes = attribute_values.get_bytes();
                for (vertex_index, attribute_bytes) in
                    attributes_bytes.chunks_exact(attribute_size).enumerate()
                {
                    let offset = vertex_index * vertex_size + attribute_offset;
                    attributes_interleaved_buffer[offset..offset + attribute_size]
                        .copy_from_slice(attribute_bytes);
                }

                attribute_offset += attribute_size.max(16);
            }

            // TODO add padding

            // let interleaved_buffer = mesh.get_vertex_buffer_data();
            if !attributes_interleaved_buffer.is_empty() {
                render_resource_context.set_asset_resource(
                    changed_mesh_handle,
                    RenderResourceId::Buffer(render_resource_context.create_buffer_with_data(
                        BufferInfo {
                            buffer_usage: BufferUsage::STORAGE,
                            ..Default::default()
                        },
                        &attributes_interleaved_buffer,
                    )),
                    mesh::VERTEX_ATTRIBUTE_STORAGE_BUFFER_ASSET_INDEX,
                );
            }

            if let Some(mesh_entities) = state.mesh_entities.get_mut(changed_mesh_handle) {
                for entity in mesh_entities.entities.iter() {
                    if let Ok(render_pipelines) = queries.q0_mut().get_mut(*entity) {
                        update_polyline_mesh(
                            render_resource_context,
                            mesh,
                            changed_mesh_handle,
                            render_pipelines,
                        );
                    }
                }
            }
        }
    }

    // handover buffers to pipeline
    for (entity, polyline_mesh, render_pipelines) in queries.q1_mut().iter_mut() {
        if let Some(handle) = &polyline_mesh.mesh {
            let mesh_entities = state
                .mesh_entities
                .entry(handle.clone_weak())
                .or_insert_with(PolylineMeshEntities::default);
            mesh_entities.entities.insert(entity);
            if let Some(mesh) = meshes.get(handle) {
                update_polyline_mesh(render_resource_context, mesh, handle, render_pipelines);
            }
        }
    }
}

fn update_polyline_mesh(
    render_resource_context: &dyn RenderResourceContext,
    mesh: &Mesh,
    handle: &Handle<Mesh>,
    mut render_pipelines: Mut<RenderPipelines>,
) {
    for render_pipeline in render_pipelines.pipelines.iter_mut() {
        debug_assert!(mesh.primitive_topology() == PrimitiveTopology::TriangleList);

        render_pipeline.specialization.primitive_topology = mesh.primitive_topology();
        // TODO: don't allocate a new vertex buffer descriptor for every entity
        // render_pipeline.specialization.vertex_buffer_layout = mesh.get_vertex_buffer_layout();
        // if let PrimitiveTopology::LineStrip | PrimitiveTopology::TriangleStrip =
        //     mesh.primitive_topology()
        // {
        //     render_pipeline.specialization.strip_index_format =
        //         mesh.indices().map(|indices| indices.into());
        // }
    }
    let index_buffer = render_resource_context
        .get_asset_resource(handle, INDEX_STORAGE_BUFFER_ASSET_INDEX)
        .unwrap()
        .get_buffer()
        .unwrap();
    let index_buffer_info = render_resource_context
        .get_buffer_info(index_buffer)
        .unwrap();
    render_pipelines.bindings.set(
        "PolylineMesh_Indices",
        RenderResourceBinding::Buffer {
            buffer: index_buffer,
            range: 0..index_buffer_info.size as u64,
            dynamic_index: None,
        },
    );

    let vertex_buffer = render_resource_context
        .get_asset_resource(handle, VERTEX_ATTRIBUTE_STORAGE_BUFFER_ASSET_INDEX)
        .unwrap()
        .get_buffer()
        .unwrap();
    let vertex_buffer_info = render_resource_context
        .get_buffer_info(vertex_buffer)
        .unwrap();
    render_pipelines.bindings.set(
        "PolylineMesh_Vertices",
        RenderResourceBinding::Buffer {
            buffer: vertex_buffer,
            range: 0..vertex_buffer_info.size as u64,
            dynamic_index: None,
        },
    );

    // if let Some(RenderResourceId::Buffer(index_buffer_resource)) =
    //     render_resource_context.get_asset_resource(handle, mesh::INDEX_BUFFER_ASSET_INDEX)
    // {
    //     let index_format: IndexFormat = mesh.indices().unwrap().into();
    //     // set index buffer into binding
    //     render_pipelines
    //         .bindings
    //         .set_index_buffer(index_buffer_resource, index_format);
    // }

    // if let Some(RenderResourceId::Buffer(vertex_attribute_buffer_resource)) =
    //     render_resource_context.get_asset_resource(handle, mesh::VERTEX_ATTRIBUTE_BUFFER_ID)
    // {
    //     // set index buffer into binding
    //     render_pipelines.bindings.vertex_attribute_buffer = Some(vertex_attribute_buffer_resource);
    // }
}

#[derive(Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Polyline {
    pub vertices: Vec<Vec3>,
}

#[derive(Debug, Default, Reflect)]
#[reflect(Component)]
pub struct PolylineMesh {
    #[reflect(ignore)]
    pub mesh: Option<Handle<Mesh>>,
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
