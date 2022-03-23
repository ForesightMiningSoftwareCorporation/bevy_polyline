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

struct Instance {
    [[location(0)]] point0: vec3<f32>;
    [[location(1)]] point1: vec3<f32>;
};

struct Vertex {
    [[location(2)]] position: vec3<f32>;
    [[location(3)]] normal: vec3<f32>;
    [[location(4)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(5)]] tangent: vec4<f32>;
#endif
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
#ifdef VERTEX_TANGENTS
    [[location(3)]] world_tangent: vec4<f32>;
#endif
};

[[stage(vertex)]]
fn vertex(instance: Instance, vertex: Vertex) -> VertexOutput {
    // calculate world position of points
    let point0 = polyline.model * vec4<f32>(instance.point0, 1.0);
    let point1 = polyline.model * vec4<f32>(instance.point1, 1.0);

    // find position between points for this vertex
    let position = mix(point0, point1, vertex.position.y);

    // calculate polyline direction
    let direction = (point1 - point0).xyz;
    let norm = length(direction);
    // up is normalized direction
    let up = direction / norm;

    // use the view direction for the billboard matrix
    let view_direction = normalize(position.xyz - view.world_position.xyz);
    let right = normalize(cross(view_direction, up));
    let forward = cross(up, right);

    // optionally adjust width for perspective
    var width = material.width;
#ifndef POLYLINEPBRMATERIAL_PERSPECTIVE
    // TODO get right of / 1.2 which is a workaround for a bug.
    width = width / view.height * (view.view_proj * position).w / 1.2;
#endif

    // construct the billboard matrix from the 3 directions
    let billboard_matrix = mat3x3<f32>(right * width, direction, forward * width);

    // world position is vertex position offset by point position
    let world_position = billboard_matrix * vertex.position + point0.xyz;
    // Then do normal projection for clip position
    let clip_position = view.view_proj * vec4<f32>(world_position, 1.0);

    // awkward syntax to get 3x3 part from 4x4 model matrix
    let world_normal = mat3x3<f32>(polyline.model[0].xyz, polyline.model[1].xyz, polyline.model[2].xyz) * mat3x3<f32>(billboard_matrix[0].xyz, billboard_matrix[1].xyz, billboard_matrix[2].xyz) * vertex.normal;

    let uv = vertex.uv;

    return VertexOutput(clip_position, vec4<f32>(world_position.xyz, 1.0), world_normal, uv);
};

struct FragmentInput {
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct FragmentOutput {
    [[location(0)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> FragmentOutput {
    return FragmentOutput(vec4<f32>(1.0, 0.0, 0.0, 1.0));
};
