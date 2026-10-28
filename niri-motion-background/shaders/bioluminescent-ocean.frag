#version 330 core
// Original bioluminescent ocean, MIT; no textures or audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float waves(vec2 p) {
    float t=u_time*0.42;
    return 0.24*sin(p.x*0.85+p.y*0.52+t)+0.15*sin(p.x*1.43-p.y*0.78+t*1.3)
         +0.07*sin(p.x*2.8+p.y*1.7-t*0.8)+0.035*sin(p.x*5.1-p.y*3.1+t*1.9);
}
vec3 sky(vec3 d) {
    float haze=exp(-abs(d.y)*7.0);
    vec3 c=mix(vec3(0.006,0.012,0.035),vec3(0.02,0.09,0.13),haze);
    float moon=length(d-normalize(vec3(-0.38,0.16,1.0)));
    c+=vec3(0.55,0.72,0.78)*(1.0-smoothstep(0.014,0.02,moon));
    c+=vec3(0.03,0.07,0.09)*exp(-moon*12.0);
    return c;
}
void main() {
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(u_time*0.025,2.4,-6.0);
    vec3 rd=normalize(vec3(uv.x,uv.y-0.17,1.35));
    vec3 col=sky(rd);
    float distance=0.0;
    bool hit=false;
    for(int i=0;i<88;i++) {
        vec3 p=ro+rd*distance;
        float h=p.y-waves(p.xz);
        if(h<0.008) {hit=true;break;}
        distance+=max(0.02,h*0.65);
        if(distance>95.0) break;
    }
    if(hit) {
        vec3 p=ro+rd*distance;
        float e=0.025;
        vec3 n=normalize(vec3(waves(p.xz-vec2(e,0))-waves(p.xz+vec2(e,0)),2.0*e,waves(p.xz-vec2(0,e))-waves(p.xz+vec2(0,e))));
        float fresnel=0.06+0.85*pow(1.0-max(dot(n,-rd),0.0),4.0);
        float crest=smoothstep(0.12,0.35,waves(p.xz));
        float threads=pow(0.5+0.5*sin(p.x*13.0+p.z*7.0+sin(p.z*2.0)+u_time*0.5),4.0);
        col=vec3(0.003,0.025,0.037)+sky(reflect(rd,n))*fresnel;
        col+=vec3(0.015,0.72,0.65)*crest*(0.38+0.62*threads)*exp(-distance*0.018);
        // Broad living shapes drift below the surface, rather than point particles.
        for(int i=0;i<3;i++) {
            float f=float(i);
            vec2 center=vec2(sin(u_time*0.06+f*2.4)*5.0,4.0+f*7.0+cos(u_time*0.04+f)*3.0);
            vec2 q=(p.xz-center+n.xz*0.5)/vec2(2.1,0.8);
            float body=exp(-dot(q,q)*2.0);
            float veins=0.6+0.4*sin(q.x*12.0+u_time*0.7);
            col+=vec3(0.01,0.15,0.24)*body*veins;
        }
        col=mix(col,vec3(0.018,0.055,0.075),1.0-exp(-distance*0.014));
    }
    col*=mix(1.0,0.8,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.5),vec3(0.65)),1.0);
}
