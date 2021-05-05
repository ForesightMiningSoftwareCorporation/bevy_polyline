#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

void main() {
    gl_Position = ViewProj * vec4(Vertex_Position, 1);
}
