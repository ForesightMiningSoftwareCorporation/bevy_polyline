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

const POLY_LINE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x6e339e9dad279849);

const MITER_JOIN_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x468f1a2db6e312a);

pub fn new_poly_line_pipeline(striped: bool) -> RenderPipeline {
    RenderPipeline {
        pipeline: POLY_LINE_PIPELINE_HANDLE.typed().clone_weak(),
        specialization: PipelineSpecialization {
            vertex_buffer_layout: VertexBufferLayout {
                name: "PolyLine".into(),
                stride: if striped { 12 } else { 24 },
                step_mode: InputStepMode::Instance,
                attributes: vec![
                    VertexAttribute {
                        name: "Instance_Point0".into(),
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        name: "Instance_Point1".into(),
                        format: VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                ],
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn build_pipelines(shaders: &mut Assets<Shader>, pipelines: &mut Assets<PipelineDescriptor>) {
    let pipeline = build_poly_line_pipeline(shaders);
    pipelines.set_untracked(POLY_LINE_PIPELINE_HANDLE, pipeline);

    let pipeline = build_miter_join_pipeline(shaders);
    pipelines.set_untracked(MITER_JOIN_PIPELINE_HANDLE, pipeline);
}

fn build_poly_line_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        name: Some("poly_line".into()),
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
                include_str!("poly_line.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("poly_line.frag"),
            ))),
        })
    }
}

pub fn new_miter_join_pipeline() -> RenderPipeline {
    RenderPipeline {
        pipeline: MITER_JOIN_PIPELINE_HANDLE.typed().clone_weak(),
        specialization: PipelineSpecialization {
            vertex_buffer_layout: VertexBufferLayout {
                name: "PolyLine".into(),
                stride: 12, // joined lines are always striped
                step_mode: InputStepMode::Instance,
                attributes: vec![
                    VertexAttribute {
                        name: "Instance_Point0".into(),
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute {
                        name: "Instance_Point1".into(),
                        format: VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                    VertexAttribute {
                        name: "Instance_Point2".into(),
                        format: VertexFormat::Float32x3,
                        offset: 24,
                        shader_location: 2,
                    },
                ],
            },
            ..Default::default()
        },
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
