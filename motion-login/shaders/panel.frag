#version 330 core
// Motion Login: bounded horizontal displacement and a subpixel RGB split.
// Zero amount is an exact, undistorted image. No random per-frame noise.
in vec2 uv;
out vec4 color;
uniform sampler2D panel;
uniform float amount;
uniform float phase;
void main() {
    vec2 texel = 1.0 / vec2(textureSize(panel, 0));
    float band = exp(-pow((uv.y - mix(0.24, 0.76, phase)) / 0.035, 2.0));
    float second = exp(-pow((uv.y - 0.72) / 0.012, 2.0));
    vec2 pos = uv + vec2((band - second * 0.4) * amount * 32.0 * texel.x, 0.0);
    vec4 base = texture(panel, pos);
    float split = amount * 6.0 * texel.x;
    // Respect premultiplied alpha at the angular transparent silhouette.
    base.r = min(texture(panel, pos + vec2(split, 0)).r, base.a);
    base.b = min(texture(panel, pos - vec2(split, 0)).b, base.a);
    base.rgb *= 1.0 - amount * 0.4 * band;
    color = base;
}
