use bevy::{
    ecs::{
        system::{BoxedSystem, IntoSystem, Local, Res, ResMut},
        world::World,
    },
    render::{
        render_graph::Node,
        renderer::{self, RenderContext},
    },
};
use std::{io::Write, marker::PhantomData};

#[derive(Debug, Default)]
pub struct GlobalRenderResourcesNode<T>
where
    T: RenderResources,
{
    command_queue: CommandQueue,
    marker: PhantomData<T>,
}

impl<T> GlobalRenderResourcesNode<T>
where
    T: RenderResources,
{
    pub fn new() -> Self {
        Self {
            command_queue: CommandQueue::default(),
            marker: Default::default(),
        }
    }
}

impl<T> Node for GlobalRenderResourcesNode<T>
where
    T: RenderResources,
{
    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl<T> SystemNode for GlobalRenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    fn get_system(&self) -> BoxedSystem {
        let system = global_render_resources_node_system::<T>
            .system()
            .config(|config| {
                config.0 = Some(GlobalRenderResourcesNodeState {
                    command_queue: self.command_queue.clone(),
                    ..Default::default()
                })
            });

        Box::new(system)
    }
}

struct GlobalRenderResourcesNodeState<T: RenderResources> {
    command_queue: CommandQueue,
    staging_buffer: Option<BufferId>,
    target_buffer: Option<BufferId>,
    _marker: PhantomData<T>,
}

impl<T: RenderResources> Default for GlobalRenderResourcesNodeState<T> {
    fn default() -> Self {
        Self {
            command_queue: Default::default(),
            staging_buffer: None,
            target_buffer: None,
            _marker: Default::default(),
        }
    }
}

fn global_render_resources_node_system<T: RenderResources>(
    mut state: Local<GlobalRenderResourcesNodeState<T>>,
    render_resources: Res<T>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
) {
    // TODO add support for textures
    // No need to do anything if no changes
    if render_resources.is_changed() {
        // Precalculate the aligned size of the whole render resources buffer
        let aligned_size = render_resources
            .iter()
            .fold(0, |aligned_size, render_resource| {
                aligned_size
                    + render_resource_context
                        .get_aligned_uniform_size(render_resource.buffer_byte_len().unwrap(), false)
            });

        // Get old buffer and possibly resize
        let staging_buffer = match state.staging_buffer {
            Some(staging_buffer) => {
                if render_resource_context
                    .get_buffer_info(staging_buffer)
                    .unwrap()
                    .size
                    != aligned_size
                {
                    render_resource_context.remove_buffer(staging_buffer);
                    render_resource_context.create_buffer(BufferInfo {
                        size: aligned_size,
                        buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                        mapped_at_creation: true,
                    })
                } else {
                    staging_buffer
                }
            }
            None => {
                // Or create a new one
                render_resource_context.create_buffer(BufferInfo {
                    size: aligned_size,
                    buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                    mapped_at_creation: true,
                })
            }
        };

        // Get old buffer and possibly resize
        let target_buffer = if let Some(target_buffer) = state.target_buffer {
            if render_resource_context
                .get_buffer_info(target_buffer)
                .unwrap()
                .size
                != aligned_size
            {
                render_resource_context.remove_buffer(target_buffer);
                render_resource_context.create_buffer(BufferInfo {
                    size: aligned_size,
                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                    mapped_at_creation: false,
                })
            } else {
                target_buffer
            }
        } else {
            // Or create a new one
            render_resource_context.create_buffer(BufferInfo {
                size: aligned_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                mapped_at_creation: false,
            })
        };

        // Write the resources into the staging buffer
        let mut offset = 0u64;
        for (index, render_resource) in render_resources.iter().enumerate() {
            let render_resource_name = render_resources.get_render_resource_name(index).unwrap();

            let size = render_resource.buffer_byte_len().unwrap();
            let aligned_size = render_resource_context.get_aligned_uniform_size(size, false) as u64;

            render_resource_context.write_mapped_buffer(
                staging_buffer,
                offset..(offset + aligned_size),
                &mut |mut buf, _render_resource_context| {
                    render_resource.write_buffer_bytes(buf);

                    // add padding
                    for _ in 0..(aligned_size - size as u64) {
                        buf.write_all(&[0]).unwrap();
                    }
                },
            );

            render_resource_bindings.set(
                render_resource_name,
                RenderResourceBinding::Buffer {
                    buffer: target_buffer,
                    range: offset..(offset + aligned_size),
                    dynamic_index: None,
                },
            );

            offset += aligned_size;
        }

        render_resource_context.unmap_buffer(staging_buffer);

        state.command_queue.copy_buffer_to_buffer(
            staging_buffer,
            0,
            target_buffer,
            0,
            aligned_size as u64,
        );
    }
}
