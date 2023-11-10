#import bevy_render::view::View

@group(0) @binding(0)
var<uniform> view: View;

struct Polyline {
    model: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> polyline: Polyline;

struct PolylineMaterial {
    color: vec4<f32>,
    depth_bias: f32,
    width: f32,
};

@group(2) @binding(0)
var<uniform> material: PolylineMaterial;

struct Vertex {
    @location(0) point_a: vec3<f32>,
    @location(1) point_b: vec3<f32>,
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var positions = array<vec3<f32>, 6u>(
        vec3(0.0, -0.5, 0.0),
        vec3(0.0, -0.5, 1.0),
        vec3(0.0, 0.5, 1.0),
        vec3(0.0, -0.5, 0.0),
        vec3(0.0, 0.5, 1.0),
        vec3(0.0, 0.5, 0.0)
    );
    let position = positions[vertex.index];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    let clip0 = view.view_proj * polyline.model * vec4(vertex.point_a, 1.0);
    let clip1 = view.view_proj * polyline.model * vec4(vertex.point_b, 1.0);
    let clip = mix(clip0, clip1, position.z);

    let resolution = vec2(view.viewport.z, view.viewport.w);
    let screen0 = resolution * (0.5 * clip0.xy / clip0.w + 0.5);
    let screen1 = resolution * (0.5 * clip1.xy / clip1.w + 0.5);

    let x_basis = normalize(screen1 - screen0);
    let y_basis = vec2(-x_basis.y, x_basis.x);

    var line_width = material.width;
    var color = material.color;

    #ifdef POLYLINE_PERSPECTIVE
        line_width /= clip.w;
        // Line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing
        if (line_width > 0.0 && line_width < 1.0) {
            color.a *= line_width;
            line_width = 1.0;
        }
    #endif

    let pt_offset = line_width * (position.x * x_basis + position.y * y_basis);
    let pt0 = screen0 + pt_offset;
    let pt1 = screen1 + pt_offset;
    let pt = mix(pt0, pt1, position.z);

    var depth: f32 = clip.z;
    if (material.depth_bias >= 0.0) {
        depth = depth * (1.0 - material.depth_bias);
    } else {
        let epsilon = 4.88e-04;
        // depth * (clip.w / depth)^-depth_bias. So that when -depth_bias is 1.0, this is equal to clip.w
        // and when equal to 0.0, it is exactly equal to depth.
        // the epsilon is here to prevent the depth from exceeding clip.w when -depth_bias = 1.0
        // clip.w represents the near plane in homogenous clip space in bevy, having a depth
        // of this value means nothing can be in front of this
        // The reason this uses an exponential function is that it makes it much easier for the
        // user to chose a value that is convinient for them
        depth = depth * exp2(-material.depth_bias * log2(clip.w / depth - epsilon));
    }

    return VertexOutput(vec4(clip.w * ((2.0 * pt) / resolution - 1.0), depth, clip.w), color);
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return in.color;
}
