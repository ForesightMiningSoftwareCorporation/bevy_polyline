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

layout(set = 1, binding = 1) uniform GlobalResources_resolution {
    vec2 resolution;
};

layout(set = 3, binding = 15) uniform PolylinePbrMaterial_width {
    float width;
};

void main() {
    vec3[] vertices = {
        { -0.5, 0.0, 0.0 },
        { -0.5, 1.0, 0.0 },
        { -0.25, 1.0, -0.433 },
        { -0.5, 0.0, 0.0 },
        { -0.25, 1.0, -0.433 },
        { -0.25, 0.0, -0.433 },

        { -0.25, 0.0, -0.433 },
        { -0.25, 1.0, -0.433 },
        { 0.25, 1.0, -0.433 },
        { -0.25, 0.0, -0.433 },
        { 0.25, 1.0, -0.433 },
        { 0.25, 0.0, -0.433 },

        { 0.25, 0.0, -0.433 },
        { 0.25, 1.0, -0.433 },
        { 0.5, 1.0, 0.0 },
        { 0.25, 0.0, -0.433 },
        { 0.5, 1.0, 0.0 },
        { 0.5, 0.0, 0.0 }
    };

    vec3[] normals = {
        { -0.866, 0.0, -0.5 },
        { -0.866, 0.0, -0.5 },
        { -0.866, 0.0, -0.5 },
        { -0.866, 0.0, -0.5 },
        { -0.866, 0.0, -0.5 },
        { -0.866, 0.0, -0.5 },

        { 0.0, 0.0, -1.0 },
        { 0.0, 0.0, -1.0 },
        { 0.0, 0.0, -1.0 },
        { 0.0, 0.0, -1.0 },
        { 0.0, 0.0, -1.0 },
        { 0.0, 0.0, -1.0 },

        { 0.866, 0.0, -0.5 },
        { 0.866, 0.0, -0.5 },
        { 0.866, 0.0, -0.5 },
        { 0.866, 0.0, -0.5 },
        { 0.866, 0.0, -0.5 },
        { 0.866, 0.0, -0.5 }
    };

    vec3 vertex = vertices[gl_VertexIndex];
    vec3 normal = normals[gl_VertexIndex];

    vec4 point0 = Model * vec4(I_Point0, 1.0);
    vec4 point1 = Model * vec4(I_Point1, 1.0);
    vec4 point = mix(point0, point1, vertex.y);

    vec3 direction = (point1 - point0).xyz;
    float norm = length(direction);
    direction = direction / norm;

    vec3 view = normalize(point.xyz - CameraPos.xyz);
    vec3 up = direction;
    vec3 right = normalize(cross(view, up));
    vec3 forward = cross(up, right);
    // up = normalize(cross(right, forward));

    float width = width / resolution.y;
#ifndef POLYLINEPBRMATERIAL_PERSPECTIVE
    width = width * (ViewProj * point).w;
#endif
    // TODO get right of / 1.2 which is a workaround for a bug.
    mat3 billboard_matrix = mat3(right * width / 1.2, up * norm, forward);
    vec3 position = billboard_matrix * vertex + point0.xyz;
    v_WorldPosition = position.xyz;
    v_WorldNormal = mat3(Model) * mat3(billboard_matrix) * normal;
    v_Uv = vertex.xy;
    gl_Position = ViewProj * vec4(position, 1.0);
}
