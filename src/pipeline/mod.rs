use bevy::{
    asset::Assets,
    prelude::HandleUntyped,
    reflect::TypeUuid,
    render::{
        pipeline::{
            FrontFace, InputStepMode, PipelineDescriptor, PipelineSpecialization, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPipeline, VertexAttribute, VertexBufferLayout,
            VertexFormat,
        },
        shader::{Shader, ShaderStage, ShaderStages},
    },
};

const POLYLINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279849);

const MITER_JOIN_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x468f1a2db6e312a);

pub fn new_polyline_pipeline() -> RenderPipeline {
    RenderPipeline {
        pipeline: POLYLINE_PIPELINE_HANDLE.typed().clone_weak(),
        ..Default::default()
    }
}

pub fn build_pipelines(shaders: &mut Assets<Shader>, pipelines: &mut Assets<PipelineDescriptor>) {
    let pipeline = build_polyline_pipeline(shaders);
    pipelines.set_untracked(POLYLINE_PIPELINE_HANDLE, pipeline);

    let pipeline = build_miter_join_pipeline(shaders);
    pipelines.set_untracked(MITER_JOIN_PIPELINE_HANDLE, pipeline);
}

fn build_polyline_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        name: Some("polyline".into()),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None, // All faces always face the camera
            polygon_mode: PolygonMode::Fill,
            clamp_depth: false,
            conservative: false,
        },
        ..PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("polyline.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("polyline.frag"),
            ))),
        })
    }
}

pub fn new_miter_join_pipeline() -> RenderPipeline {
    RenderPipeline {
        pipeline: MITER_JOIN_PIPELINE_HANDLE.typed().clone_weak(),
        ..Default::default()
    }
}

fn build_miter_join_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        name: Some("miter_join".into()),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None, // All faces always face the camera
            polygon_mode: PolygonMode::Fill,
            clamp_depth: false,
            conservative: false,
        },
        ..PipelineDescriptor::default_config(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("miter_join.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("miter_join.frag"),
            ))),
        })
    }
}
