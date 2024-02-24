use crate::polyline::{
    DrawPolyline, PolylinePipeline, PolylinePipelineKey, PolylineUniform, PolylineViewBindGroup,
    SetPolylineBindGroup,
};

use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
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
        render_asset::{
            PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets,
        },
        render_phase::*,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, ViewUniformOffset, VisibleEntities},
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
    pub fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "polyline_material_layout",
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(PolylineMaterialUniform::min_size().into()),
                },
                count: None,
            }],
        )
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
    type PreparedAsset = GpuPolylineMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PolylineMaterialPipeline>,
    );

    fn prepare_asset(
        self,
        (device, queue, polyline_pipeline): &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        let value = PolylineMaterialUniform {
            width: self.width,
            depth_bias: self.depth_bias,
            color: self.color.as_linear_rgba_f32().into(),
        };

        let mut buffer = UniformBuffer::from(value);
        buffer.write_buffer(device, queue);

        let bind_group = device.create_bind_group(
            Some("polyline_material_bind_group"),
            &polyline_pipeline.material_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.binding().unwrap(),
            }],
        );

        let alpha_mode = if self.color.a() < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };

        Ok(GpuPolylineMaterial {
            buffer,
            perspective: self.perspective,
            alpha_mode,
            bind_group,
        })
    }

    fn asset_usage(&self) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using ['PolylineMaterial']
#[derive(Default)]
pub struct PolylineMaterialPlugin;

impl Plugin for PolylineMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<PolylineMaterial>()
            .add_plugins(ExtractComponentPlugin::<Handle<PolylineMaterial>>::default())
            .add_plugins(RenderAssetPlugin::<PolylineMaterial>::default());
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawMaterial>()
                .add_render_command::<Opaque3d, DrawMaterial>()
                .add_render_command::<AlphaMask3d, DrawMaterial>()
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
}

impl FromWorld for PolylineMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let material_layout = PolylineMaterial::bind_group_layout(render_device);

        PolylineMaterialPipeline {
            polyline_pipeline: world.get_resource::<PolylinePipeline>().unwrap().to_owned(),
            material_layout,
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
        descriptor.layout = vec![
            self.polyline_pipeline.view_layout.clone(),
            self.polyline_pipeline.polyline_layout.clone(),
            self.material_layout.clone(),
        ];
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
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetPolylineViewBindGroup<I> {
    type ViewQuery = (Read<ViewUniformOffset>, Read<PolylineViewBindGroup>);
    type ItemQuery = ();
    type Param = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, mesh_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<Self::ItemQuery>,
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
    type Param = SRes<RenderAssets<PolylineMaterial>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        material_handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials
            .into_inner()
            .get(material_handle.unwrap())
            .unwrap();
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
    pipeline_cache: Res<PipelineCache>,
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

        let mut polyline_key = PolylinePipelineKey::from_msaa_samples(msaa.samples());
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
                        pipelines.specialize(&pipeline_cache, &material_pipeline, polyline_key);

                    // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                    // gives the z component of translation of the mesh in view space
                    let polyline_z = inverse_view_row_2.dot(polyline_uniform.transform.col(3));
                    match material.alpha_mode {
                        AlphaMode::Opaque => {
                            opaque_phase.add(Opaque3d {
                                entity: *visible_entity,
                                draw_function: draw_opaque,
                                pipeline: pipeline_id,
                                batch_range: 0..1,
                                dynamic_offset: None,
                                asset_id: todo!("What mesh goes here?"),
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
                                batch_range: 0..1,
                                dynamic_offset: None,
                            });
                        }
                        AlphaMode::Blend
                        | AlphaMode::Premultiplied
                        | AlphaMode::Add
                        | AlphaMode::Multiply => {
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
                                dynamic_offset: None,
                            });
                        }
                    }
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
