#version 330 core
// Original botanical mathematical line sculpture. MIT; no textures/audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float TAU=6.28318530718;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float segment(vec2 p,vec2 a,vec2 b){vec2 q=p-a,d=b-a;return length(q-d*clamp(dot(q,d)/max(dot(d,d),1e-6),0.0,1.0));}
vec2 project(vec3 p){float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.45);return p.xy*1.35*framing/(5.3-p.z);}
vec3 center(int i){
    if(i==0)return vec3(-1.13,0.32,0.15);
    if(i==1)return vec3(-0.25,0.97,-0.15);
    if(i==2)return vec3(0.85,0.65,-0.20);
    if(i==3)return vec3(1.55,-0.15,-0.6);
    if(i==4)return vec3(-0.45,-0.12,0.3);
    return vec3(0.55,-0.54,0.45);
}
vec3 petal(float a,float layer,float index){
    float lobes=5.0+mod(index,3.0);
    float radius=0.12+0.31*pow(0.5+0.5*cos(a*lobes+layer*0.28),0.72);
    radius*=1.0-layer*0.11;
    vec3 p=vec3(cos(a)*radius,sin(a)*radius,0.10*sin(a*lobes+layer*0.6));
    p.xz=rot(0.20*sin(u_time*0.075+index)+0.18)*p.xz;
    p.xy=rot(index*0.71+layer*0.035+0.05*sin(u_time*0.14+index))*p.xy;
    return p;
}
void ink(inout vec3 col,float distance,vec3 tint,float strength,float width){
    float aa=1.3/u_resolution.y;
    float core=1.0-smoothstep(width,width+aa,distance);
    float halo=exp(-distance*160.0);
    col+=tint*(core*0.75+halo*0.16)*strength;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 col=vec3(0.004,0.028,0.033);
    col+=vec3(0.009,0.042,0.040)*exp(-dot(uv-vec2(-0.18,0.1),uv-vec2(-0.18,0.1))*3.0);
    for(int i=0;i<6;i++){
        float index=float(i);vec3 c=center(i);
        c.x+=0.035*sin(u_time*0.16+index);
        vec3 root=vec3(c.x*0.25,-1.65,-0.15);
        float stem=10.0;
        vec2 prev=project(root);
        for(int j=1;j<=32;j++){
            float s=float(j)/32.0;
            vec3 p=mix(root,c,s);
            p.x+=0.18*sin(s*3.14159)*sin(index+u_time*0.10);
            p.z+=0.2*sin(s*3.14159);
            vec2 next=project(p);stem=min(stem,segment(uv,prev,next));prev=next;
        }
        ink(col,stem,vec3(0.34,0.56,0.36),0.5,0.0005);
        // Two continuous leaf loops along each sine-shaped stem.
        for(int leaf=0;leaf<2;leaf++){
            float side=leaf==0?-1.0:1.0;
            vec3 anchor=mix(root,c,0.34+float(leaf)*0.18);
            float leafD=10.0;vec2 last=project(anchor);
            for(int j=1;j<=28;j++){
                float a=float(j)/28.0*TAU;
                vec3 q=anchor+vec3(side*0.28*(1.0-cos(a)),0.12*sin(a)+0.16*(1.0-cos(a)),0.04*sin(a));
                vec2 next=project(q);leafD=min(leafD,segment(uv,last,next));last=next;
            }
            ink(col,leafD,vec3(0.40,0.66,0.45),0.50,0.00045);
        }
        vec2 screenCenter=project(c);
        if(length(uv-screenCenter)>0.19)continue;
        for(int layer=0;layer<6;layer++){
            float f=float(layer)*0.5,d=10.0;vec2 prevP=project(c+petal(0.0,f,index));
            for(int j=1;j<=176;j++){
                float a=TAU*float(j)/176.0;
                vec2 next=project(c+petal(a,f,index));
                d=min(d,segment(uv,prevP,next));prevP=next;
            }
            vec3 tint=mix(vec3(0.95,0.64,0.27),vec3(0.85,0.27,0.25),f*0.28);
            ink(col,d,tint,0.52,0.00045);
        }
        // A tangent travels around the flower without displacing its center.
        float a=u_time*0.13+index;
        vec2 p0=project(c+petal(a,0.0,index));
        vec2 tangent=normalize(project(c+petal(a+0.02,0.0,index))-p0);
        ink(col,segment(uv,p0-tangent*0.028,p0+tangent*0.028),vec3(0.9,0.8,0.55),0.7,0.0004);
        float ring=abs(length(uv-screenCenter)-0.009);
        ink(col,ring,vec3(0.88,0.66,0.29),0.6,0.0004);
    }
    col*=mix(1.0,0.82,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.1),vec3(0.72)),1.0);
}
