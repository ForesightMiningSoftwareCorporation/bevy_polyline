#version 450

layout(location = 0) out vec4 o_Target;

// layout(set = 1, binding = 0) uniform PolyLineMaterial_color {
//     vec4 color;
// };

void main() {
    // o_Target = color;
    o_Target = vec4(1.0);
}
