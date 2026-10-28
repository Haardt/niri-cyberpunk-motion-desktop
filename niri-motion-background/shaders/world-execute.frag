#version 330 core
// Original breathing glass landscape with continuous contour lines. MIT; no textures/audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.0-2.0*f);return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.0),f.x),f.y);}
float growth(){return 0.48+0.52*(0.5-0.5*cos(u_time*0.085));}
float terrain(vec2 p){
    float g=growth();
    float h=noise(p*0.42)*0.8+noise(p*0.93)*0.35+noise(p*1.8)*0.12;
    float peak=1.9*exp(-dot(p-vec2(-1.0,0.6),p-vec2(-1.0,0.6))*0.14);
    return g*g*(h+peak)*(1.0-smoothstep(3.5,7.5,length(p)));
}
vec3 normalAt(vec3 p){float e=0.015;return normalize(vec3(terrain(p.xz-vec2(e,0))-terrain(p.xz+vec2(e,0)),2.0*e,terrain(p.xz-vec2(0,e))-terrain(p.xz+vec2(0,e))));}
vec3 sky(vec3 d){return vec3(0.015,0.025,0.045)+vec3(1.5,1.6,1.8)*pow(max(dot(d,normalize(vec3(-0.6,0.8,-0.5))),0.0),32.0);}
float gridLine(float x,float footprint){float d=abs(fract(x+0.5)-0.5);return 1.0-smoothstep(0.025,0.025+footprint,d);}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    float g=growth(),angle=0.20*sin(u_time*0.025);
    vec3 ro=vec3(8.4*sin(0.6+angle),5.8,8.4*cos(0.6+angle));
    vec3 f=normalize(vec3(0,0.5,0)-ro),r=normalize(cross(f,vec3(0,1,0))),up=cross(r,f);
    vec3 rd=normalize(f*1.3+r*uv.x+up*uv.y);
    vec3 col=vec3(0.004,0.008,0.018);
    float d=0.0;bool hit=false;
    for(int i=0;i<130;i++){
        vec3 p=ro+rd*d;
        float h=p.y-terrain(p.xz);
        if(h<0.003){
            // Solve the height-field intersection accurately before drawing contours.
            for(int j=0;j<5;j++){
                vec3 q=ro+rd*d;float value=q.y-terrain(q.xz);
                vec3 ahead=q+rd*0.003;
                float derivative=(ahead.y-terrain(ahead.xz)-value)/0.003;
                if(abs(derivative)>0.08)d-=clamp(value/derivative,-0.04,0.04);
            }
            hit=true;break;
        }
        d+=max(h*0.43,0.004);
        if(d>32.0)break;
    }
    if(hit){
        vec3 p=ro+rd*d,n=normalAt(p);
        float extent=3.0+g*2.6;
        float formation=1.0-smoothstep(extent-0.4,extent+0.15,length(p.xz));
        
        float flowPhase=p.y*6.0+0.31+0.12*sin(p.x*0.9+p.z*0.6);
        float flow=gridLine(flowPhase,clamp(fwidth(flowPhase),0.008,0.25));
        float contours=gridLine(p.y*5.0+0.29,clamp(fwidth(p.y*5.0),0.008,0.25))*g;
        float rim=exp(-abs(length(p.xz)-extent)*7.0)*smoothstep(0.05,0.3,g);
        
        float fresnel=0.08+0.6*pow(1.0-max(dot(n,-rd),0.0),4.0);
        vec3 glass=sky(reflect(rd,n))*fresnel+vec3(0.015,0.045,0.055);
        vec3 lines=mix(vec3(0.04,0.55,0.63),vec3(0.70,0.48,0.19),smoothstep(0.3,2.2,p.y));
        col+=(glass+lines*(flow*0.58+contours*0.18))*formation;
        col+=vec3(0.9,0.65,0.28)*rim*0.7;
        col=mix(col,vec3(0.006,0.012,0.026),1.0-exp(-d*0.018));
    }
    col*=mix(1.0,0.82,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.2),vec3(0.65)),1.0);
}
