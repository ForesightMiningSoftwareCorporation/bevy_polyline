//WIP

struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};

[[group(0), binding(0)]]
var<uniform> view: View;

struct PolylineMaterial {
    width: f32;
    color: vec4<f32>;
    perspective: u32;
    alpha_cutoff: f32;
};

[[group(1), binding(0)]]
var<uniform> material: PolylineMaterial;

struct Polyline {
    model: mat4x4<f32>;
};

[[group(2), binding(0)]]
var<uniform> mesh: Polyline;