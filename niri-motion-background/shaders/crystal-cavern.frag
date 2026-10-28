#version 330 core
// Original luminous crystal cave and dark reflective pool, MIT.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float crystal(vec3 p,float radius,float height){
    float hex=max(abs(p.x)*0.866025+abs(p.z)*0.5,abs(p.z));
    float side=hex-radius;
    float cap=(p.y-height+hex*1.3)*0.60;
    return max(side,max(cap,-p.y));
}
vec2 scene(vec3 p){
    vec2 cell=floor((p.xz+1.5)/3.0);
    vec3 q=vec3(mod(p.x+1.5,3.0)-1.5,p.y,mod(p.z+1.5,3.0)-1.5);
    float seed=hash(cell);
    q.xz=rot(seed*6.283)*q.xz;
    float height=1.1+seed*2.1;
    float d=100.0;
    // Keep a central water passage clear for the camera.
    float present=step(1.0,abs(cell.x));
    if(present>0.5){
        vec3 main=q;main.x-=main.y*0.12;
        d=crystal(main,0.31+seed*0.13,height);
        vec3 side=q-vec3(0.57,0,0.16);side.x-=side.y*0.24;
        d=min(d,crystal(side,0.22,height*0.65));
        side=q-vec3(-0.43,0,-0.31);side.z+=side.y*0.25;
        d=min(d,crystal(side,0.18,height*0.48));
        d*=0.65;
    }
    float wall=6.9-abs(p.x)+0.20*sin(p.z*1.4+p.y*2.0)+0.15*sin(p.y*4.0-p.z);
    float ceiling=5.5-p.y+0.24*sin(p.z*0.8+p.x);
    float rock=min(wall,ceiling);
    vec2 result=vec2(d,1.0);
    if(rock<result.x)result=vec2(rock,2.0);
    if(p.y<result.x)result=vec2(p.y,0.0);
    // Cell boundary is a conservative step bound, never a visible surface.
    float boundary=1.5-max(abs(mod(p.x+1.5,3.0)-1.5),abs(mod(p.z+1.5,3.0)-1.5));
    if(present<0.5) boundary=1.5-abs(p.x);
    result.x=min(result.x,max(0.025,boundary));
    return result;
}
vec3 normalAt(vec3 p){vec2 e=vec2(0.003,0);return normalize(vec3(scene(p+e.xyy).x-scene(p-e.xyy).x,scene(p+e.yxy).x-scene(p-e.yxy).x,scene(p+e.yyx).x-scene(p-e.yyx).x));}
vec3 environment(vec3 d){
    float panel=pow(max(dot(d,normalize(vec3(-0.7,0.6,0.3))),0.0),22.0);
    return vec3(0.025,0.035,0.08)+vec3(0.4,0.75,1.3)*panel;
}
vec3 material(vec3 p,vec3 n,vec3 rd,float id){
    if(id>1.5)return vec3(0.025,0.025,0.055)*(0.5+0.5*max(n.y,0.0));
    float seed=hash(floor((p.xz+1.5)/3.0));
    vec3 tint=mix(vec3(0.13,0.11,0.65),vec3(0.025,0.52,0.72),seed);
    float pulse=0.62+0.38*sin(u_time*0.55+seed*6.283);
    float veins=pow(0.5+0.5*sin(p.y*15.0+p.x*8.0+p.z*6.0),8.0);
    float fresnel=0.15+0.8*pow(1.0-max(dot(n,-rd),0.0),4.0);
    float diffuse=max(dot(n,normalize(vec3(-0.5,0.8,-0.4))),0.0);
    return tint*(0.14+0.35*diffuse+0.40*pulse+0.24*veins*pulse)
         +environment(reflect(rd,n))*fresnel+vec3(0.35,0.5,0.7)*pow(diffuse,30.0);
}
float trace(vec3 ro,vec3 rd,out float id){
    float distance=0.02;id=0.0;
    for(int i=0;i<256;i++){
        vec2 hit=scene(ro+rd*distance);id=hit.y;
        if(hit.x<0.004+distance*0.0002)return distance;
        if(distance>38.0)break;
        distance+=max(hit.x*0.7,0.005);
    }
    return 38.0;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(0.3*sin(u_time*0.07),1.65,-4.0+u_time*0.045);
    vec3 rd=normalize(vec3(uv.x,uv.y-0.04,1.15));
    float id;float distance=trace(ro,rd,id);
    vec3 col=vec3(0.018,0.025,0.065);
    if(distance<38.0){
        vec3 p=ro+rd*distance,n=normalAt(p);
        if(id<0.5){
            n=normalize(vec3(0.018*sin(p.z*6.0+u_time*0.5),1.0,0.012*sin(p.x*8.0-u_time*0.4)));
            vec3 reflected=reflect(rd,n);float rid;
            float d=trace(p+vec3(0,0.025,0),reflected,rid);
            vec3 reflection=environment(reflected);
            if(d<38.0){vec3 rp=p+vec3(0,0.025,0)+reflected*d;reflection=material(rp,normalAt(rp),reflected,rid);}
            float ripple=0.5+0.5*sin(p.x*9.0+p.z*4.0+u_time*0.4);
            col=vec3(0.012,0.019,0.045)+reflection*0.72+vec3(0.005,0.014,0.025)*ripple;
        }else col=material(p,n,rd,id);
        col=mix(col,vec3(0.018,0.025,0.065),1.0-exp(-distance*0.10));
    }
    col*=mix(1.0,0.8,u_night);
    fragColor=vec4(pow(col/(vec3(1.0)+col),vec3(0.55)),1.0);
}
