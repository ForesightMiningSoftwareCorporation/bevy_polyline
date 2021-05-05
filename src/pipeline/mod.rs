use bevy::{
    asset::Assets,
    render::{
        pipeline::{
            Face, FrontFace, PipelineDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
        },
        shader::{Shader, ShaderStage, ShaderStages},
    },
};

pub(crate) fn build_poly_line_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        name: Some("poly_line".into()),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
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
