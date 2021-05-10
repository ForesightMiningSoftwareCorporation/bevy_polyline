#version 450

layout(location = 1) in vec4 Vertex_Color;
layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = Vertex_Color;
}
