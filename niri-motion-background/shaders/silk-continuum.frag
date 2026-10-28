#version 330 core
// Original fullscreen dream environment, MIT. The camera is inside an unbounded
// folded opalescent passage, with gyroid glass veils, depth fog and self-reflections.
// No central object, stage, frame, particles, external assets or audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
vec3 path(float z){return vec3(0.68*sin(z*0.26)+0.20*sin(z*0.70),0.40*cos(z*0.19)+0.15*sin(z*0.45),z);}
vec2 scene(vec3 p){
    vec3 q=p-path(p.z);q.z=p.z;
    float radius=length(q.xy),angle=atan(q.y,q.x);
    float twist=angle+p.z*0.28+0.10*sin(u_time*0.035);
    float wallRadius=1.52+0.30*sin(twist*3.0+p.z*0.14)+0.14*sin(twist*5.0-p.z*0.22);
    float wall=(wallRadius-radius)*0.37;
    vec3 g=vec3(q.x*1.7,q.y*1.5,p.z*0.9+0.045*u_time);
    float gyroid=dot(sin(g),cos(g.yzx));
    float veil=max((abs(gyroid)-0.064)*0.25,(0.68-radius)*0.48);
    return wall<veil?vec2(wall,1.0):vec2(veil,2.0);
}
vec3 normalAt(vec3 p){
    const vec2 e=vec2(0.0015,-0.0015);
    return normalize(e.xyy*scene(p+e.xyy).x+e.yyx*scene(p+e.yyx).x
                    +e.yxy*scene(p+e.yxy).x+e.xxx*scene(p+e.xxx).x);
}
float trace(vec3 ro,vec3 rd,int budget,out float id){
    float distance=0.012;id=0.0;
    bool outside=scene(ro+rd*distance).x>0.001;
    for(int i=0;i<180;i++){
        if(i>=budget||distance>22.0)break;
        vec3 p=ro+rd*distance;vec2 d=scene(p);
        // Transmitted rays first leave the current thin glass veil.
        if(!outside){
            if(d.x>0.0015)outside=true;
            else{distance+=max(abs(d.x)*0.85,0.004);continue;}
        }
        if(d.x<0.0007+distance*0.00005){
            for(int k=0;k<3;k++){
                p=ro+rd*distance;float value=scene(p).x;
                float derivative=(scene(p+rd*0.002).x-value)/0.002;
                if(abs(derivative)>0.03)distance-=clamp(value/derivative,-0.02,0.02);
            }
            id=scene(ro+rd*distance).y;return distance;
        }
        distance+=max(d.x*0.82,0.001);
    }
    return 22.0;
}
vec3 atmosphere(vec3 rd){
    float phase=rd.x*1.6+rd.y*0.8+0.15*sin(u_time*0.025);
    vec3 spectral=0.58+0.42*cos(vec3(0.0,2.0944,4.1888)+phase+0.6);
    return vec3(0.085,0.11,0.18)+spectral*vec3(0.16,0.13,0.16);
}
vec3 studio(vec3 rd){
    float key=pow(max(dot(rd,normalize(vec3(-0.5,0.7,-0.2))),0.0),22.0);
    float cyan=exp(-pow((rd.x+0.25)*8.0,2.0))*smoothstep(-0.3,0.65,rd.y);
    float rose=pow(max(dot(rd,normalize(vec3(0.6,-0.3,0.4))),0.0),15.0);
    return atmosphere(rd)*0.7+vec3(2.4,2.1,1.7)*key+vec3(0.20,0.70,1.10)*cyan+vec3(0.80,0.18,0.42)*rose;
}
vec3 material(vec3 p,vec3 rd,vec3 reflected,float id){
    vec3 n=normalAt(p);if(dot(n,rd)>0.0)n=-n;
    float facing=clamp(dot(n,-rd),0.0,1.0),fresnel=0.10+0.70*pow(1.0-facing,4.0);
    vec3 q=p-path(p.z);float angle=atan(q.y,q.x);
    float phase=angle*0.7+p.z*0.32+2.8*(1.0-facing)+0.12*sin(u_time*0.08);
    vec3 film=0.50+0.48*cos(vec3(0.0,2.0944,4.1888)+phase);
    vec3 pearl=mix(vec3(0.80,0.83,0.93),film,0.76);
    vec3 lamp=path(p.z+4.5)+vec3(-0.7,0.5,0.0),light=normalize(lamp-p);
    float diffuse=max(dot(n,light),0.0);
    float specular=pow(max(dot(n,normalize(light-rd)),0.0),75.0);
    vec3 color=pearl*(0.075+diffuse*0.25)+reflected*mix(vec3(0.70),film,0.42)*(0.23+fresnel*0.62);
    color+=vec3(1.20,1.05,0.86)*specular*0.80;
    float flow=pow(0.5+0.5*sin(p.z*1.7+angle*2.0-u_time*0.09),10.0);
    color+=mix(vec3(0.045,0.14,0.24),vec3(0.24,0.09,0.16),film.r)*flow*0.45;
    float ao=clamp(scene(p+n*0.12).x/0.038,0.20,1.0);
    color+=film*pow(1.0-facing,3.0)*0.30;
    color*=0.25+0.75*ao;
    if(id>1.5)color=mix(color,studio(refract(rd,n,0.78))*pearl,0.18);
    return color;
}
vec3 secondary(vec3 ro,vec3 rd,int budget){
    float id;float d=trace(ro,rd,budget,id);
    vec3 fog=atmosphere(rd);
    if(d>=22.0)return fog;
    vec3 p=ro+rd*d,n=normalAt(p);
    vec3 color=material(p,rd,studio(reflect(rd,n)),id);
    return mix(color,fog,1.0-exp(-d*0.075));
}
vec3 render(vec2 uv){
    float z=u_time*0.075;
    vec3 ro=path(z);
    vec3 target=path(z+3.1)+vec3(0.12*sin(u_time*0.035),0.10*cos(u_time*0.029),0.0);
    vec3 forward=normalize(target-ro),right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*1.06+right*uv.x+up*uv.y);
    float id;float d=trace(ro,rd,180,id);
    vec3 fog=atmosphere(rd),color=fog;
    if(d<22.0){
        vec3 p=ro+rd*d,n=normalAt(p);if(dot(n,rd)>0.0)n=-n;
        vec3 reflection=secondary(p+n*0.018,reflect(rd,n),80);
        color=material(p,rd,reflection,id);
        if(id>1.5){
            vec3 transmitted=refract(rd,n,0.80);
            vec3 behind=secondary(p+transmitted*0.025,transmitted,110);
            float transparency=0.24+0.12*pow(max(dot(n,-rd),0.0),2.0);
            color=mix(color,behind,transparency);
        }
        color=mix(color,fog,1.0-exp(-d*0.085));
    }
    color*=mix(1.0,0.88,u_night);
    return pow(vec3(1.0)-exp(-max(color,vec3(0.0))*1.65),vec3(0.78));
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    // Edge-to-edge perspective: portrait naturally crops the same environment.
    vec3 color=vec3(0.0);
    for(int y=0;y<2;y++)for(int x=0;x<2;x++){
        color+=render(uv+(vec2(x,y)-0.5)*0.5/u_resolution.y);
    }
    fragColor=vec4(color*0.25,1.0);
}
