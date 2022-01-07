use crate::{material::PolylineMaterial, FRAG_SHADER_HANDLE, VERT_SHADER_HANDLE};
use bevy::{
    core::cast_slice,
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::{RenderAsset, RenderAssets},
        render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            std140::AsStd140, BindGroupLayout, BlendState, Buffer, BufferInitDescriptor,
            BufferUsages, ColorTargetState, ColorWrites, Face, FragmentState, FrontFace,
            PolygonMode, PrimitiveState, RenderPipelineDescriptor, SpecializedPipeline,
            TextureFormat, VertexBufferLayout, VertexState, VertexStepMode,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
        view::{ComputedVisibility, Visibility},
    },
};

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
            contents: &vertex_buffer_data,
        });

        Ok(GpuPolyline {
            vertex_buffer,
            vertex_count: polyline.vertices.len() as u32,
        })
    }
}

#[derive(AsStd140, Component, Clone)]
pub struct PolylineUniform {
    pub transform: Mat4,
    pub inverse_transpose_model: Mat4,
}

/// The GPU-representation of a [`Polyline`]
#[derive(Debug, Clone)]
pub struct GpuPolyline {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
}

pub struct PolylinePipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
}

impl SpecializedPipeline for PolylinePipeline {
    type Key = PolylinePipelineKey;
    fn specialize(
        &self,
        key: Self::Key,
    ) -> RenderPipelineDescriptor {
        let vertex_array_stride = 32;
            let vertex_attributes = vec![
                // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
            ];
        let mut shader_defs = Vec::new();
        let (label, blend, depth_write_enabled);

        if key.contains(PolylinePipelineKey::TRANSPARENT_MAIN_PASS) {
            label = "transparent_polyline_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else {
            label = "opaque_polyline_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: VERT_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![VertexBufferLayout {
                    array_stride: vertex_array_stride,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vertex_attributes,
                }],
            },
            fragment: Some(FragmentState {
                shader: FRAG_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![self.view_layout.clone(), self.mesh_layout.clone()]),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
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
    /// MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct PolylinePipelineKey: u32 {
        const NONE = 0;
        const NOOP = (1 << 0);
        const TRANSPARENT_MAIN_PASS = (1 << 1);
        const MSAA_RESERVED_BITS = PolylinePipelineKey::MSAA_MASK_BITS << PolylinePipelineKey::MSAA_SHIFT_BITS;
    }
}

impl PolylinePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        PolylinePipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
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
            pass.draw(0..gpu_polyline.vertex_count, 0..1);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
