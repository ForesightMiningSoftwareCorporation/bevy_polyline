use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    primitives::HalfSpace,
    render_resource::{BindGroup, BindGroupEntries, Buffer, BufferInitDescriptor, BufferUsages},
    renderer::RenderDevice,
    Render, RenderApp, RenderSet,
};
use bytemuck::{Pod, Zeroable};

use crate::material::PolylineMaterialPipeline;

pub const MAX_HALF_SPACES: usize = 10;

#[derive(Default)]
pub struct PolylineClippingPlugin;

impl Plugin for PolylineClippingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClippingSettings>()
            .add_plugins(ExtractResourcePlugin::<ClippingSettings>::default());
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                prepare_half_spaces
                    .in_set(RenderSet::PrepareBindGroups)
                    .run_if(|res: Res<ClippingSettings>| res.is_changed()),
            );
        }
    }
}

#[derive(Debug, Default, Resource, Clone, Deref, DerefMut)]
pub struct ClippingSettings(Vec<HalfSpace>);

impl ExtractResource for ClippingSettings {
    type Source = ClippingSettings;
    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct HalfSpaceData {
    // Make the count a vec4 to make alignment easier
    count: [u32; 4],
    half_spaces: [[f32; 4]; MAX_HALF_SPACES],
}

impl From<Vec<HalfSpace>> for HalfSpaceData {
    fn from(half_spaces: Vec<HalfSpace>) -> Self {
        let count = half_spaces.len().min(MAX_HALF_SPACES);

        let mut data = HalfSpaceData {
            count: [count as u32; 4],
            half_spaces: [[0.0; 4]; MAX_HALF_SPACES],
        };
        for (i, half_space) in half_spaces.iter().take(count).enumerate() {
            data.half_spaces[i] = half_space.normal_d().into();
        }

        data
    }
}

#[derive(Resource)]
pub struct HalfSpacesUniform {
    pub buffer: Buffer,
    pub bind_group: BindGroup,
}

pub fn prepare_half_spaces(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<PolylineMaterialPipeline>,
    clipping_settings: Res<ClippingSettings>,
) {
    let half_space_data: HalfSpaceData = clipping_settings.0.clone().into();

    let half_spaces_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("half_spaces_buffer"),
        contents: bytemuck::bytes_of(&half_space_data),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        Some("half_spaces_bind_group"),
        &pipeline.half_spaces_layout,
        &BindGroupEntries::single(half_spaces_buffer.as_entire_binding()),
    );

    commands.insert_resource(HalfSpacesUniform {
        buffer: half_spaces_buffer,
        bind_group,
    });
}
