mod pipeline;

use std::sync::Arc;

use bevy::{
    core::AsBytes,
    ecs::{reflect::ReflectComponent, world::WorldCell},
    math::{Mat4, Vec3},
    prelude::{
        Assets, ClearColor, GlobalTransform, Handle, HandleUntyped, Shader, Transform, World,
    },
    reflect::{Reflect, TypeUuid},
    render::{
        camera::ActiveCameras,
        pass::{LoadOp, PassDescriptor, TextureAttachment},
        pipeline::{
            BindGroupDescriptorId, IndexFormat, InputStepMode, PipelineCompiler,
            PipelineDescriptor, PipelineSpecialization, VertexAttribute, VertexFormat,
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

pub const INSTANCED_POLY_LINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279498);

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
    instance_buffer_id: Option<BufferId>,
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
            instance_buffer_id: None,
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
            // Manually set vertex buffer layout
            vertex_buffer_layout: VertexBufferLayout {
                name: "PolyLine".into(),
                stride: 76,
                step_mode: InputStepMode::Instance,
                attributes: vec![
                    VertexAttribute {
                        name: "Instance_Point0".into(),
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        name: "Instance_Model1".into(),
                        format: VertexFormat::Float32x4,
                        offset: 12,
                        shader_location: 1,
                    },
                    VertexAttribute {
                        name: "Instance_Model2".into(),
                        format: VertexFormat::Float32x4,
                        offset: 28,
                        shader_location: 2,
                    },
                    VertexAttribute {
                        name: "Instance_Model3".into(),
                        format: VertexFormat::Float32x4,
                        offset: 44,
                        shader_location: 3,
                    },
                    VertexAttribute {
                        name: "Instance_Model4".into(),
                        format: VertexFormat::Float32x4,
                        offset: 60,
                        shader_location: 4,
                    },
                    VertexAttribute {
                        name: "Instance_Point1".into(),
                        format: VertexFormat::Float32x3,
                        offset: 76,
                        shader_location: 5,
                    },
                ],
            },
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

        // TODO get rid of workaround
        // Or do a full 'compile_instanced_pipeline' function
        let specialized_pipeline_descriptor = {
            let specialized_pipeline_descriptor = pipeline_descriptors
                .get_mut(&specialized_pipeline_handle)
                .unwrap();
            specialized_pipeline_descriptor
                .get_layout_mut()
                .unwrap()
                .vertex_buffer_descriptors
                .get_mut(0)
                .unwrap()
                .step_mode = InputStepMode::Instance;

            render_resource_context.create_render_pipeline(
                INSTANCED_POLY_LINE_PIPELINE_HANDLE.typed(),
                &specialized_pipeline_descriptor,
                &*shaders,
            );

            specialized_pipeline_descriptor.clone()
        };

        pipeline_descriptors.set_untracked(
            INSTANCED_POLY_LINE_PIPELINE_HANDLE,
            specialized_pipeline_descriptor,
        );
        self.specialized_pipeline_handle
            .replace(INSTANCED_POLY_LINE_PIPELINE_HANDLE.typed());
    }
}

// Update bind groups and collect SetBindGroupCommands in Vec
fn update_bind_groups(
    render_resource_bindings: &mut RenderResourceBindings,
    pipeline_descriptor: &PipelineDescriptor,
    render_resource_context: &dyn RenderResourceContext,
    set_bind_group_commands: &mut Vec<SetBindGroupCommand>,
) {
    // Try to set up the bind group for each descriptor in the pipeline layout
    // Some will be set up later, during update
    for bind_group_descriptor in &pipeline_descriptor.layout.as_ref().unwrap().bind_groups {
        if let Some(bind_group) = render_resource_bindings
            .update_bind_group(bind_group_descriptor, render_resource_context)
        {
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
        self.bind_groups.clear();

        if self.instance_buffer_id.is_none() {
            let render_resource_context = world
                .get_resource_mut::<Box<dyn RenderResourceContext>>()
                .unwrap();

            self.instance_buffer_id.replace(
                render_resource_context.create_buffer_with_data(
                    BufferInfo {
                        size: 164,
                        buffer_usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
                        mapped_at_creation: false,
                    },
                    &[
                        Vec3::new(0.0, 0.0, 0.0).as_bytes(),
                        Mat4::IDENTITY.to_cols_array().as_bytes(),
                        Vec3::new(1.0, 0.0, 0.0).as_bytes(),
                        Mat4::IDENTITY.to_cols_array().as_bytes(),
                        Vec3::new(2.0, 0.0, 0.0).as_bytes(),
                    ]
                    .concat(),
                ),
            );
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
                render_pass.set_vertex_buffer(0, self.instance_buffer_id.unwrap(), 0);
                render_pass.draw(0..6, 0..2);
            },
        );
    }
}
