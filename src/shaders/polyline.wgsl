struct View {
    view_proj: mat4x4<f32>;
    view: mat4x4<f32>;
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

struct Polyline {
    model: mat4x4<f32>;
};

[[group(1), binding(0)]]
var<uniform> polyline: Polyline;


struct PolylineMaterial {
    color: vec4<f32>;
    width: f32;
};

[[group(2), binding(0)]]
var<uniform> material: PolylineMaterial;

struct Vertex {
    [[location(0)]] I_Point0_: vec3<f32>;
    [[location(1)]] I_Point1_: vec3<f32>;
    [[builtin(vertex_index)]] index: u32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

let positions: array<vec3<f32>, 6u> = array<vec3<f32>, 6u>(
    vec3<f32>(0.0, -0.5, 0.0),
    vec3<f32>(0.0, -0.5, 1.0),
    vec3<f32>(0.0, 0.5, 1.0),
    vec3<f32>(0.0, -0.5, 0.0),
    vec3<f32>(0.0, 0.5, 1.0),
    vec3<f32>(0.0, 0.5, 0.0)
);

[[stage(vertex)]]
// fn vertex([[builtin(vertex_index)]] vertex_index: u32, vertex: Vertex) -> VertexOutput {
fn vertex(vertex: Vertex) -> VertexOutput {
    var positions = positions;
    let position = positions[vertex.index];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    let clip0 = view.view_proj * polyline.model * vec4<f32>(vertex.I_Point0_, 1.0);
    let clip1 = view.view_proj * polyline.model * vec4<f32>(vertex.I_Point1_, 1.0);
    let clip = mix(clip0, clip1, position.z);

    let resolution = vec2<f32>(view.width, view.height);
    let screen0 = resolution * (0.5 * clip0.xy / clip0.w + 0.5);
    let screen1 = resolution * (0.5 * clip1.xy / clip1.w + 0.5);

    let xBasis = normalize(screen1 - screen0);
    let yBasis = vec2<f32>(-xBasis.y, xBasis.x);

    var line_width = material.width;
    var color = material.color;

    #ifdef POLYLINE_PERSPECTIVE
         line_width = line_width / clip.w;
        // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
        if (line_width < 1.0) {
            color.a = color.a * line_width;
            line_width = 1.0;
        }
    #endif

    let pt0 = screen0 + line_width * (position.x * xBasis + position.y * yBasis);
    let pt1 = screen1 + line_width * (position.x * xBasis + position.y * yBasis);
    let pt = mix(pt0, pt1, position.z);

    let depth = clip.z;

    return VertexOutput(vec4<f32>(clip.w * ((2.0 * pt) / resolution - 1.0), depth, clip.w), color);
};

struct FragmentInput {
    [[location(0)]] color: vec4<f32>;
};

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(in.color);
};
