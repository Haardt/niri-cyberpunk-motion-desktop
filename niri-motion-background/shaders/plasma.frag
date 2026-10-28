#version 330 core
in vec2 v_uv;
layout(location = 0) out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;

vec3 palette(float t) {
    vec3 a = vec3(0.5, 0.5, 0.5);
    vec3 b = vec3(0.5, 0.5, 0.5);
    vec3 c = vec3(1.0, 1.0, 1.0);
    vec3 d = vec3(0.0, 0.33, 0.67);
    return a + b * cos(6.28318 * (c * t + d));
}

void main() {
    vec2 uv = v_uv;
    vec2 p = (uv - 0.5) * 3.0;
    float t = u_time * 0.3;
    float v = sin(p.x * 2.0 + t)
            + sin(p.y * 2.5 + t * 1.3)
            + sin((p.x + p.y) * 1.5 + t * 0.7)
            + sin(length(p) * 3.0 - t);
    vec3 col = palette(v * 0.15 + t * 0.05);
    fragColor = vec4(col, 1.0) * 1.0;
}
