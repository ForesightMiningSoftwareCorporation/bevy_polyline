use crate::{
    polyline::{
        DrawPolyline, PolylinePipeline, PolylinePipelineKey, PolylineUniform,
        PolylineViewBindGroup, SetPolylineBindGroup,
    },
    SHADER_HANDLE,
};
use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        render_asset::{RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::*,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, ViewUniformOffset, VisibleEntities},
        RenderApp, RenderStage,
    },
};
use std::fmt::Debug;

#[derive(Component, Debug, PartialEq, Clone, Copy, TypeUuid)]
#[uuid = "69b87497-2ba0-4c38-ba82-f54bf1ffe873"]
pub struct PolylineMaterial {
    /// Width of the line.
    ///
    /// Corresponds to screen pixels when line is positioned nearest the
    /// camera.
    pub width: f32,
    pub color: Color,
    /// How closer to the camera than real geometry the line should be.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.0.
    pub depth_bias: f32,
    /// Whether to reduce line width with perspective.
    ///
    /// When `perspective` is `true`, `width` corresponds to screen pixels at
    /// the near plane and becomes progressively smaller further away. This is done
    /// by dividing `width` by the w component of the homogeneous coordinate.
    ///
    /// If the width where to be lower than 1, the color of the line is faded. This
    /// prevents flickering.
    ///
    /// Note that `depth_bias` **does not** interact with this in any way.
    pub perspective: bool,
}

impl Default for PolylineMaterial {
    fn default() -> Self {
        Self {
            width: 10.0,
            color: Color::WHITE,
            depth_bias: 0.0,
            perspective: false,
        }
    }
}

impl PolylineMaterial {
    fn fragment_shader() -> Handle<Shader> {
        SHADER_HANDLE.typed()
    }

    fn vertex_shader() -> Handle<Shader> {
        SHADER_HANDLE.typed()
    }

    pub fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(PolylineMaterialUniform::min_size().into()),
                },
                count: None,
            }],
            label: Some("polyline_material_layout"),
        })
    }

    #[inline]
    fn bind_group(render_asset: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        &render_asset.bind_group
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}

#[derive(ShaderType, Component, Clone)]
pub struct PolylineMaterialUniform {
    pub color: Vec4,
    pub depth_bias: f32,
    pub width: f32,
}

pub struct GpuPolylineMaterial {
    pub buffer: UniformBuffer<PolylineMaterialUniform>,
    pub perspective: bool,
    pub bind_group: BindGroup,
    pub alpha_mode: AlphaMode,
}

impl RenderAsset for PolylineMaterial {
    type ExtractedAsset = PolylineMaterial;
    type PreparedAsset = GpuPolylineMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PolylineMaterialPipeline>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        *self
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (device, queue, polyline_pipeline): &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<
        Self::PreparedAsset,
        bevy::render::render_asset::PrepareAssetError<Self::ExtractedAsset>,
    > {
        let value = PolylineMaterialUniform {
            width: material.width,
            depth_bias: material.depth_bias,
            color: material.color.as_linear_rgba_f32().into(),
        };

        let mut buffer = UniformBuffer::from(value);
        buffer.write_buffer(device, queue);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.binding().unwrap(),
            }],
            label: Some("polyline_material_bind_group"),
            layout: &polyline_pipeline.material_layout,
        });

        let alpha_mode = if material.color.a() < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };

        Ok(GpuPolylineMaterial {
            buffer,
            perspective: material.perspective,
            alpha_mode,
            bind_group,
        })
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using ['PolylineMaterial']
#[derive(Default)]
pub struct PolylineMaterialPlugin;

impl Plugin for PolylineMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<PolylineMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<PolylineMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<PolylineMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawMaterial>()
                .add_render_command::<Opaque3d, DrawMaterial>()
                .add_render_command::<AlphaMask3d, DrawMaterial>()
                .init_resource::<PolylineMaterialPipeline>()
                .init_resource::<SpecializedRenderPipelines<PolylineMaterialPipeline>>()
                .add_system_to_stage(RenderStage::Queue, queue_material_polylines);
        }
    }
}

#[derive(Resource)]
pub struct PolylineMaterialPipeline {
    pub polyline_pipeline: PolylinePipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
}

impl FromWorld for PolylineMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = PolylineMaterial::bind_group_layout(render_device);

        PolylineMaterialPipeline {
            polyline_pipeline: world.get_resource::<PolylinePipeline>().unwrap().to_owned(),
            material_layout,
            vertex_shader: PolylineMaterial::vertex_shader(),
            fragment_shader: PolylineMaterial::fragment_shader(),
        }
    }
}

