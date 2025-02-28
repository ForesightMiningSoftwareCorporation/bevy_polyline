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
    #ifdef JOINS
        @location(1) uv: vec2<f32>,
        @location(2) max_u: f32,
    #endif
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
    var clip0 = view.clip_from_world * polyline.model * vec4(vertex.point_a, 1.0);
    var clip1 = view.clip_from_world * polyline.model * vec4(vertex.point_b, 1.0);

    // Manual near plane clipping to avoid errors when doing the perspective divide inside this shader.
    clip0 = clip_near_plane(clip0, clip1);
    clip1 = clip_near_plane(clip1, clip0);

    let clip = mix(clip0, clip1, position.z);

    let resolution = vec2(view.viewport.z, view.viewport.w);
    var screen0 = resolution * (0.5 * clip0.xy / clip0.w + 0.5);
    var screen1 = resolution * (0.5 * clip1.xy / clip1.w + 0.5);

    let diff = screen1 - screen0;
    let len = length(diff);

    let x_basis = diff / len;
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

    // low-poly join technique similar to
    // https://www.researchgate.net/publication/220200701_High-Quality_Cartographic_Roads_on_High-Resolution_DEMs
    #ifdef JOINS
        screen0 = screen0 - x_basis * line_width / 2.0;
        screen1 = screen1 + x_basis * line_width / 2.0;

        let max_u = 1.0 + line_width / len;
        let uv = vec2((2.0 * position.z - 1.0) * max_u, position.y * 2.0);
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

    #ifdef JOINS
        return VertexOutput(vec4(clip.w * ((2.0 * pt) / resolution - 1.0), depth, clip.w), color, uv, max_u - 1.0);
    #else
        return VertexOutput(vec4(clip.w * ((2.0 * pt) / resolution - 1.0), depth, clip.w), color);
    #endif
}

fn clip_near_plane(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    // Move a if a is behind the near plane and b is in front. 
    if a.z > a.w && b.z <= b.w {
        // Interpolate a towards b until it's at the near plane.
        let distance_a = a.z - a.w;
        let distance_b = b.z - b.w;
        let t = distance_a / (distance_a - distance_b);
        return a + (b - a) * t;
    }
    return a;
}

struct FragmentInput {
    @location(0) color: vec4<f32>,
    #ifdef JOINS
        @location(1) uv: vec2<f32>,
        @location(2) max_u: f32,
    #endif
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    #ifdef JOINS
        if ( abs( in.uv.x ) > 1.0 ) {
            let a = in.uv.y;
            let b = select(in.uv.x + 1.0, in.uv.x - 1.0, in.uv.x > 0.0) / in.max_u;
            let len2 = a * a + b * b;

            if ( len2 > 1.0 ) {
                discard;
            }
        }
    #endif

    return in.color;
}
