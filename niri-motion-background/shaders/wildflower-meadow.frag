#version 330 core
// Original procedural windblown wildflower meadow. MIT. No external assets.
in vec2 v_uv;
out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;
uniform float u_night;

float hash(float n) { return fract(sin(n * 127.1) * 43758.5453); }
float hill(float x) { return 0.43 + 0.025 * sin(x * 2.1) + 0.012 * sin(x * 5.3); }
float wind(float x, float t) {
    return 0.58 * sin(t * 0.85 - x * 1.8)
         + 0.27 * sin(t * 1.37 - x * 3.1 + 1.3)
         + 0.15 * sin(t * 0.43 + x * 0.9);
}
float line(vec2 p, vec2 a, vec2 b) {
    vec2 ab = b - a;
    return length(p - a - ab * clamp(dot(p - a, ab) / max(dot(ab, ab), 0.000001), 0.0, 1.0));
}
void main() {
    float aspect = u_resolution.x / max(u_resolution.y, 1.0);
    vec2 p = vec2((v_uv.x - 0.5) * aspect, 1.0 - v_uv.y);
    float aa = 1.4 / max(u_resolution.y, 1.0);
    float t = u_time;
    vec3 sky = mix(vec3(0.76, 0.87, 0.88), vec3(0.27, 0.57, 0.79), smoothstep(0.4, 1.0, p.y));
    float cloud = sin(p.x * 4.0 + t * 0.015 + sin(p.y * 12.0))
                + 0.4 * sin(p.x * 8.0 - p.y * 17.0 + t * 0.023);
    sky = mix(sky, vec3(0.96, 0.95, 0.88), smoothstep(0.7, 1.35, cloud) * smoothstep(0.48, 0.7, p.y) * 0.6);
    float sun = exp(-length((p - vec2(-0.48, 0.79)) * vec2(1.0, 1.15)) * 6.0);
    sky += vec3(0.22, 0.16, 0.07) * sun;
    float horizon = hill(p.x);
    vec3 ground = mix(vec3(0.085, 0.19, 0.075), vec3(0.39, 0.49, 0.23), smoothstep(0.0, horizon, p.y));
    ground += vec3(0.025, 0.035, 0.008) * sin(p.x * 18.0 + p.y * 24.0);
    vec3 col = mix(ground, sky, smoothstep(horizon - aa, horizon + aa, p.y));

    // Back-to-front rows, with stable random placement and coherent wind.
    for (int row = 0; row < 22; ++row) {
        float depth = float(row) / 21.0;
        float scale = mix(0.07, 1.0, depth * depth);
        float spacing = 0.11 * scale;
        float baseY = 0.43 - 0.56 * depth * depth;
        float cell = floor(p.x / spacing);
        for (int neighbor = -2; neighbor <= 2; ++neighbor) {
            float id = cell + float(neighbor);
            float seed = id + float(row) * 117.7;
            float r = hash(seed + 3.7);
            float rootX = (id + 0.15 + 0.7 * hash(seed)) * spacing;
            float rootY = baseY + 0.018 * scale * hash(seed + 8.0);
            if (row < 5) rootY += hill(rootX) - 0.43;
            float height = scale * (0.12 + 0.15 * r);
            float bend = scale * (0.035 * wind(rootX, t) + 0.009 * sin(t * 1.9 + seed));
            float h = clamp((p.y - rootY) / height, 0.0, 1.0);
            float stemX = rootX + bend * h * h;
            float stem = (1.0 - smoothstep(0.0012 * scale, 0.0012 * scale + aa, abs(p.x - stemX)))
                       * smoothstep(rootY - aa, rootY + aa, p.y)
                       * (1.0 - smoothstep(rootY + height - aa, rootY + height + aa, p.y));
            vec3 green = mix(vec3(0.10, 0.24, 0.055), vec3(0.34, 0.47, 0.10), r);
            col = mix(col, green, stem);
            vec2 leafRoot = vec2(rootX + bend * 0.25, rootY + height * 0.5);
            float side = hash(seed + 2.0) > 0.5 ? 1.0 : -1.0;
            float leaf = line(p, leafRoot, leafRoot + vec2(side * 0.038 * scale, 0.035 * scale));
            col = mix(col, green, 1.0 - smoothstep(0.002 * scale, 0.002 * scale + aa, leaf));

            if (hash(seed + 14.0) > 0.32) {
                vec2 head = vec2(rootX + bend, rootY + height);
                vec2 q = p - head;
                float radius = scale * (0.014 + 0.012 * hash(seed + 9.0));
                float angle = atan(q.y, q.x) + 0.2 * wind(rootX, t);
                float petals = radius * (0.76 + 0.24 * cos(angle * (r > 0.5 ? 7.0 : 5.0)));
                float mask = 1.0 - smoothstep(petals - aa, petals + aa, length(q));
                float kind = hash(seed + 6.0);
                vec3 flower = kind < 0.28 ? vec3(0.95, 0.91, 0.76)
                            : kind < 0.51 ? vec3(0.86, 0.26, 0.32)
                            : kind < 0.76 ? vec3(0.51, 0.37, 0.79)
                            : vec3(0.98, 0.72, 0.16);
                flower *= 0.84 + 0.16 * smoothstep(-radius, radius, q.y);
                col = mix(col, flower, mask);
                float center = 1.0 - smoothstep(radius * 0.27, radius * 0.27 + aa, length(q));
                col = mix(col, vec3(0.63, 0.36, 0.065), center);
            }
        }
    }
    col *= mix(vec3(1.0), vec3(0.38, 0.48, 0.68), u_night);
    fragColor = vec4(col, 1.0);
}
