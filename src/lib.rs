mod pipeline;

use std::sync::Arc;

use bevy::{
    core::AsBytes,
    ecs::{reflect::ReflectComponent, world::WorldCell},
    math::Vec3,
    prelude::{
        Assets, ClearColor, GlobalTransform, Handle, HandleUntyped, Shader, Transform, World,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        camera::ActiveCameras,
        pass::{LoadOp, PassDescriptor, TextureAttachment},
        pipeline::{
            BindGroupDescriptorId, IndexFormat, PipelineCompiler, PipelineDescriptor,
            PipelineSpecialization, VertexAttribute, VertexFormat,
        },
        render_graph::{base, Node, ResourceSlotInfo},
        renderer::{
            BindGroupId, BufferId, BufferInfo, BufferUsage, RenderResourceBindings,
            RenderResourceContext, RenderResourceType,
        },
    },
};
use bevy::{
    prelude::{Bundle, Plugin, Visible},
    render::pipeline::VertexBufferLayout,
};

pub const POLY_LINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279849);

pub struct PolyLinePlugin {}

impl Plugin for PolyLinePlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.register_type::<PolyLine>();

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
        dbg!(pipelines.get(&POLY_LINE_PIPELINE_HANDLE));
    }
}

#[derive(Default, Reflect)]
#[reflect(Component)]
pub struct PolyLine {
    pub vertices: Vec<Vec3>,
}

#[derive(Bundle, Default)]
pub struct PolyLineBundle {
    pub poly_line: PolyLine,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visible: Visible,
}

#[derive(Debug)]
struct SetBindGroupCommand {
    index: u32,
    descriptor_id: BindGroupDescriptorId,
    bind_group: BindGroupId,
    dynamic_uniform_indices: Option<Arc<[u32]>>,
}

pub struct PolyLineNode {
    vertex_buffer_id: Option<BufferId>,
    index_buffer_id: Option<BufferId>,
    pass_descriptor: PassDescriptor,
    inputs: Vec<ResourceSlotInfo>,
    color_attachment_input_indices: Vec<Option<usize>>,
    color_resolve_target_input_indices: Vec<Option<usize>>,
    depth_stencil_attachment_input_index: Option<usize>,
    default_clear_color_inputs: Vec<usize>,
    specialized_pipeline_handle: Option<Handle<PipelineDescriptor>>,
    bind_groups: Vec<SetBindGroupCommand>,
}

impl PolyLineNode {
    pub fn new(pass_descriptor: PassDescriptor) -> Self {
        let mut inputs = Vec::new();
        let mut color_attachment_input_indices = Vec::new();
        let mut color_resolve_target_input_indices = Vec::new();
        for color_attachment in pass_descriptor.color_attachments.iter() {
            if let TextureAttachment::Input(ref name) = color_attachment.attachment {
                color_attachment_input_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_attachment_input_indices.push(None);
            }

            if let Some(TextureAttachment::Input(ref name)) = color_attachment.resolve_target {
                color_resolve_target_input_indices.push(Some(inputs.len()));
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            } else {
                color_resolve_target_input_indices.push(None);
            }
        }

        let mut depth_stencil_attachment_input_index = None;
        if let Some(ref depth_stencil_attachment) = pass_descriptor.depth_stencil_attachment {
            if let TextureAttachment::Input(ref name) = depth_stencil_attachment.attachment {
                depth_stencil_attachment_input_index = Some(inputs.len());
                inputs.push(ResourceSlotInfo::new(
                    name.to_string(),
                    RenderResourceType::Texture,
                ));
            }
        }

        Self {
            vertex_buffer_id: None,
            index_buffer_id: None,
            pass_descriptor,
            inputs,
            color_attachment_input_indices,
            color_resolve_target_input_indices,
            depth_stencil_attachment_input_index,
            default_clear_color_inputs: vec![],
            specialized_pipeline_handle: None,
            bind_groups: vec![],
        }
    }

    pub fn use_default_clear_color(&mut self, color_attachment_index: usize) {
        self.default_clear_color_inputs.push(color_attachment_index);
    }

