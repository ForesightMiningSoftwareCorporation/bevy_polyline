#version 450

layout(location = 0) in vec3 I_Point0;
layout(location = 1) in vec3 I_Point1;

layout(location = 0) out vec3 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(std140, set = 0, binding = 1) uniform CameraPosition {
    vec4 CameraPos;
};

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 3) out vec4 v_WorldTangent;
#endif

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 1) buffer PolylineMesh_Indices {
    int[] indices;
};

struct VertexData {
    vec3 position;
    vec3 normal;
    vec2 uv;
};

layout(set = 2, binding = 2) buffer PolylineMesh_Vertices {
    VertexData[] vertices;
};

layout(set = 1, binding = 1) uniform GlobalResources_resolution {
    vec2 resolution;
};

layout(set = 3, binding = 15) uniform PolylinePbrMaterial_width {
    float width;
};

void main() {
    int index = indices[gl_VertexIndex];
    VertexData vertex = vertices[index];

    vec4 point0 = Model * vec4(I_Point0, 1.0);
    vec4 point1 = Model * vec4(I_Point1, 1.0);
    vec4 point = mix(point0, point1, vertex.position.y);

    vec3 direction = (point1 - point0).xyz;
    float norm = length(direction);
    direction = direction / norm;

    vec3 view = normalize(point.xyz - CameraPos.xyz);
    vec3 up = direction;
    vec3 right = normalize(cross(view, up));
    vec3 forward = cross(up, right);
    // up = normalize(cross(right, forward));

    float width = width;
#ifndef POLYLINEPBRMATERIAL_PERSPECTIVE
    // TODO get right of / 1.2 which is a workaround for a bug.
    width = width / resolution.y * (ViewProj * point).w / 1.2;
#endif
    mat3 billboard_matrix = mat3(right * width, up * norm, forward);
    vec3 position = billboard_matrix * vertex.position + point0.xyz;
    v_WorldPosition = position.xyz;
    v_WorldNormal = mat3(Model) * mat3(billboard_matrix) * vertex.normal;
    v_Uv = vertex.position.xy;
    gl_Position = ViewProj * vec4(position, 1.0);
}
