#version 330 core
// Original rain glass and defocused warm city lights, MIT.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p) {return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
vec3 outside(vec2 p) {
    vec3 c=mix(vec3(0.012,0.022,0.043),vec3(0.065,0.085,0.11),smoothstep(-0.5,0.5,p.y));
    for(int i=0;i<36;i++) {
        float f=float(i);
        vec2 id=vec2(f,3.1);
        vec2 q=p-vec2((hash(id)-0.5)*2.3,(hash(id+4.0)-0.5)*0.72-0.12);
        float r=0.018+0.038*hash(id+7.0);
        float glow=exp(-dot(q,q)/(r*r*5.0));
        float bokeh=1.0-smoothstep(r*0.75,r*1.8,length(q));
        vec3 tint=mix(vec3(1.0,0.38,0.10),vec3(0.12,0.56,0.70),step(0.72,hash(id+2.0)));
        c+=tint*(bokeh*0.38+glow*0.16);
        c+=tint*exp(-abs(q.x)*45.0)*exp(-abs(q.y+0.25)*7.0)*0.025;
    }
    return c;
}
// Smooth falling beads leave thin wet trails. Phase is faded at each cell boundary.
vec3 drops(vec2 p, float scale, float speed) {
    vec2 grid=p*scale;
    vec2 cell=floor(grid);
    vec3 result=vec3(0.0);
    for(int x=-1;x<=1;x++) for(int y=-1;y<=1;y++) {
        vec2 id=cell+vec2(x,y);
        float seed=hash(id);
        float phase=fract(u_time*speed+seed*7.0);
        float life=smoothstep(0.0,0.12,phase)*(1.0-smoothstep(0.85,1.0,phase));
        vec2 center=id+vec2(0.25+0.5*seed,1.15-phase*1.35);
        vec2 q=grid-center;
        q.x+=0.035*sin(q.y*5.0+seed*30.0);
        vec2 shape=q/vec2(0.16,0.23);
        float d=length(shape);
        float bead=(1.0-smoothstep(0.78,1.0,d))*life;
        float trail=(1.0-smoothstep(0.012,0.035,abs(q.x)))
                   *smoothstep(0.0,0.16,q.y)*(1.0-smoothstep(0.3,0.85,q.y))*life*0.35;
        result.xy+=shape*bead*0.022;
        result.z+=bead+trail;
    }
    return result;
}
void main() {
    vec2 p=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 rain=drops(p,10.0,0.10)+drops(p+vec2(3.4,1.8),18.0,0.065)*0.55;
    vec3 col=outside(p+rain.xy);
    col*=1.0-clamp(rain.z,0.0,1.0)*0.23;
    col+=vec3(0.55,0.65,0.73)*max(rain.y-rain.x*0.6,0.0)*3.5;
    col+=vec3(0.08,0.11,0.13)*clamp(rain.z,0.0,1.0)*0.35;
    col*=1.0-0.18*smoothstep(0.3,1.0,length(p));
    col*=mix(1.0,0.82,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.5),vec3(0.75)),1.0);
}