    /// Set up and compile the specialized pipeline to use
    fn setup_specialized_pipeline(&mut self, world: &mut WorldCell) {
        // Get all the necessary resources
        let mut pipeline_descriptors = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        let mut pipeline_compiler = world.get_resource_mut::<PipelineCompiler>().unwrap();
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();

        let render_resource_context = world
            .get_resource::<Box<dyn RenderResourceContext>>()
            .unwrap();

        let pipeline_descriptor = pipeline_descriptors
            .get(&POLY_LINE_PIPELINE_HANDLE)
            .unwrap()
            .clone();

        let pipeline_specialization = PipelineSpecialization {
            // use the sample count specified in the pass descriptor
            sample_count: self.pass_descriptor.sample_count,
            vertex_buffer_layout: VertexBufferLayout::new_from_attribute(
                VertexAttribute {
                    name: "Vertex_Position".into(),
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                bevy::render::pipeline::InputStepMode::Vertex,
            ),
            ..Default::default()
        };

        let specialized_pipeline_handle = if let Some(specialized_pipeline) = pipeline_compiler
            .get_specialized_pipeline(&POLY_LINE_PIPELINE_HANDLE.typed(), &pipeline_specialization)
        {
            specialized_pipeline
        } else {
            pipeline_compiler.compile_pipeline(
                &**render_resource_context,
                &mut pipeline_descriptors,
                &mut shaders,
                &POLY_LINE_PIPELINE_HANDLE.typed(),
                &pipeline_specialization,
            )
        };

        render_resource_context.create_render_pipeline(
            specialized_pipeline_handle.clone(),
            &pipeline_descriptor,
            &*shaders,
        );

        self.specialized_pipeline_handle
            .replace(specialized_pipeline_handle);
    }
}

// Update bind groups and collect SetBindGroupCommands in Vec
fn update_bind_groups(
    render_resource_bindings: &mut RenderResourceBindings,
    pipeline_descriptor: &PipelineDescriptor,
    render_resource_context: &dyn RenderResourceContext,
    set_bind_group_commands: &mut Vec<SetBindGroupCommand>,
) {
    dbg!(&render_resource_bindings);

    // Try to set up the bind group for each descriptor in the pipeline layout
    // Some will be set up later, during update
    for bind_group_descriptor in &pipeline_descriptor.layout.as_ref().unwrap().bind_groups {
        dbg!(bind_group_descriptor);
        if let Some(bind_group) = render_resource_bindings
            .update_bind_group(bind_group_descriptor, render_resource_context)
        {
            dbg!(bind_group);
            set_bind_group_commands.push(SetBindGroupCommand {
                index: bind_group_descriptor.index,
                descriptor_id: bind_group_descriptor.id,
                bind_group: bind_group.id,
                dynamic_uniform_indices: None,
            })
        }
    }
}

impl Node for PolyLineNode {
    fn input(&self) -> &[ResourceSlotInfo] {
        &self.inputs
    }

    fn prepare(&mut self, world: &mut World) {
        if self.vertex_buffer_id.is_none() {
            let render_resource_context = world
                .get_resource_mut::<Box<dyn RenderResourceContext>>()
                .unwrap();

            self.vertex_buffer_id.replace(
                render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: 48,
                        buffer_usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
                        mapped_at_creation: true,
                    },
                    &[
                        [0.0, -0.5, 0f32],
                        [1.0, -0.5, 0f32],
                        [1.0, 0.5, 0f32],
                        [0.0, -0.5, 0f32],
                        [1.0, 0.5, 0f32],
                        [0.0, 0.5, 0f32],
                    ]
                    .as_bytes(),
                ),
            );

