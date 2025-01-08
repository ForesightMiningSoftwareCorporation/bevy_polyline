use crate::{
    clipping::HalfSpacesUniform,
    polyline::{
        DrawPolyline, PolylinePipeline, PolylinePipelineKey, PolylineUniform,
        PolylineViewBindGroup, SetPolylineBindGroup,
    },
};

use bevy::{
    core_pipeline::{
        core_3d::{AlphaMask3d, Opaque3d, Opaque3dBinKey, Transparent3d},
        prepass::OpaqueNoLightmap3dBinKey,
    },
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
        extract_component::ExtractComponentPlugin,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::*,
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        view::{
            check_visibility, ExtractedView, ViewUniformOffset, VisibilitySystems, VisibleEntities,
        },
        Render, RenderApp, RenderSet,
    },
};
use std::fmt::Debug;

#[derive(Asset, Debug, PartialEq, Clone, Copy, TypePath)]
pub struct PolylineMaterial {
    /// Width of the line.
    ///
    /// Corresponds to screen pixels when line is positioned nearest the
    /// camera.
    pub width: f32,
    pub color: LinearRgba,
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
    /// Whether to clip this polyline with the half spaces defined in
    /// [`ClippingSettings`](crate::clipping::ClippingSettings).
    ///
    /// When `enable_clipping` is `true`, the polyline will only be drawn until
    /// the point it intersects with a half space defined in the clipping
    /// settings.
    pub enable_clipping: bool,
}

impl Default for PolylineMaterial {
    fn default() -> Self {
        Self {
            width: 10.0,
            color: Color::WHITE.to_linear(),
            depth_bias: 0.0,
            perspective: false,
            enable_clipping: false,
        }
    }
}

impl PolylineMaterial {
    pub fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "polyline_material_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<PolylineMaterialUniform>(false),
            ),
        )
    }

    #[inline]
    fn bind_group(render_asset: &GpuPolylineMaterial) -> &BindGroup {
        &render_asset.bind_group
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &GpuPolylineMaterial) -> &[u32] {
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
    pub enable_clipping: bool,
    pub bind_group: BindGroup,
    pub alpha_mode: AlphaMode,
}

impl RenderAsset for GpuPolylineMaterial {
    type SourceAsset = PolylineMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PolylineMaterialPipeline>,
    );

    fn prepare_asset(
        polyline_material: Self::SourceAsset,
        (device, queue, polyline_pipeline): &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let value = PolylineMaterialUniform {
            width: polyline_material.width,
            depth_bias: polyline_material.depth_bias,
            color: polyline_material.color.to_f32_array().into(),
        };

        let mut buffer = UniformBuffer::from(value);
        buffer.write_buffer(device, queue);

        let Some(buffer_binding) = buffer.binding() else {
            return Err(PrepareAssetError::RetryNextUpdate(polyline_material));
        };

        let bind_group = device.create_bind_group(
            Some("polyline_material_bind_group"),
            &polyline_pipeline.material_layout,
            &BindGroupEntries::single(buffer_binding),
        );

        let alpha_mode = if polyline_material.color.alpha() < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };

        Ok(GpuPolylineMaterial {
            buffer,
            perspective: polyline_material.perspective,
            enable_clipping: polyline_material.enable_clipping,
            alpha_mode,
            bind_group,
        })
    }
}

pub type WithPolyline = With<Handle<PolylineMaterial>>;

/// Adds the necessary ECS resources and render logic to enable rendering entities using ['PolylineMaterial']
#[derive(Default)]
pub struct PolylineMaterialPlugin;

