#version 330 core
// Original dream sculpture, MIT. A continuous (2,3) torus-braid ribbon with
// pearlescent interference, real secondary reflections and orbiting light threads.
// Self-contained; no images, textures, sound, particles or external assets.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
vec3 local(vec3 p){
    p.xz=rot(u_time*0.032+0.35)*p.xz;
    p.xy=rot(0.32+0.10*sin(u_time*0.055))*p.xy;
    p.yz=rot(0.48)*p.yz;
    return p;
}
vec2 scene(vec3 world){
    vec3 p=local(world);
    float radial=length(p.xz),angle=atan(p.z,p.x);
    // The two branches exchange at the atan seam, forming one closed trefoil.
    float braid=angle*1.5+0.14*sin(u_time*0.07);
    float major=1.16+0.035*sin(u_time*0.10);
    float minor=0.46+0.025*sin(u_time*0.12+angle*2.0);
    vec2 crossSection=rot(braid)*vec2(radial-major,p.y);
    float width=0.31+0.025*sin(u_time*0.08+angle*2.0);
    vec2 sizes=vec2(width,0.095);
    float a=(length((crossSection-vec2(minor,0))/sizes)-1.0)*0.095;
    float b=(length((crossSection+vec2(minor,0))/sizes)-1.0)*0.095;
    float ribbon=min(a,b)*0.47;
    vec3 q=world;q.xy=rot(0.52+u_time*0.012)*q.xy;q.yz=rot(0.65)*q.yz;
    float arc=length(vec2(length(q.xz)-2.05,q.y-0.10*sin(atan(q.z,q.x)*3.0+u_time*0.04)))-0.012;
    q=world;q.yz=rot(1.10)*q.yz;q.xz=rot(-0.45+u_time*0.009)*q.xz;
    float arc2=length(vec2(length(q.xy)-1.97,q.z-0.11*sin(atan(q.y,q.x)*2.0-u_time*0.03)))-0.010;
    float threads=min(arc,arc2);
    return ribbon<threads?vec2(ribbon,1.0):vec2(threads,2.0);
}
vec3 normalAt(vec3 p){
    const vec2 e=vec2(0.0015,-0.0015);
    return normalize(e.xyy*scene(p+e.xyy).x+e.yyx*scene(p+e.yyx).x
                    +e.yxy*scene(p+e.yxy).x+e.xxx*scene(p+e.xxx).x);
}
float trace(vec3 ro,vec3 rd,int budget,out float id,out float halo){
    // Exact outer bound avoids spending surface evaluations in empty space.
    float b=dot(ro,rd),disc=b*b-dot(ro,ro)+2.25*2.25;
    id=0.0;halo=0.0;
    if(disc<0.0)return 20.0;
    float distance=max(0.005,-b-sqrt(disc));float end=-b+sqrt(disc);
    for(int i=0;i<200;i++){
        if(i>=budget||distance>end)break;
        vec2 d=scene(ro+rd*distance);
        halo=max(halo,exp(-max(d.x,0.0)*34.0));
        if(d.x<0.00055+distance*0.000015){
            for(int j=0;j<4;j++){
                vec3 p=ro+rd*distance;float value=scene(p).x;
                float derivative=(scene(p+rd*0.0015).x-value)/0.0015;
                if(abs(derivative)>0.025)distance-=clamp(value/derivative,-0.02,0.02);
            }
            id=scene(ro+rd*distance).y;return distance;
        }
        distance+=max(d.x*0.88,0.0007);
    }
    return 20.0;
}
vec3 studio(vec3 d){
    vec3 color=mix(vec3(0.07,0.08,0.14),vec3(0.30,0.34,0.44),smoothstep(-0.25,0.9,d.y));
    float key=pow(max(dot(d,normalize(vec3(-0.65,0.65,0.45))),0.0),22.0);
    float rose=pow(max(dot(d,normalize(vec3(0.8,0.15,-0.4))),0.0),15.0);
    float cyan=exp(-pow((d.x+0.30)*11.0,2.0))*smoothstep(0.1,0.8,d.y);
    float gold=exp(-pow((d.z-0.45)*14.0,2.0))*smoothstep(0.0,0.5,d.y);
    return color+vec3(3.2,2.9,2.4)*key+vec3(0.75,0.18,0.46)*rose
                +vec3(0.24,0.70,1.05)*cyan+vec3(1.25,0.75,0.30)*gold;
}
vec3 material(vec3 p,vec3 rd,vec3 reflection,float id){
    vec3 n=normalAt(p);
    if(id>1.5){
        float flow=0.65+0.35*sin(p.x*2.0+p.y*2.3-u_time*0.18);
        return mix(vec3(0.25,0.60,0.80),vec3(1.0,0.65,0.29),flow)*(0.45+0.45*pow(abs(dot(n,-rd)),3.0));
    }
    float facing=clamp(dot(n,-rd),0.0,1.0);
    float fresnel=0.14+0.78*pow(1.0-facing,4.0);
    vec3 q=local(p);float angle=atan(q.z,q.x);
    float phase=4.5*(1.0-facing)+q.y*1.4+0.8*sin(angle*2.0)+u_time*0.025;
    vec3 film=0.60+0.40*cos(vec3(0.0,2.0944,4.1888)+phase);
    vec3 pearl=mix(vec3(0.80,0.83,0.91),film,0.52);
    vec3 transmitted=studio(refract(rd,n,0.74));
    vec3 light=normalize(vec3(-0.5,0.8,0.4));
    float diffuse=max(dot(n,light),0.0);
    float highlight=pow(max(dot(n,normalize(light-rd)),0.0),90.0);
    vec3 color=pearl*(0.20+diffuse*0.30)+reflection*mix(vec3(0.55),film,fresnel*0.65)*(0.43+fresnel*0.78);
    color+=transmitted*pearl*(0.18+0.10*facing);
    color+=vec3(0.95,0.83,0.64)*highlight*0.65;
    float flowing=pow(0.5+0.5*sin(angle*2.0+q.y*3.5-u_time*0.14),12.0);
    color+=mix(vec3(0.035,0.12,0.21),vec3(0.24,0.07,0.13),sin(angle)*0.5+0.5)*flowing;
    float ao=clamp(scene(p+n*0.16).x/0.050,0.12,1.0);
    return color*(0.48+ao*0.52);
}
vec3 sculpture(vec3 ro,vec3 rd,int budget,out float hitDistance,out float halo){
    float id;hitDistance=trace(ro,rd,budget,id,halo);
    if(hitDistance>=20.0)return vec3(0.0);
    vec3 p=ro+rd*hitDistance,n=normalAt(p),reflected=reflect(rd,n);
    vec3 reflection=studio(reflected);
    if(id<1.5){
        float rid,rh;vec3 start=p+n*0.018;
        float r=trace(start,reflected,128,rid,rh);
        if(r<20.0){
            vec3 rp=start+reflected*r,rn=normalAt(rp);
            vec3 other=material(rp,reflected,studio(reflect(reflected,rn)),rid);
            reflection=mix(other,reflection,0.18+0.4*(1.0-exp(-r*0.15)));
        }
    }
    return material(p,rd,reflection,id);
}
vec3 render(vec2 uv){
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.45);
    float angle=0.20*sin(u_time*0.035);
    vec3 ro=vec3(sin(angle)*7.4,1.1,cos(angle)*7.4);
    vec3 forward=normalize(-ro),right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*(1.48*framing)+right*uv.x+up*uv.y);
    float distance,halo;vec3 object=sculpture(ro,rd,200,distance,halo);
    vec3 background=vec3(0.006,0.009,0.023);
    background+=vec3(0.025,0.008,0.037)*exp(-dot(uv-vec2(0.22,0.12),uv-vec2(0.22,0.12))*4.0);
    background+=vec3(0.008,0.026,0.033)*exp(-dot(uv+vec2(0.30,0.02),uv+vec2(0.30,0.02))*3.0);
    vec3 color=background;
    if(distance<20.0)color=object;
    else color+=mix(vec3(0.025,0.08,0.14),vec3(0.12,0.035,0.08),smoothstep(-0.5,0.5,uv.x))*halo*0.35;
    if(rd.y<-0.0001){
        float floorDistance=(-2.22-ro.y)/rd.y;
        if(floorDistance>0.0&&floorDistance<distance&&floorDistance<20.0){
            vec3 p=ro+rd*floorDistance;
            vec3 n=normalize(vec3(0.004*sin(p.z*3.0+u_time*0.08),1,0.003*sin(p.x*3.0-u_time*0.06)));
            float reflectedDistance,reflectedHalo;
            vec3 reflection=sculpture(p+vec3(0,0.015,0),reflect(rd,n),100,reflectedDistance,reflectedHalo);
            color+=reflection*0.19*exp(-floorDistance*0.035)*(1.0-smoothstep(12.0,20.0,floorDistance));
        }
    }
    color*=mix(1.0,0.86,u_night);
    color=vec3(1.0)-exp(-color*1.48);
    return pow(max(color,vec3(0.0)),vec3(0.67));
}

void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 color=vec3(0.0);
    for(int y=0;y<2;y++)for(int x=0;x<2;x++){
        color+=render(uv+(vec2(x,y)-0.5)*0.5/u_resolution.y);
    }
    fragColor=vec4(color*0.25,1.0);
}
