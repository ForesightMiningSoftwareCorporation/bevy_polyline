use crate::material::PolylineMaterialHandle;
use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    prelude::*,
    reflect::TypePath,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::RenderDevice,
        sync_world::{RenderEntity, SyncToRenderWorld},
        view::{self, ViewUniform, ViewUniforms, VisibilityClass},
        Extract, Render, RenderApp, RenderSet,
    },
};

pub struct PolylineBasePlugin;

impl Plugin for PolylineBasePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Polyline>()
            .add_plugins(RenderAssetPlugin::<GpuPolyline>::default());
    }
}

pub struct PolylineRenderPlugin;
impl Plugin for PolylineRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UniformComponentPlugin::<PolylineUniform>::default());
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<PolylinePipeline>()
            .add_systems(ExtractSchedule, extract_polylines)
            .add_systems(
                Render,
                (
                    prepare_polyline_bind_group.in_set(RenderSet::PrepareBindGroups),
                    prepare_polyline_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Bundle, Default)]
pub struct PolylineBundle {
    pub polyline: PolylineHandle,
    pub material: PolylineMaterialHandle,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

#[derive(Debug, Default, Asset, Clone, TypePath)]
pub struct Polyline {
    pub vertices: Vec<Vec3>,
}

#[derive(Debug, Clone, Default, Component)]
#[require(SyncToRenderWorld, VisibilityClass)]
#[component(on_add = view::add_visibility_class::<PolylineHandle>)]
pub struct PolylineHandle(pub Handle<Polyline>);

impl RenderAsset for GpuPolyline {
    type SourceAsset = Polyline;

    type Param = SRes<RenderDevice>;

    fn prepare_asset(
        polyline: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        render_device: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let vertex_buffer_data = bytemuck::cast_slice(polyline.vertices.as_slice());
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
            RenderEntity,
            &InheritedVisibility,
            &ViewVisibility,
            &GlobalTransform,
            &PolylineHandle,
        )>,
    >,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, inherited_visibility, view_visibility, transform, handle) in query.iter() {
        if !inherited_visibility.get() || !view_visibility.get() {
            continue;
        }
        let transform = transform.compute_matrix();
        values.push((
            entity,
            (
                PolylineHandle(handle.0.clone_weak()),
                PolylineUniform { transform },
            ),
        ));
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

#[derive(Clone, Resource)]
pub struct PolylinePipeline {
    pub view_layout: BindGroupLayout,
    pub polyline_layout: BindGroupLayout,
    pub shader: Handle<Shader>,
}

impl FromWorld for PolylinePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let view_layout = render_device.create_bind_group_layout(
            "polyline_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let polyline_layout = render_device.create_bind_group_layout(
            "polyline_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<PolylineUniform>(true),
            ),
        );

        PolylinePipeline {
            view_layout,
            polyline_layout,
            shader: crate::SHADER_HANDLE,
        }
    }
}

impl SpecializedRenderPipeline for PolylinePipeline {
    type Key = PolylinePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
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

        let mut vertex_layout = VertexBufferLayout {
            step_mode: VertexStepMode::Instance,
            array_stride: VertexFormat::Float32x3.size(),
            attributes: vec![VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout.clone(), {
                    vertex_layout.attributes[0].shader_location = 1;
                    vertex_layout
                }],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![], // This is set in `PolylineMaterialPipeline::specialize()`
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
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: true,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct PolylinePipelineKey: u32 {
        const NONE = 0;
        const PERSPECTIVE = (1 << 0);
        const TRANSPARENT_MAIN_PASS = (1 << 1);
        const HDR = (1 << 2);
        const CLIPPING = (1 << 3);
        const MSAA_RESERVED_BITS = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
    }
}

impl PolylinePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
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

pub fn prepare_polyline_bind_group(
    mut commands: Commands,
    polyline_pipeline: Res<PolylinePipeline>,
    render_device: Res<RenderDevice>,
    polyline_uniforms: Res<ComponentUniforms<PolylineUniform>>,
) {
    if let Some(binding) = polyline_uniforms.uniforms().binding() {
        commands.insert_resource(PolylineBindGroup {
            value: render_device.create_bind_group(
                Some("polyline_bind_group"),
                &polyline_pipeline.polyline_layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[derive(Component)]
pub struct PolylineViewBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_polyline_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    polyline_pipeline: Res<PolylinePipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<Entity, With<bevy::render::view::ExtractedView>>,
) {
    for entity in views.iter() {
        let view_bind_group = render_device.create_bind_group(
            Some("polyline_view_bind_group"),
            &polyline_pipeline.view_layout,
            &BindGroupEntries::single(&view_uniforms.uniforms),
        );

        commands.entity(entity).insert(PolylineViewBindGroup {
            value: view_bind_group,
        });
    }
}

pub struct SetPolylineBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetPolylineBindGroup<I> {
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<PolylineUniform>>;
    type Param = SRes<PolylineBindGroup>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        polyline_index: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(dynamic_index) = polyline_index else {
            return RenderCommandResult::Failure("polyline_index is None");
        };
        pass.set_bind_group(I, &bind_group.into_inner().value, &[dynamic_index.index()]);
        RenderCommandResult::Success
    }
}

pub struct DrawPolyline;
impl<P: PhaseItem> RenderCommand<P> for DrawPolyline {
    type ViewQuery = ();
    type ItemQuery = Read<PolylineHandle>;
    type Param = SRes<RenderAssets<GpuPolyline>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        pl_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        polylines: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(gpu_polyline) = polylines.into_inner().get(&pl_handle.unwrap().0) {
            if gpu_polyline.vertex_count < 2 {
                return RenderCommandResult::Success;
            }

            let item_size = VertexFormat::Float32x3.size();
            let buffer_size = gpu_polyline.vertex_buffer.size() - item_size;
            pass.set_vertex_buffer(0, gpu_polyline.vertex_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(1, gpu_polyline.vertex_buffer.slice(item_size..));

            let num_instances = gpu_polyline.vertex_count.max(1) - 1;
            pass.draw(0..6, 0..num_instances);

            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure("Error loading gpu polyline")
        }
    }
}
