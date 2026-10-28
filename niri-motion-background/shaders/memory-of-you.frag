#version 330 core
// Original reflective liquid with analytic, time-lagged wake history. MIT.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
vec2 path(float t){return vec2(1.65*sin(t*0.19)+0.35*sin(t*0.41),1.4*cos(t*0.15));}
float heightAt(vec2 p){
    float t=u_time;
    float h=0.025*sin(p.x*1.9+p.y*1.1-t*0.18);
    for(int i=0;i<12;i++){
        float age=float(i)*1.6;
        vec2 q=p-path(t-age);
        float radius=length(q);
        float traveling=radius-age*0.10;
        h+=0.034*cos(traveling*9.0)*exp(-abs(traveling)*1.8)*exp(-age*0.13);
    }
    return h;
}
float segment(vec2 p,vec2 a,vec2 b){vec2 d=b-a;return length(p-a-d*clamp(dot(p-a,d)/max(dot(d,d),1e-6),0.0,1.0));}
vec3 studio(vec3 d){
    vec3 c=mix(vec3(0.003,0.004,0.009),vec3(0.035,0.045,0.075),smoothstep(-0.1,0.8,d.y));
    float gold=exp(-pow((d.x+0.43)*20.0,2.0))*smoothstep(0.10,0.35,d.y);
    float core=exp(-pow((d.x+0.43)*70.0,2.0))*smoothstep(0.10,0.35,d.y);
    float blue=exp(-pow((d.z-0.46)*28.0,2.0))*smoothstep(0.3,0.6,d.y);
    float rim=pow(max(dot(d,normalize(vec3(0.6,0.4,-0.6))),0.0),50.0);
    return c+vec3(2.7,1.3,0.5)*gold+vec3(5.0,4.3,3.5)*core+vec3(0.30,0.55,1.4)*blue+vec3(0.8,0.35,0.6)*rim;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(3.0,4.5,5.4);
    vec3 f=normalize(vec3(0,0,0)-ro),r=normalize(cross(f,vec3(0,1,0))),up=cross(r,f);
    vec3 rd=normalize(f*1.32+r*uv.x+up*uv.y);
    float d=0.0;bool hit=false;
    for(int i=0;i<85;i++){
        vec3 p=ro+rd*d;float h=p.y-heightAt(p.xz);
        if(h<0.002){hit=true;break;}
        d+=max(h*0.60,0.003);
        if(d>28.0)break;
    }
    vec3 col=vec3(0.006,0.008,0.018);
    if(hit){
        vec3 p=ro+rd*d;float e=0.012;
        vec3 n=normalize(vec3(heightAt(p.xz-vec2(e,0))-heightAt(p.xz+vec2(e,0)),2.0*e,heightAt(p.xz-vec2(0,e))-heightAt(p.xz+vec2(0,e))));
        float fresnel=0.08+0.85*pow(1.0-max(dot(n,-rd),0.0),4.0);
        col=vec3(0.002,0.003,0.006)+studio(reflect(rd,n))*(0.3+fresnel);
        float trail=10.0,oldTrail=10.0;
        vec2 prev=path(u_time);
        for(int i=1;i<=32;i++){
            float age=float(i)*0.48;vec2 next=path(u_time-age);
            float line=segment(p.xz,prev,next);
            if(i<=7)trail=min(trail,line+float(i)*0.002);
            oldTrail=min(oldTrail,line+age*0.009);
            prev=next;
        }
        float aa=max(0.003,d*1.2/u_resolution.y);
        float ribbon=1.0-smoothstep(0.012,0.012+aa,trail);
        col+=vec3(1.4,0.70,0.25)*(ribbon*0.9+exp(-trail*15.0)*0.16);
        col+=vec3(0.15,0.25,0.58)*exp(-oldTrail*12.0)*0.18;
        col=mix(col,vec3(0.009,0.014,0.027),1.0-exp(-d*0.025));
    }
    col*=mix(1.0,0.85,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.25),vec3(0.65)),1.0);
}
