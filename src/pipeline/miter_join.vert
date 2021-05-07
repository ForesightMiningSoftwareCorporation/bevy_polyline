#version 450

layout(location = 0) in vec3 Instance_Point0;
layout(location = 1) in vec3 Instance_Point1;
layout(location = 2) in vec3 Instance_Point2;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 0) uniform PolyLineMaterial_width {
    float width;
};

layout(set = 3, binding = 0) uniform GlobalResources_resolution {
    vec2 resolution;
};

void main() {
    vec3[] positions = {
        {0.0, 0.0, 0.0},
        {1.0, 0.0, 0.0},
        {0.0, 1.0, 0.0},
        {0.0, 0.0, 0.0},
        {0.0, 1.0, 0.0},
        {0.0, 0.0, 1.0}
    };

    vec3 position = positions[gl_VertexIndex];

    // algorithm based on https://wwwtyro.net/2019/11/18/instanced-lines.html
    vec4 clip0 = ViewProj * Model * vec4(Instance_Point0, 1);
    vec4 clip1 = ViewProj * Model * vec4(Instance_Point1, 1);
    vec4 clip2 = ViewProj * Model * vec4(Instance_Point2, 1);

    vec2 screen0 = resolution * (0.5 * clip0.xy/clip0.w + 0.5);
    vec2 screen1 = resolution * (0.5 * clip1.xy/clip1.w + 0.5);
    vec2 screen2 = resolution * (0.5 * clip2.xy/clip2.w + 0.5);

    vec2 tangent = normalize(normalize(screen2 - screen1) + normalize(screen1 - screen0));
    vec2 miter = vec2(-tangent.y, tangent.x);

    vec2 ab = screen1 - screen0;
    vec2 cb = screen1 - screen2;
    vec2 abNorm = normalize(vec2(-ab.y, ab.x));
    vec2 cbNorm = -normalize(vec2(-cb.y, cb.x));

    float sigma = sign(dot(ab + cb, miter));

    vec2 p0 = 0.5 * width * sigma * (sigma < 0.0 ? abNorm : cbNorm);
    // TODO improve singularity case
    vec2 p1 = 0.5 * miter * sigma * width / max(dot(miter, abNorm), 0.3);
    vec2 p2 = 0.5 * width * sigma * (sigma < 0.0 ? cbNorm : abNorm);

    vec2 pt = screen1 + position.x * p0 + position.y * p1 + position.z * p2;

    gl_Position = vec4(clip1.w * ((2.0 * pt) / resolution - 1.0), clip1.z, clip1.w);
}