            self.index_buffer_id
                .replace(render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: 12,
                        buffer_usage: BufferUsage::INDEX | BufferUsage::COPY_DST,
                        mapped_at_creation: true,
                    },
                    &[0u16, 1, 2, 3, 4, 5].as_bytes(),
                ));
        }

        if self.specialized_pipeline_handle.is_none() {
            self.setup_specialized_pipeline(&mut world.cell());
        }

        let world = world.cell();

        let render_resource_context = world
            .get_resource::<Box<dyn RenderResourceContext>>()
            .unwrap();

        let mut render_resource_bindings =
            world.get_resource_mut::<RenderResourceBindings>().unwrap();

        let pipeline_descriptors = world.get_resource::<Assets<PipelineDescriptor>>().unwrap();

        let pipeline_descriptor = pipeline_descriptors
            .get(self.specialized_pipeline_handle.as_ref().unwrap())
            .unwrap();

        let mut active_cameras = world.get_resource_mut::<ActiveCameras>().unwrap();
        let active_camera = active_cameras.get_mut(base::camera::CAMERA_3D).unwrap();

        for bind_group_descriptor in &pipeline_descriptor.layout.as_ref().unwrap().bind_groups {
            if let Some(bind_group) = active_camera
                .bindings
                .update_bind_group(bind_group_descriptor, &**render_resource_context)
            {
                dbg!(&bind_group);
                self.bind_groups.push(SetBindGroupCommand {
                    index: bind_group_descriptor.index,
                    descriptor_id: bind_group_descriptor.id,
                    bind_group: bind_group.id,
                    dynamic_uniform_indices: bind_group.dynamic_uniform_indices.clone(),
                })
            }
        }

        // Prepare bind groups
        // Get the necessary resources
        update_bind_groups(
            &mut render_resource_bindings,
            pipeline_descriptor,
            &**render_resource_context,
            &mut self.bind_groups,
        );
    }

    fn update(
        &mut self,
        world: &bevy::prelude::World,
        _render_context: &mut dyn bevy::render::renderer::RenderContext,
        input: &bevy::render::render_graph::ResourceSlots,
        _output: &mut bevy::render::render_graph::ResourceSlots,
    ) {
        for (i, color_attachment) in self
            .pass_descriptor
            .color_attachments
            .iter_mut()
            .enumerate()
        {
            if self.default_clear_color_inputs.contains(&i) {
                if let Some(default_clear_color) = world.get_resource::<ClearColor>() {
                    color_attachment.ops.load = LoadOp::Clear(default_clear_color.0);
                }
            }
            if let Some(input_index) = self.color_attachment_input_indices[i] {
                color_attachment.attachment =
                    TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
            }
            if let Some(input_index) = self.color_resolve_target_input_indices[i] {
                color_attachment.resolve_target = Some(TextureAttachment::Id(
                    input.get(input_index).unwrap().get_texture().unwrap(),
                ));
            }
        }

        if let Some(input_index) = self.depth_stencil_attachment_input_index {
            self.pass_descriptor
                .depth_stencil_attachment
                .as_mut()
                .unwrap()
                .attachment =
                TextureAttachment::Id(input.get(input_index).unwrap().get_texture().unwrap());
        }

        let render_resource_bindings = world.get_resource::<RenderResourceBindings>().unwrap();
        _render_context.begin_pass(
            &self.pass_descriptor,
            &render_resource_bindings,
            &mut |render_pass| {
                render_pass.set_pipeline(&self.specialized_pipeline_handle.as_ref().unwrap());
                self.bind_groups.iter().for_each(|command| {
                    render_pass.set_bind_group(
                        command.index,
                        command.descriptor_id,
                        command.bind_group,
                        command.dynamic_uniform_indices.as_deref(),
                    );
                });
                render_pass.set_vertex_buffer(0, self.vertex_buffer_id.unwrap(), 0);
                render_pass.set_index_buffer(self.index_buffer_id.unwrap(), 0, IndexFormat::Uint16);
                render_pass.draw_indexed(0..6, 0, 0..1);
            },
        );
    }
}

// fn draw_poly_lines_system(
//     mut draw_context: DrawContext,
//     msaa: Res<Msaa>,
//     query: Query<(&mut Draw, &PolyLine, &Visible)>,
// ) {
//     query.for_each_mut(|(mut draw, poly_line, visible)| {
//         draw.set_pipeline(&POLY_LINE_PIPELINE_HANDLE.typed());
//         // draw.set_vertex_buffer(slot, buffer, offset);
//         // draw.set_index_buffer(buffer, offset, index_format);
//         // draw.draw_indexed(indices, base_vertex, instances);
//     });
// }
