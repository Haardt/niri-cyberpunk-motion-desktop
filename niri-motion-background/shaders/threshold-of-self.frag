#version 330 core
// Original fictional consciousness-breakthrough sculpture. MIT.
// A visual metaphor: braided light pushes through a segmented glass shell.
// No textures, audio, noise particles, text or external assets.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float opening(){float p=0.5+0.5*sin(u_time*0.14-0.8);return p*p;}
float effort(){return 0.5+0.5*sin(u_time*0.53+0.4*sin(u_time*0.19));}
vec3 local(vec3 p){p.xz=rot(u_time*0.045+0.3)*p.xz;return p;}
vec2 lightField(vec3 p){
    vec3 q=p;
    q.xy=rot(0.62+u_time*0.025)*q.xy;
    q.yz=rot(0.40)*q.yz;
    float a=atan(q.z,q.x),radial=length(q.xz);
    vec2 hit=vec2(100.0,2.0);
    // Three continuous interlaced toroidal fibers, with bounded slope.
    for(int i=0;i<3;i++){
        float f=float(i),phase=a*3.0+f*2.094395+u_time*0.28;
        vec2 center=vec2(0.66+0.10*opening()+0.035*effort()+0.17*cos(phase),0.17*sin(phase));
        float d=(length(vec2(radial,q.y)-center)-0.022)*0.65;
        if(d<hit.x)hit=vec2(d,2.0+f);
    }
    float core=length(p)-(0.32+0.018*sin(p.x*8.0+u_time*0.7)*sin(p.y*9.0-p.z*4.0));
    if(core<hit.x)hit=vec2(core,5.0);
    float open=opening();
    // The outward curves pass through meridian seams in the shell.
    for(int i=0;i<3;i++){
        float f=float(i);vec3 r=p;r.xz=rot(f*2.094395)*r.xz;
        float end=0.69+open*1.70;
        float s=clamp((r.x-0.55)/max(end-0.55,0.01),0.0,1.0);
        vec2 center=vec2(0.20*sin(r.x*3.0-u_time*0.20+f),0.038*sin(r.x*4.0+u_time*0.13));
        float radius=0.018*(1.0-s*0.78);
        float filament=max(length(r.yz-center)-radius,max(0.55-r.x,r.x-end))*0.65;
        if(filament<hit.x)hit=vec2(filament,6.0+f);
    }
    return hit;
}
float shellField(vec3 p){
    float r=length(p),phi=atan(p.z,p.x),theta=asin(clamp(p.y/max(r,0.001),-1.0,1.0));
    float open=opening();
    float sector=floor((phi+PI)/(PI/3.0));
    float panelShift=0.025*sin(sector*1.7)*cos(theta*3.0);
    float pressure=0.028*effort()*sin(theta*5.0+phi*3.0);
    float radius=1.17+open*(0.24+panelShift)+pressure;
    float skin=abs(r-radius)-0.022;
    float meridian=abs(sin(phi*3.0))*max(length(p.xz),0.08)/3.0;
    float latitude=abs(sin(theta*3.0))*r/3.0;
    float seam=min(meridian,latitude);
    float gap=0.013+0.095*open;
    return max(skin,gap-seam)*0.40;
}
vec2 scene(vec3 p,bool hull){
    vec3 q=local(p);vec2 d=lightField(q);
    if(hull){float h=shellField(q);if(h<d.x)d=vec2(h,1.0);}
    return d;
}
vec3 normalAt(vec3 p,bool hull){
    vec2 e=vec2(0.0018,0);
    return normalize(vec3(scene(p+e.xyy,hull).x-scene(p-e.xyy,hull).x,
                          scene(p+e.yxy,hull).x-scene(p-e.yxy,hull).x,
                          scene(p+e.yyx,hull).x-scene(p-e.yyx,hull).x));
}
float trace(vec3 ro,vec3 rd,bool hull,out float id,out float glow){
    float distance=0.02;id=0.0;glow=0.0;
    for(int i=0;i<160;i++){
        vec3 p=ro+rd*distance;vec2 d=scene(p,hull);
        if(d.y>1.5)glow=max(glow,exp(-max(d.x,0.0)*45.0));
        if(d.x<0.0012+distance*0.000035){id=d.y;return distance;}
        distance+=max(d.x*0.85,0.001);
        if(distance>13.0)break;
    }
    return 13.0;
}
vec3 studio(vec3 d){
    vec3 c=vec3(0.018,0.025,0.045);
    float panel=exp(-pow((d.x+0.48)*8.0,2.0))*smoothstep(0.1,0.7,d.y);
    float rim=pow(max(dot(d,normalize(vec3(0.7,0.3,0.7))),0.0),28.0);
    return c+vec3(0.65,0.85,1.15)*panel+vec3(0.65,0.32,0.19)*rim;
}
vec3 emission(vec3 p,vec3 rd,float id){
    vec3 n=normalAt(p,false),q=local(p);
    float facing=max(abs(dot(n,-rd)),0.0);
    float pulse=0.72+0.28*effort();
    vec3 color=id<2.5?vec3(0.035,0.65,0.95):(id<3.5?vec3(0.10,0.85,0.55):vec3(1.0,0.29,0.16));
    if(id>4.5 && id<5.5){
        float wave=0.5+0.5*sin(q.x*13.0+sin(q.y*11.0+u_time*0.4))*cos(q.y*14.0+sin(q.z*13.0-u_time*0.35));
        color=mix(vec3(0.06,0.31,0.48),vec3(0.55,0.85,0.83),wave);
        return color*(0.60+facing*0.8)*pulse;
    }
    if(id>5.5)color=mix(vec3(0.08,0.80,0.95),vec3(1.0,0.65,0.28),opening());
    return color*(1.1+1.5*facing)*pulse+vec3(0.75,0.85,0.90)*pow(facing,12.0)*0.65;
}
vec3 radiance(vec3 ro,vec3 rd){
    float id,glow;float d=trace(ro,rd,true,id,glow);
    vec3 col=vec3(0.003,0.005,0.014);
    if(d<13.0){
        vec3 p=ro+rd*d;
        if(id<1.5){
            vec3 n=normalAt(p,true);if(dot(n,rd)>0.0)n=-n;
            float facing=max(dot(n,-rd),0.0);
            float fresnel=0.08+0.75*pow(1.0-facing,4.0);
            vec3 q=local(p);float r=length(q);
            float phi=atan(q.z,q.x),theta=asin(clamp(q.y/r,-1.0,1.0));
            float seam=min(abs(sin(phi*3.0))*max(length(q.xz),0.08)/3.0,abs(sin(theta*3.0))*r/3.0);
            float edge=exp(-abs(seam-(0.013+0.095*opening()))*95.0);
            float etch=1.0-smoothstep(0.025,0.12,abs(sin(theta*35.0+0.8*sin(phi*4.0))));
            col=studio(reflect(rd,n))*fresnel+vec3(0.013,0.025,0.045);
            col+=mix(vec3(0.025,0.22,0.35),vec3(0.8,0.33,0.12),opening())*edge*(0.35+0.45*effort());
            col+=vec3(0.025,0.055,0.075)*etch;
            float innerId,innerGlow;
            vec3 inside=p+rd*0.065;
            float inner=trace(inside,rd,false,innerId,innerGlow);
            float transmission=0.30+0.22*opening();
            if(inner<13.0)col+=emission(inside+rd*inner,rd,innerId)*transmission*(1.0-fresnel*0.6);
            else col+=vec3(0.025,0.14,0.22)*innerGlow*transmission*0.18;
        }else col=emission(p,rd,id);
    }else{
        col+=mix(vec3(0.02,0.20,0.30),vec3(0.28,0.12,0.055),opening())*glow*0.20;
    }
    return col;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.4);
    vec3 ro=vec3(0.4*sin(u_time*0.035),0.30,6.0);
    vec3 f=normalize(vec3(0,0,0)-ro),r=normalize(cross(f,vec3(0,1,0))),up=cross(r,f);
    vec3 rd=normalize(f*(1.46*framing)+r*uv.x+up*uv.y);
    vec3 col=radiance(ro,rd);
    if(rd.y<-0.001){
        float floorDistance=(-1.75-ro.y)/rd.y;
        if(floorDistance>0.0 && floorDistance<18.0){
            vec3 p=ro+rd*floorDistance;
            // The floor is below the complete sculpture and its escaping fibers.
            if(p.z>-2.8){
                vec3 reflection=radiance(p+vec3(0,0.005,0),reflect(rd,vec3(0,1,0)));
                col+=reflection*0.22*exp(-floorDistance*0.018);
            }
        }
    }
    col+=vec3(0.006,0.012,0.026)*exp(-dot(uv,uv)*4.0);
    col*=mix(1.0,0.84,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.15),vec3(0.66)),1.0);
}
