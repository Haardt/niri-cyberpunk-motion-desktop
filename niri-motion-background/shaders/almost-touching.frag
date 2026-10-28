#version 330 core
// Original mathematical light membranes. MIT. No textures or audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float gap(){return 0.055+0.31*(0.5+0.5*sin(u_time*0.17));}
float membrane(vec2 p,float side){
    return 0.48*sin(p.x*1.55+u_time*0.21)+0.23*sin(p.y*3.2-p.x*0.8-u_time*0.16)
          +0.17*sin(p.y*5.0+u_time*0.4)*exp(-abs(p.x)*1.8);
}
float capsule(vec3 p,vec3 a,vec3 b,float r){vec3 q=p-a,d=b-a;return length(q-d*clamp(dot(q,d)/dot(d,d),0.0,1.0))-r;}
vec2 scene(vec3 p){
    float side=p.x<0.0?-1.0:1.0;
    vec2 q=vec2(p.x-side*(1.35+gap()),p.y);
    q.y+=0.14*sin(q.x*1.4+side*0.7+u_time*0.13);
    float edge=(length(q/vec2(1.35,0.98))-1.0)*0.80;
    float sheet=max(abs(p.z-membrane(p.xy,side))-0.009,edge)*0.53;
    float g=gap();
    float ya=-0.14*sin(1.19+u_time*0.13);
    float yb=-0.14*sin(-1.19+u_time*0.13);
    vec3 a=vec3(-g,ya,membrane(vec2(-g,ya),-1.0));
    vec3 b=vec3(g,yb,membrane(vec2(g,yb),1.0));
    float bridge=capsule(p,a,b,0.010);
    bridge=max(bridge,(g-0.14)*0.8);
    return sheet<bridge?vec2(sheet,side<0.0?1.0:2.0):vec2(bridge,3.0);
}
vec3 normalAt(vec3 p){vec2 e=vec2(0.0015,0);return normalize(vec3(scene(p+e.xyy).x-scene(p-e.xyy).x,scene(p+e.yxy).x-scene(p-e.yxy).x,scene(p+e.yyx).x-scene(p-e.yyx).x));}
float trace(vec3 ro,vec3 rd,out float id,out float halo){
    float t=0.0;halo=0.0;id=0.0;
    for(int i=0;i<120;i++){
        vec3 p=ro+rd*t;vec2 d=scene(p);
        halo=max(halo,exp(-max(d.x,0.0)*16.0)*exp(-abs(p.z)*0.8));
        if(d.x<0.0018+t*0.00008){
            id=d.y;
            // Refine the continuous surface before evaluating fine linework.
            // This prevents ray-march tolerance from quantizing the fibers.
            if(id<2.5){
                float side=id<1.5?-1.0:1.0;
                for(int k=0;k<4;k++){
                    vec3 q=ro+rd*t;
                    float value=q.z-membrane(q.xy,side)-0.009;
                    vec3 ahead=q+rd*0.002;
                    float derivative=(ahead.z-membrane(ahead.xy,side)-0.009-value)/0.002;
                    if(abs(derivative)>0.1)t-=clamp(value/derivative,-0.025,0.025);
                }
            }
            return t;
        }
        t+=max(d.x*0.82,0.0015);
        if(t>14.0)break;
    }
    return 14.0;
}
vec3 studio(vec3 d){
    return vec3(0.035,0.05,0.07)+vec3(1.3,1.5,1.65)*pow(max(dot(d,normalize(vec3(-0.5,0.8,0.9))),0.0),22.0)
          +vec3(0.95,0.48,0.34)*pow(max(dot(d,normalize(vec3(0.9,0.1,0.7))),0.0),32.0);
}
vec3 shade(vec3 p,vec3 rd,float id){
    if(id>2.5)return vec3(2.3,1.9,1.35);
    vec3 n=normalAt(p);if(dot(n,rd)>0.0)n=-n;
    float facing=max(dot(n,-rd),0.0);
    vec3 tint=id<1.5?vec3(0.045,0.55,0.65):vec3(0.80,0.25,0.23);
    float flow=p.y*43.0+2.0*sin(p.x*1.9+u_time*0.2)+sin(p.x*5.0-u_time*0.3)*0.5;
    float width=clamp(130.0/u_resolution.y,0.045,0.25);
    float lines=1.0-smoothstep(0.08,0.08+width,abs(sin(flow)));
    float crossLines=1.0-smoothstep(0.03,0.03+width*0.7,abs(sin(p.x*26.0+sin(p.y*2.0))));
    float edge=pow(1.0-facing,3.0);
    float side=id<1.5?-1.0:1.0;
    vec2 local=vec2(p.x-side*(1.35+gap()),p.y);
    local.y+=0.14*sin(local.x*1.4+side*0.7+u_time*0.13);
    float perimeter=abs(length(local/vec2(1.35,0.98))-1.0);
    float hem=exp(-perimeter*90.0);
    float silk=0.5+0.5*sin(p.x*3.0+p.y*4.0+u_time*0.18);
    vec3 col=tint*(0.085+lines*0.95+crossLines*0.11+hem*1.15+silk*0.04);
    col+=studio(reflect(rd,n))*(0.055+edge*0.65);
    col+=tint*edge*0.5;
    col+=vec3(0.3,0.38,0.4)*pow(max(dot(n,normalize(vec3(-0.4,0.7,1.0))),0.0),50.0)*0.6;
    return col;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(0.22*sin(u_time*0.04),0.14,6.2);
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.45);
    vec3 rd=normalize(vec3(uv.x,uv.y,-1.42*framing));
    float id,halo;float d=trace(ro,rd,id,halo);
    vec3 col=vec3(0.004,0.009,0.018)+vec3(0.008,0.018,0.025)*exp(-dot(uv,uv)*3.0);
    if(d<14.0)col+=shade(ro+rd*d,rd,id);
    else col+=mix(vec3(0.02,0.19,0.24),vec3(0.25,0.07,0.06),smoothstep(-0.1,0.1,uv.x))*halo*0.22;
    // A subdued reflection anchors the floating membranes above a dark stage.
    if(rd.y<0.0){
        float floorT=(-1.5-ro.y)/rd.y;
        if(floorT>0.0 && floorT<14.0 && floorT<d){
            vec3 p=ro+rd*floorT,reflectDir=reflect(rd,vec3(0,1,0));float rid,rh;
            float reflected=trace(p+vec3(0,0.008,0),reflectDir,rid,rh);
            if(reflected<14.0)col+=shade(p+vec3(0,0.008,0)+reflectDir*reflected,reflectDir,rid)*0.14*exp(-floorT*0.035);
        }
    }
    col*=mix(1.0,0.82,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.1),vec3(0.65)),1.0);
}
