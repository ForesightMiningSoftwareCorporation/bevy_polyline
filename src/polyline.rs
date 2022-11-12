use crate::{material::PolylineMaterial, SHADER_HANDLE};
use bevy::{
    core::cast_slice,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::{GlobalLightMeta, LightMeta, ViewClusterBindings, ViewShadowBindings},
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::{RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::*,
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ViewUniform, ViewUniforms},
        Extract, RenderApp, RenderStage,
    },
};

pub struct PolylineBasePlugin;

impl Plugin for PolylineBasePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Polyline>()
            .add_plugin(RenderAssetPlugin::<Polyline>::default());
    }
}

pub struct PolylineRenderPlugin;
impl Plugin for PolylineRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(UniformComponentPlugin::<PolylineUniform>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<PolylinePipeline>()
            .add_system_to_stage(RenderStage::Extract, extract_polylines)
            .add_system_to_stage(RenderStage::Queue, queue_polyline_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_polyline_view_bind_groups);
    }
}

#[derive(Bundle, Default)]
pub struct PolylineBundle {
    pub polyline: Handle<Polyline>,
    pub material: Handle<PolylineMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

#[derive(Debug, Default, Component, Clone, TypeUuid)]
#[uuid = "c76af88a-8afe-405c-9a64-0a7d845d2546"]
pub struct Polyline {
    pub vertices: Vec<Vec3>,
}

impl RenderAsset for Polyline {
    type ExtractedAsset = Polyline;

    type PreparedAsset = GpuPolyline;

    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        polyline: Self::ExtractedAsset,
        render_device: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<
        Self::PreparedAsset,
        bevy::render::render_asset::PrepareAssetError<Self::ExtractedAsset>,
    > {
        let vertex_buffer_data = cast_slice(polyline.vertices.as_slice());
        let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("Polyline Vertex Buffer"),
            contents: vertex_buffer_data,
        });

        Ok(GpuPolyline {
            vertex_buffer,
            vertex_count: polyline.vertices.len() as u32,
        })
    }
}

#[derive(Component, Clone, ShaderType)]
pub struct PolylineUniform {
    pub transform: Mat4,
    //pub inverse_transpose_model: Mat4,
}

/// The GPU-representation of a [`Polyline`]
#[derive(Debug, Clone)]
pub struct GpuPolyline {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
}

pub fn extract_polylines(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<
        Query<(
            Entity,
            &ComputedVisibility,
            &GlobalTransform,
            &Handle<Polyline>,
        )>,
    >,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, computed_visibility, transform, handle) in query.iter() {
        if !computed_visibility.is_visible() {
            continue;
        }
        let transform = transform.compute_matrix();
        values.push((
            entity,
            (
                handle.clone_weak(),
                PolylineUniform {
                    transform,
                    //inverse_transpose_model: transform.inverse().transpose(),
                },
            ),
        ));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

#[derive(Clone, Resource)]
pub struct PolylinePipeline {
    pub view_layout: BindGroupLayout,
    pub polyline_layout: BindGroupLayout,
}

impl FromWorld for PolylinePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(ViewUniform::min_size().into()),
                    },
                    count: None,
                },
            ],
            label: Some("polyline_view_layout"),
        });

        let polyline_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(PolylineUniform::min_size().into()),
                },
                count: None,
            }],
            label: Some("polyline_layout"),
        });
        PolylinePipeline {
            view_layout,
            polyline_layout,
        }
    }
}

impl SpecializedRenderPipeline for PolylinePipeline {
    type Key = PolylinePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_attributes = vec![
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
        ];
        let shader_defs = Vec::new();
        let (label, blend, depth_write_enabled);

        if key.contains(PolylinePipelineKey::TRANSPARENT_MAIN_PASS) {
            label = "transparent_polyline_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if key.contains(PolylinePipelineKey::PERSPECTIVE) {
            // We need to use transparent pass with perspective to support thin line fading.
            label = "transparent_polyline_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // Because we are expecting an opaque matl we should enable depth writes, as we don't
            // need to blend most lines.
            depth_write_enabled = true;
        } else {
            label = "opaque_polyline_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
        }

        let format = match key.contains(PolylinePipelineKey::HDR) {
            true => bevy::render::view::ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![VertexBufferLayout {
                    array_stride: 12,
                    step_mode: VertexStepMode::Instance,
                    attributes: vertex_attributes,
                }],
            },
            fragment: Some(FragmentState {
                shader: SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: None, // This is set in `PolylineMaterialPipeline::specialize()`
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct PolylinePipelineKey: u32 {
        const NONE = 0;
        const PERSPECTIVE = (1 << 0);
        const TRANSPARENT_MAIN_PASS = (1 << 1);
        const HDR = (1 << 2);
        const MSAA_RESERVED_BITS = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
    }
}

impl PolylinePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            PolylinePipelineKey::HDR
        } else {
            PolylinePipelineKey::NONE
        }
    }
}

#[derive(Resource)]
pub struct PolylineBindGroup {
    pub value: BindGroup,
}

pub fn queue_polyline_bind_group(
    mut commands: Commands,
    polyline_pipeline: Res<PolylinePipeline>,
    render_device: Res<RenderDevice>,
    polyline_uniforms: Res<ComponentUniforms<PolylineUniform>>,
) {
    if let Some(binding) = polyline_uniforms.uniforms().binding() {
        commands.insert_resource(PolylineBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("polyline_bind_group"),
                layout: &polyline_pipeline.polyline_layout,
            }),
        });
    }
}

#[derive(Component)]
pub struct PolylineViewBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_polyline_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    polyline_pipeline: Res<PolylinePipeline>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &ViewShadowBindings, &ViewClusterBindings)>,
) {
    if let (Some(view_binding), Some(_light_binding), Some(_point_light_binding)) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
    ) {
        for (entity, _view_shadow_bindings, _view_cluster_bindings) in views.iter() {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: view_binding.clone(),
                    },
                    /* Can add these bindings in the future if needed
                    BindGroupEntry {
                        binding: 1,
                        resource: light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.point_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&shadow_pipeline.point_light_sampler),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.directional_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(
                            &shadow_pipeline.directional_light_sampler,
                        ),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: point_light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: view_cluster_bindings
                            .cluster_light_index_lists
                            .binding()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: view_cluster_bindings
                            .cluster_offsets_and_counts
                            .binding()
                            .unwrap(),
                    },
                    */
                ],
                label: Some("polyline_view_bind_group"),
                layout: &polyline_pipeline.view_layout,
            });

            commands.entity(entity).insert(PolylineViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

pub struct SetPolylineBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetPolylineBindGroup<I> {
    type Param = (
        SRes<PolylineBindGroup>,
        SQuery<Read<DynamicUniformIndex<PolylineUniform>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (polyline_bind_group, polyline_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let polyline_index = polyline_query.get(item).unwrap();
        pass.set_bind_group(
            I,
            &polyline_bind_group.into_inner().value,
            &[polyline_index.index()],
        );
        RenderCommandResult::Success
    }
}

pub struct DrawPolyline;
impl EntityRenderCommand for DrawPolyline {
    type Param = (SRes<RenderAssets<Polyline>>, SQuery<Read<Handle<Polyline>>>);
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (polylines, pl_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let pl_handle = pl_query.get(item).unwrap();
        if let Some(gpu_polyline) = polylines.into_inner().get(pl_handle) {
            pass.set_vertex_buffer(0, gpu_polyline.vertex_buffer.slice(..));
            let num_instances = gpu_polyline.vertex_count.max(1) - 1;
            pass.draw(0..6, 0..num_instances);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
