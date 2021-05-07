#version 450

layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 1) uniform PolyLineMaterial_color {
    vec4 color;
};

void main() {
    o_Target = color;
}
