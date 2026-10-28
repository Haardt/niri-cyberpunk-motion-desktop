#version 330 core
in vec2 v_uv;
layout(location = 0) out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;

void main() {
    vec2 uv = v_uv;
    float t = u_time * 0.1;
    vec3 a = vec3(0.05, 0.08, 0.18);
    vec3 b = vec3(0.85, 0.25, 0.45);
    float m = smoothstep(0.0, 1.0, uv.y + 0.15 * sin(uv.x * 3.0 + t));
    vec3 col = mix(a, b, m);
    fragColor = vec4(col, 1.0) * 1.0;
}
