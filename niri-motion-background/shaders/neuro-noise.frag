// Adapted from Paper Shaders, Copyright Lost Coast Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
// Changes: OpenGL 3.3, procedural noise, fixed low-cost presets, day/night palettes.
#version 330 core
uniform vec2 u_resolution;
uniform float u_night;

uniform float u_time;

vec4 u_colorFront;
vec4 u_colorMid;
vec4 u_colorBack;
float u_brightness;
float u_contrast;



out vec4 fragColor;


vec2 rotate(vec2 uv, float th) {
  return mat2(cos(th), sin(th), -sin(th), cos(th)) * uv;
}


float neuroShape(vec2 uv, float t) {
  vec2 sine_acc = vec2(0.);
  vec2 res = vec2(0.);
  float scale = 8.;

  for (int j = 0; j < 12; j++) {
    uv = rotate(uv, 1.);
    sine_acc = rotate(sine_acc, 1.);
    vec2 layer = uv * scale + float(j) + sine_acc - t;
    sine_acc += sin(layer);
    res += (.5 + .5 * cos(layer)) / scale;
    scale *= (1.2);
  }
  return res.x + res.y;
}

void main() {
  u_colorBack = vec4(mix(vec3(.025,.04,.065),vec3(.004,.006,.014),u_night),1.);
  u_colorFront = vec4(mix(vec3(.6,.9,.9),vec3(.3,.7,.65),u_night),1.);
  u_colorMid = vec4(mix(vec3(.08,.35,.5),vec3(.04,.16,.22),u_night),1.);
  u_brightness = .05;
  u_contrast = .3;
  vec2 shape_uv = ((gl_FragCoord.xy-.5*u_resolution)/min(u_resolution.x,u_resolution.y))*12.;
  shape_uv *= .13;

  float t = .5 * u_time;

  float noise = neuroShape(shape_uv, t);

  noise = (1. + u_brightness) * noise * noise;
  noise = pow(noise, .7 + 6. * u_contrast);
  noise = min(1.4, noise);

  float blend = smoothstep(0.7, 1.4, noise);

  vec4 frontC = u_colorFront;
  frontC.rgb *= frontC.a;
  vec4 midC = u_colorMid;
  midC.rgb *= midC.a;
  vec4 blendFront = mix(midC, frontC, blend);

  float safeNoise = max(noise, 0.0);
  vec3 color = blendFront.rgb * safeNoise;
  float opacity = clamp(blendFront.a * safeNoise, 0., 1.);

  vec3 bgColor = u_colorBack.rgb * u_colorBack.a;
  color = color + bgColor * (1. - opacity);
  opacity = opacity + u_colorBack.a * (1. - opacity);

  
  color += 1. / 256. * (fract(sin(dot(.014 * gl_FragCoord.xy, vec2(12.9898, 78.233))) * 43758.5453123) - .5);


  fragColor = vec4(color, opacity);
}