impl Plugin for PolylineMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<PolylineMaterial>()
            .add_plugins(ExtractComponentPlugin::<Handle<PolylineMaterial>>::default())
            .add_plugins(RenderAssetPlugin::<GpuPolylineMaterial>::default())
            .add_systems(
                PostUpdate,
                check_visibility::<WithPolyline>.in_set(VisibilitySystems::CheckVisibility),
            );
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawPolylineMaterial>()
                .add_render_command::<Opaque3d, DrawPolylineMaterial>()
                .add_render_command::<AlphaMask3d, DrawPolylineMaterial>()
                .init_resource::<PolylineMaterialPipeline>()
                .init_resource::<SpecializedRenderPipelines<PolylineMaterialPipeline>>()
                .add_systems(Render, queue_material_polylines.in_set(RenderSet::Queue));
        }
    }
}

#[derive(Resource)]
pub struct PolylineMaterialPipeline {
    pub polyline_pipeline: PolylinePipeline,
    pub material_layout: BindGroupLayout,
    pub half_spaces_layout: BindGroupLayout,
}

impl FromWorld for PolylineMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = PolylineMaterial::bind_group_layout(render_device);

        let half_spaces_layout = render_device.create_bind_group_layout(
            "half_spaces_layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        let pipeline = world.get_resource::<PolylinePipeline>().unwrap();
        PolylineMaterialPipeline {
            polyline_pipeline: pipeline.to_owned(),
            material_layout,
            half_spaces_layout,
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
                .push("POLYLINE_PERSPECTIVE".into());
        }
        if key.contains(PolylinePipelineKey::CLIPPING) {
            if let Some(fragment_state) = descriptor.fragment.as_mut() {
                fragment_state.shader_defs.push("POLYLINE_CLIPPING".into());
            }
        }
        descriptor.layout = vec![
            self.polyline_pipeline.view_layout.clone(),
            self.polyline_pipeline.polyline_layout.clone(),
            self.material_layout.clone(),
            self.half_spaces_layout.clone(),
        ];
        descriptor
    }
}

type DrawPolylineMaterial = (
    SetItemPipeline,
    SetPolylineViewBindGroup<0>,
    SetPolylineBindGroup<1>,
    SetMaterialBindGroup<2>,
    SetHalfSpacesBindGroup<3>,
    DrawPolyline,
);

