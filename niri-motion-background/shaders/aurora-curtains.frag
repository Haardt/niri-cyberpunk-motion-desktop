#version 330 core
// Original procedural aurora curtains. MIT; no textures or audio.
in vec2 v_uv;
out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;
uniform float u_night;

void main() {
    vec2 uv = v_uv;
    float aspect = u_resolution.x / max(u_resolution.y, 1.0);
    vec2 p = vec2((uv.x - 0.5) * aspect, 1.0 - uv.y);
    float t = u_time * 0.16;
    vec3 col = mix(vec3(0.014, 0.025, 0.065), vec3(0.003, 0.008, 0.025), u_night);
    col += vec3(0.02, 0.035, 0.055) * exp(-3.0 * p.y);

    // Broad analytic ribbons stay smooth at reduced render resolutions.
    for (int i = 0; i < 4; ++i) {
        float layer = float(i);
        float x = p.x * (1.6 + layer * 0.24) + layer * 2.7;
        float fold = sin(x * 2.1 + t + layer)
                   + 0.32 * sin(x * 4.7 - t * 0.7 + layer);
        float base = 0.25 + layer * 0.09 + 0.075 * fold;
        float height = p.y - base;
        float edge = smoothstep(-0.025, 0.025, height);
        float veil = edge * exp(-max(height, 0.0) * (6.0 + layer));
        float strands = 0.62 + 0.22 * sin(x * 23.0 + fold * 2.0 + t)
                             + 0.12 * sin(x * 41.0 - t * 0.6);
        float hem = exp(-abs(height) * 65.0);
        vec3 green = vec3(0.055, 0.58, 0.36);
        vec3 violet = vec3(0.29, 0.10, 0.52);
        vec3 tint = mix(green, violet, smoothstep(0.02, 0.3, height));
        float envelope = exp(-0.22 * p.x * p.x);
        col += tint * (veil * strands * 0.46 + hem * 0.13) * envelope;
    }
    float vignette = 1.0 - 0.25 * smoothstep(0.25, 0.9, length(uv - 0.5));
    col = vec3(1.0) - exp(-col * 1.25);
    fragColor = vec4(col * vignette, 1.0);
}
