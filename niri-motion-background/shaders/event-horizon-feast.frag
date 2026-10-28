#version 330 core
// Original cinematic black hole illustration, MIT. Curved light rays and
// continuous emissive gas filaments; artistic approximation, no textures/audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
vec3 gas(vec3 p, float t) {
    float r=length(p.xz), a=atan(p.z,p.x);
    float orbit=a-t*0.35/pow(max(r,1.6),1.5);
    float spiral=r*23.0+orbit*4.0+1.6*sin(orbit*7.0+r*2.0-t*0.12);
    float threads=pow(0.5+0.5*sin(spiral),3.0);
    float fine=0.5+0.5*sin(r*53.0+orbit*9.0-t*0.18);
    float turbulence=0.65+0.35*sin(orbit*13.0+r*4.0+t*0.12);
    float inner=smoothstep(1.55,2.05,r);
    float outer=1.0-smoothstep(5.6,7.8,r);
    float heat=pow(2.2/max(r,1.6),1.7);
    float feeding=pow(0.5+0.5*sin(orbit*3.0+r*1.8),12.0);
    float tails=smoothstep(5.0,6.5,r)*(1.0-smoothstep(8.0,10.5,r))*feeding;
    float density=inner*outer*(0.24+0.95*threads+0.24*fine)*turbulence+tails*0.55;
    // A bright approaching side and a redder receding side suggest orbital motion.
    float doppler=clamp(1.0+0.65*p.x/max(r,0.1),0.35,1.65);
    vec3 hot=mix(vec3(1.0,0.18,0.025),vec3(1.0,0.79,0.43),clamp(heat*0.75,0.0,1.0));
    hot=mix(hot,vec3(0.85,0.92,1.0),smoothstep(0.85,1.5,heat)*0.65);
    return hot*density*(1.1+heat*4.5)*doppler;
}
vec3 background(vec3 d) {
    float cloud=sin(d.x*5.0+sin(d.z*6.0))+0.4*sin(d.y*13.0+d.z*3.0);
    return vec3(0.002,0.003,0.009)+vec3(0.018,0.012,0.027)*pow(max(cloud*0.5,0.0),3.0);
}
void main() {
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/max(u_resolution.y,1.0);
    float t=u_time;
    vec3 ro=vec3(0.35*sin(t*0.035),2.6,13.5);
    vec3 forward=normalize(vec3(0.0,0.0,0.0)-ro);
    vec3 right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*1.18+right*uv.x+up*uv.y);
    vec3 p=ro, col=vec3(0.0);
    float angular=dot(cross(ro,rd),cross(ro,rd));
    float transmission=1.0;
    bool captured=false;
    for(int i=0;i<180;i++) {
        float r=length(p);
        if(r<1.0) {captured=true;break;}
        if(r>26.0 || transmission<0.015) break;
        float ds=clamp(r*0.075,0.035,0.65);
        vec3 acceleration=-1.5*angular*p/pow(max(r,1.0),5.0);
        vec3 nextDirection=normalize(rd+acceleration*ds);
        vec3 nextP=p+normalize(rd+nextDirection)*ds;
        // Integrate disk crossings analytically rather than sampling tiny points.
        if(p.y*nextP.y<=0.0) {
            float fraction=clamp(p.y/(p.y-nextP.y),0.0,1.0);
            vec3 crossing=mix(p,nextP,fraction);
            vec3 emission=gas(crossing,t);
            float density=clamp(length(emission)*0.38,0.0,0.94);
            col+=transmission*emission;
            transmission*=1.0-density;
        }
        // Warm diffuse atmosphere gives the dense filaments a soft luminous halo.
        float diskRadius=length(p.xz);
        float mist=exp(-abs(p.y)*9.0)*smoothstep(1.6,2.1,diskRadius)
                  *(1.0-smoothstep(6.0,8.0,diskRadius));
        col+=transmission*vec3(0.24,0.075,0.015)*mist*ds*0.55;
        p=nextP;rd=nextDirection;
    }
    if(!captured) col+=transmission*background(rd);
    // Bounded broad bloom, separate from the silhouette of the event horizon.
    float radius=length(uv*vec2(1.0,1.07));
    col+=vec3(0.045,0.014,0.003)*exp(-radius*5.0)*smoothstep(0.13,0.24,radius);
    col*=mix(1.0,0.78,u_night);
    col=vec3(1.0)-exp(-col*1.15);
    col=pow(max(col,vec3(0.0)),vec3(0.4545));
    fragColor=vec4(col,1.0);
}
