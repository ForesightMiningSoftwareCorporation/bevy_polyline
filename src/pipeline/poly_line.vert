#version 450

layout(location = 0) in vec4 Instance_Model1;
layout(location = 1) in vec4 Instance_Model2;
layout(location = 2) in vec4 Instance_Model3;
layout(location = 3) in vec4 Instance_Model4;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

void main() {
    vec3[] positions = {
        {0.0, -0.5, 0.0},
        {1.0, -0.5, 0.0},
        {1.0, 0.5, 0.0},
        {0.0, -0.5, 0.0},
        {1.0, 0.5, 0.0},
        {0.0, 0.5, 0.0}
    };

    vec3 position = positions[gl_VertexIndex];

    mat4 Model = mat4(Instance_Model1, Instance_Model2, Instance_Model3, Instance_Model4);

    gl_Position = ViewProj * Model * vec4(position, 1);
}
