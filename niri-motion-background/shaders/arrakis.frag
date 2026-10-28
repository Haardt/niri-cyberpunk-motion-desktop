#version 330 core
// Original analytic desert silhouette; no 3D ray marching.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
void main(){
    vec2 uv=gl_FragCoord.xy/u_resolution;
    float aspect=u_resolution.x/u_resolution.y,t=u_time*.025;
    vec3 sky=mix(vec3(.12,.055,.045),vec3(.5,.25,.11),uv.y);
    vec3 night=mix(vec3(.008,.012,.035),vec3(.04,.055,.12),uv.y);
    vec3 col=mix(sky,night,u_night);
    vec2 sunp=(uv-vec2(.72,.67))*vec2(aspect,1.);
    col+=mix(vec3(.7,.26,.06),vec3(.2,.24,.35),u_night)*exp(-length(sunp)*16.);
    col=mix(col,mix(vec3(.94,.6,.25),vec3(.46,.52,.65),u_night),1.-smoothstep(.065,.067,length(sunp)));
    for(int i=0;i<5;i++){
        float j=float(i);float ridge=.43-j*.075+.048*sin(uv.x*(3.+j*.65)+j*2.+t*(.3+j*.08))+.025*sin(uv.x*9.+j);
        vec3 sand=mix(vec3(.28,.12,.055),vec3(.035,.022,.026),j/4.);
        sand=mix(sand,sand*vec3(.28,.4,.85),u_night);
        col=mix(col,sand,1.-smoothstep(ridge-.002,ridge+.002,uv.y));
    }
    fragColor=vec4(col,1.);
}
