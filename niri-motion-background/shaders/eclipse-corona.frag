#version 330 core
// Adapted from https://www.subframe.com/motion/eclipse-corona
// Copyright (c) 2026 Subframe. SPDX-License-Identifier: MIT
// Changes: OpenGL 3.3 output, fixed alignment, day/night palette.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
void main() {
    vec2 p=(gl_FragCoord.xy-.5*u_resolution)/min(u_resolution.x,u_resolution.y);
    float r=length(p);
    float a=atan(p.y,p.x);
    float rays=.7+.2*sin(a*13.0+u_time*.06)+.1*sin(a*31.0-u_time*.04);
    float corona=exp(-max(r-.275,0.0)*13.0)*rays;
    vec3 bg=mix(vec3(.014,.019,.027),vec3(.002,.003,.008),u_night);
    vec3 c=bg+vec3(.83,.86,.86)*corona*mix(.48,.28,u_night);
    float sun=1.0-smoothstep(.268,.274,r);
    c+=sun*vec3(1.0,.67,.32);
    vec2 shift=vec2(.006*sin(u_time*.035),.008+.005*sin(u_time*.09));
    float moon=1.0-smoothstep(.273,.277,length(p-shift));
    c=mix(c,bg,moon);
    c+=exp(-abs(r-.28)*240.0)*.14*(1.0-moon);
    fragColor=vec4(c,1.);
}