pub struct SetPolylineViewBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetPolylineViewBindGroup<I> {
    type ViewQuery = (Read<ViewUniformOffset>, Read<PolylineViewBindGroup>);
    type ItemQuery = ();
    type Param = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, mesh_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh_view_bind_group.value, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

pub struct SetMaterialBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetMaterialBindGroup<I> {
    type ViewQuery = ();
    type ItemQuery = Read<Handle<PolylineMaterial>>;
    type Param = SRes<RenderAssets<GpuPolylineMaterial>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        material_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material) = material_handle.and_then(|h| materials.into_inner().get(h)) else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(
            I,
            PolylineMaterial::bind_group(material),
            PolylineMaterial::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

pub struct SetHalfSpacesBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetHalfSpacesBindGroup<I> {
    type ViewQuery = ();
    type ItemQuery = ();
    type Param = SRes<HalfSpacesUniform>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        half_spaces: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &half_spaces.into_inner().bind_group, &[]);
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
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_materials: Res<RenderAssets<GpuPolylineMaterial>>,
    material_meshes: Query<(&Handle<PolylineMaterial>, &PolylineUniform)>,
    views: Query<(Entity, &ExtractedView, &VisibleEntities)>,
    mut opaque_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transparent_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    let draw_opaque = opaque_draw_functions.read().id::<DrawPolylineMaterial>();
    let draw_alpha_mask = alpha_mask_draw_functions
        .read()
        .id::<DrawPolylineMaterial>();
    let draw_transparent = transparent_draw_functions
        .read()
        .id::<DrawPolylineMaterial>();

    for (view_entity, view, visible_entities) in &views {
        let inverse_view_matrix = view.world_from_view.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);

        for visible_entity in visible_entities.get::<WithPolyline>() {
            let mut polyline_key = PolylinePipelineKey::from_msaa_samples(msaa.samples());
            polyline_key |= PolylinePipelineKey::from_hdr(view.hdr);

            let Ok((material_handle, polyline_uniform)) = material_meshes.get(*visible_entity)
            else {
                continue;
            };
            let Some(material) = render_materials.get(material_handle) else {
                continue;
            };
            if material.alpha_mode == AlphaMode::Blend {
                polyline_key |= PolylinePipelineKey::TRANSPARENT_MAIN_PASS
            }
            if material.perspective {
                polyline_key |= PolylinePipelineKey::PERSPECTIVE
            }
            if material.enable_clipping {
                polyline_key |= PolylinePipelineKey::CLIPPING
            }
            let pipeline_id =
                pipelines.specialize(&pipeline_cache, &material_pipeline, polyline_key);

            let (Some(opaque_phase), Some(alpha_mask_phase), Some(transparent_phase)) = (
                opaque_phases.get_mut(&view_entity),
                alpha_mask_phases.get_mut(&view_entity),
                transparent_phases.get_mut(&view_entity),
            ) else {
                continue;
            };

            match material.alpha_mode {
                AlphaMode::Opaque => {
                    opaque_phase.add(
                        Opaque3dBinKey {
                            pipeline: pipeline_id,
                            draw_function: draw_opaque,
                            // The draw command doesn't use a mesh handle so we don't need an `asset_id`
                            asset_id: AssetId::<Mesh>::invalid().untyped(),
                            material_bind_group_id: Some(material.bind_group.id()),
                            lightmap_image: None,
                        },
                        *visible_entity,
                        BinnedRenderPhaseType::NonMesh,
                    );
                }
                AlphaMode::Mask(_) => {
                    alpha_mask_phase.add(
                        OpaqueNoLightmap3dBinKey {
                            draw_function: draw_alpha_mask,
                            pipeline: pipeline_id,
                            asset_id: AssetId::<Mesh>::invalid().untyped(),
                            material_bind_group_id: Some(material.bind_group.id()),
                        },
                        *visible_entity,
                        BinnedRenderPhaseType::NonMesh,
                    );
                }
                AlphaMode::Blend
                | AlphaMode::Premultiplied
                | AlphaMode::Add
                | AlphaMode::Multiply => {
                    // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                    // gives the z component of translation of the mesh in view space
                    let polyline_z = inverse_view_row_2.dot(polyline_uniform.transform.col(3));
                    transparent_phase.add(Transparent3d {
                        entity: *visible_entity,
                        draw_function: draw_transparent,
                        pipeline: pipeline_id,
                        // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                        // lowest sort key and getting closer should increase. As we have
                        // -z in front of the camera, the largest distance is -far with values increasing toward the
                        // camera. As such we can just use mesh_z as the distance
                        distance: polyline_z,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::NONE,
                    });
                }
            }
        }
    }
}

/// Sets how a material's base color alpha channel is used for transparency.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug)]
pub enum AlphaMode {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
    /// Similar to [`AlphaMode::Blend`], however assumes RGB channel values are
    /// [premultiplied](https://en.wikipedia.org/wiki/Alpha_compositing#Straight_versus_premultiplied).
    ///
    /// For otherwise constant RGB values, behaves more like [`AlphaMode::Blend`] for
    /// alpha values closer to 1.0, and more like [`AlphaMode::Add`] for
    /// alpha values closer to 0.0.
    ///
    /// Can be used to avoid “border” or “outline” artifacts that can occur
    /// when using plain alpha-blended textures.
    Premultiplied,
    /// Combines the color of the fragments with the colors behind them in an
    /// additive process, (i.e. like light) producing lighter results.
    ///
    /// Black produces no effect. Alpha values can be used to modulate the result.
    ///
    /// Useful for effects like holograms, ghosts, lasers and other energy beams.
    Add,
    /// Combines the color of the fragments with the colors behind them in a
    /// multiplicative process, (i.e. like pigments) producing darker results.
    ///
    /// White produces no effect. Alpha values can be used to modulate the result.
    ///
    /// Useful for effects like stained glass, window tint film and some colored liquids.
    Multiply,
}

impl Eq for AlphaMode {}
