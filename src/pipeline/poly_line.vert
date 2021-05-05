#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec4 Instance_Model1;
layout(location = 2) in vec4 Instance_Model2;
layout(location = 3) in vec4 Instance_Model3;
layout(location = 4) in vec4 Instance_Model4;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

void main() {
    mat4 Model = mat4(Instance_Model1, Instance_Model2, Instance_Model3, Instance_Model4);

    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1);
}
