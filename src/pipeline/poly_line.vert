#version 450

layout(location = 0) in vec3 Instance_Point0;
layout(location = 2) in vec3 Instance_Point1;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

void main() {
    mat4 Model = mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );

    // TODO uniforms
    float width = 10.0;
    vec2 resolution = vec2(1280,720);

    vec3[] positions = {
        {0.0, -0.5, 0.0},
        {0.0, -0.5, 1.0},
        {0.0, 0.5, 1.0},
        {0.0, -0.5, 0.0},
        {0.0, 0.5, 1.0},
        {0.0, 0.5, 0.0}
    };

    vec3 position = positions[gl_VertexIndex];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    vec4 clip0 = ViewProj * Model * vec4(Instance_Point0, 1);
    vec4 clip1 = ViewProj * Model * vec4(Instance_Point1, 1);

    vec2 screen0 = resolution * (0.5 * clip0.xy/clip0.w + 0.5);
    vec2 screen1 = resolution * (0.5 * clip1.xy/clip1.w + 0.5);

    vec2 xBasis = normalize(screen1 - screen0);
    vec2 yBasis = vec2(-xBasis.y, xBasis.x);

    vec2 pt0 = screen0 + width * (position.x * xBasis + position.y * yBasis);
    vec2 pt1 = screen1 + width * (position.x * xBasis + position.y * yBasis);
    vec2 pt = mix(pt0, pt1, position.z);

    vec4 clip = mix(clip0, clip1, position.z);

    gl_Position = vec4(clip.w * ((2.0 * pt) / resolution - 1.0), clip.z, clip.w);
}
