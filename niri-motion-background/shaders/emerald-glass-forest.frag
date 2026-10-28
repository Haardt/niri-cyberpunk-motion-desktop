#version 330 core
// Original emerald glass grove, MIT. Procedural faceted trees and mist.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float capsule(vec3 p,vec3 a,vec3 b,float r){vec3 q=p-a,d=b-a;return length(q-d*clamp(dot(q,d)/dot(d,d),0.0,1.0))-r;}
vec2 scene(vec3 p){
    vec2 cell=floor((p.xz+2.0)/4.0);
    vec3 q=vec3(mod(p.x+2.0,4.0)-2.0,p.y,mod(p.z+2.0,4.0)-2.0);
    float seed=hash(cell);
    q.xz-=(vec2(hash(cell+3.1),hash(cell+7.9))-0.5)*0.55;
    q.y/=0.85+seed*0.5;
    float sway=sin(u_time*0.25+cell.x+cell.y)*0.04;
    q.x-=sway*max(q.y-1.5,0.0);
    float trunk=capsule(q,vec3(0,0,0),vec3(0,5.0,0),0.12);
    trunk=min(trunk,capsule(q,vec3(0,2.4,0),vec3(0.8,4.5,0.1),0.065));
    trunk=min(trunk,capsule(q,vec3(0,3.0,0),vec3(-0.7,4.9,-0.2),0.055));
    trunk=min(trunk,capsule(q,vec3(0,3.6,0),vec3(0.1,5.0,0.75),0.05));
    vec3 leaf=(q-vec3(0,5.0,0))/vec3(1.0,1.65,1.0);
    float crown=(abs(leaf.x)+abs(leaf.y)+abs(leaf.z)-1.0)*0.53;
    float tree=min(trunk,crown);
    float border=2.0-max(abs(mod(p.x+2.0,4.0)-2.0),abs(mod(p.z+2.0,4.0)-2.0));
    tree=min(tree,max(0.025,border));
    return tree<p.y?vec2(tree,1.0):vec2(p.y,0.0);
}
vec3 normalAt(vec3 p){vec2 e=vec2(0.003,0);return normalize(vec3(scene(p+e.xyy).x-scene(p-e.xyy).x,scene(p+e.yxy).x-scene(p-e.yxy).x,scene(p+e.yyx).x-scene(p-e.yyx).x));}
vec3 environment(vec3 d){
    float light=pow(max(dot(d,normalize(vec3(-0.45,0.8,0.3))),0.0),18.0);
    float shafts=pow(0.5+0.5*sin(d.x*23.0+d.z*9.0),10.0);
    return mix(vec3(0.025,0.10,0.075),vec3(0.38,0.58,0.40),smoothstep(-0.2,0.8,d.y))
          +vec3(1.2,1.0,0.55)*light+vec3(0.16,0.27,0.17)*shafts*max(d.y,0.0);
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(1.55,2.1,-6.0+u_time*0.035);
    vec3 rd=normalize(vec3(uv.x,uv.y+0.12,1.25));
    float distance=0.0;vec2 hit=vec2(0);
    for(int i=0;i<145;i++){
        hit=scene(ro+rd*distance);
        if(hit.x<0.003 || distance>48.0)break;
        distance+=max(hit.x*0.65,0.006);
    }
    vec3 col=environment(rd)*0.27;
    if(distance<48.0 && hit.x<0.01){
        vec3 p=ro+rd*distance,n=normalAt(p);
        if(hit.y>0.5){
            vec3 reflection=environment(reflect(rd,n));
            vec3 refraction=environment(refract(rd,n,0.67));
            float fresnel=0.12+0.75*pow(1.0-max(dot(n,-rd),0.0),4.0);
            float interior=0.5+0.5*sin(p.y*8.0+p.x*3.0+p.z*4.0);
            col=mix(refraction*vec3(0.08,0.68,0.37),reflection,fresnel);
            col+=vec3(0.04,0.25,0.13)*pow(interior,5.0);
            col+=vec3(0.5,0.85,0.62)*pow(max(dot(n,normalize(vec3(-0.4,0.8,-0.1))),0.0),32.0);
        }else{
            float moss=0.5+0.5*sin(p.x*4.0)*sin(p.z*3.0);
            col=mix(vec3(0.015,0.055,0.037),vec3(0.045,0.14,0.07),moss);
            col+=environment(reflect(rd,n))*0.07;
        }
        col=mix(col,vec3(0.13,0.25,0.19),1.0-exp(-distance*0.065));
    }
    float shafts=pow(max(0.0,sin((uv.x+uv.y*0.45)*31.0+sin(uv.y*3.0))),12.0);
    col+=vec3(0.10,0.13,0.065)*shafts*smoothstep(-0.5,0.5,uv.y);
    col*=mix(1.0,0.67,u_night);
    fragColor=vec4(pow(col/(vec3(1.0)+col),vec3(0.55)),1.0);
}