impl SpecializedRenderPipeline for PolylineMaterialPipeline {
    type Key = PolylinePipelineKey;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self.polyline_pipeline.specialize(key);
        if key.contains(PolylinePipelineKey::PERSPECTIVE) {
            descriptor
                .vertex
                .shader_defs
                .push("POLYLINE_PERSPECTIVE".to_string());
        }
        //descriptor.vertex.shader = self.vertex_shader.clone();
        //descriptor.fragment.as_mut().unwrap().shader = self.fragment_shader.clone();
        descriptor.layout = Some(vec![
            self.polyline_pipeline.view_layout.clone(),
            self.polyline_pipeline.polyline_layout.clone(),
            self.material_layout.clone(),
        ]);
        descriptor
    }
}

type DrawMaterial = (
    SetItemPipeline,
    SetPolylineViewBindGroup<0>,
    SetPolylineBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawPolyline,
);

pub struct SetPolylineViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetPolylineViewBindGroup<I> {
    type Param = SQuery<(
        Read<ViewUniformOffset>,
        //Read<ViewLightsUniformOffset>,
        Read<PolylineViewBindGroup>,
    )>;
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        view_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (view_uniform, mesh_view_bind_group) = view_query.get_inner(view).unwrap();
        pass.set_bind_group(
            I,
            &mesh_view_bind_group.value,
            &[view_uniform.offset], //, view_lights.offset],
        );

        RenderCommandResult::Success
    }
}

pub struct SetMaterialBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetMaterialBindGroup<I> {
    type Param = (
        SRes<RenderAssets<PolylineMaterial>>,
        SQuery<Read<Handle<PolylineMaterial>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(
            I,
            PolylineMaterial::bind_group(material),
            PolylineMaterial::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn queue_material_polylines(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<PolylineMaterialPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PolylineMaterialPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    msaa: Res<Msaa>,
    render_materials: Res<RenderAssets<PolylineMaterial>>,
    material_meshes: Query<(&Handle<PolylineMaterial>, &PolylineUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (view, visible_entities, mut opaque_phase, mut alpha_mask_phase, mut transparent_phase) in
        views.iter_mut()
    {
        let draw_opaque = opaque_draw_functions
            .read()
            .get_id::<DrawMaterial>()
            .unwrap();
        let draw_alpha_mask = alpha_mask_draw_functions
            .read()
            .get_id::<DrawMaterial>()
            .unwrap();
        let draw_transparent = transparent_draw_functions
            .read()
            .get_id::<DrawMaterial>()
            .unwrap();

        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);

        let mut polyline_key = PolylinePipelineKey::from_msaa_samples(msaa.samples);
        polyline_key |= PolylinePipelineKey::from_hdr(view.hdr);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, polyline_uniform)) = material_meshes.get(*visible_entity) {
                if let Some(material) = render_materials.get(material_handle) {
                    if material.alpha_mode == AlphaMode::Blend {
                        polyline_key |= PolylinePipelineKey::TRANSPARENT_MAIN_PASS
                    }
                    if material.perspective {
                        polyline_key |= PolylinePipelineKey::PERSPECTIVE
                    }
                    let pipeline_id =
                        pipelines.specialize(&mut pipeline_cache, &material_pipeline, polyline_key);

                    // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                    // gives the z component of translation of the mesh in view space
                    let polyline_z = inverse_view_row_2.dot(polyline_uniform.transform.col(3));
                    match material.alpha_mode {
                        AlphaMode::Opaque => {
                            opaque_phase.add(Opaque3d {
                                entity: *visible_entity,
                                draw_function: draw_opaque,
                                pipeline: pipeline_id,
                                // NOTE: Front-to-back ordering for opaque with ascending sort means near should have the
                                // lowest sort key and getting further away should increase. As we have
                                // -z in front of the camera, values in view space decrease away from the
                                // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                distance: -polyline_z,
                            });
                        }
                        AlphaMode::Mask(_) => {
                            alpha_mask_phase.add(AlphaMask3d {
                                entity: *visible_entity,
                                draw_function: draw_alpha_mask,
                                pipeline: pipeline_id,
                                // NOTE: Front-to-back ordering for alpha mask with ascending sort means near should have the
                                // lowest sort key and getting further away should increase. As we have
                                // -z in front of the camera, values in view space decrease away from the
                                // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                distance: -polyline_z,
                            });
                        }
                        AlphaMode::Blend => {
                            transparent_phase.add(Transparent3d {
                                entity: *visible_entity,
                                draw_function: draw_transparent,
                                pipeline: pipeline_id,
                                // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                                // lowest sort key and getting closer should increase. As we have
                                // -z in front of the camera, the largest distance is -far with values increasing toward the
                                // camera. As such we can just use mesh_z as the distance
                                distance: polyline_z,
                            });
                        }
                    }
                }
            }
        }
    }
}
