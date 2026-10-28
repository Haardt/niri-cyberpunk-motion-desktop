#version 330 core
// Original endless warm library, MIT. Geometry and book spines are procedural.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float box(vec3 p,vec3 b){vec3 d=abs(p)-b;return length(max(d,0.0))+min(max(d.x,max(d.y,d.z)),0.0);}
vec2 scene(vec3 p){
    float wall=3.35-abs(p.x);
    float d=min(p.y,min(6.0-p.y,wall));float id=0.0;
    vec3 q=vec3(abs(p.x),p.y,p.z);
    float tier=floor((q.y-0.16)/0.72);
    float shelfY=mod(q.y-0.16,0.72)-0.36;
    float shelf=max(box(vec3(q.x-2.82,shelfY,0),vec3(0.53,0.035,100)),max(0.16-q.y,q.y-5.8));
    if(shelf<d){d=shelf;id=1.0;}
    float bookId=floor(q.z/0.19);
    float seed=hash(vec2(bookId,tier));
    float bookHeight=0.39+0.22*seed;
    float bookBase=tier*0.72+0.16+0.36+0.04;
    vec3 b=vec3(q.x-(2.77+0.07*seed),q.y-bookBase-bookHeight*0.5,mod(q.z,0.19)-0.095);
    float books=box(b,vec3(0.30,bookHeight*0.5,0.077+0.008*seed));
    books=max(books,max(0.58-q.y,q.y-5.7));
    if(books<d){d=books;id=2.0;}
    float z=mod(q.z+1.8,3.6)-1.8;
    float pillar=box(vec3(q.x-2.24,q.y-1.7,z),vec3(0.10,1.7,0.11));
    float arc=max(abs(length(vec2(p.x,p.y-3.4))-2.24)-0.10,max(3.4-p.y,abs(z)-0.11));
    float frame=min(pillar,arc);
    if(frame<d){d=frame;id=1.0;}
    float lampZ=mod(p.z+2.4,4.8)-2.4;
    float lamp=length(vec3(p.x,p.y-4.65,lampZ))-0.16;
    if(lamp<d){d=lamp;id=3.0;}
    float wire=max(length(vec2(p.x,lampZ))-0.012,abs(p.y-5.32)-0.66);
    if(wire<d){d=wire;id=1.0;}
    return vec2(d,id);
}
vec3 normalAt(vec3 p){vec2 e=vec2(0.002,0);return normalize(vec3(scene(p+e.xyy).x-scene(p-e.xyy).x,scene(p+e.yxy).x-scene(p-e.yxy).x,scene(p+e.yyx).x-scene(p-e.yyx).x));}
vec3 shade(vec3 p,vec3 n,vec3 rd,float id){
    vec3 base=vec3(0.18,0.095,0.042);
    if(id<0.5){
        float planks=0.5+0.5*sin(p.x*17.0+0.12*sin(p.z*7.0));
        base=vec3(0.20,0.12,0.065)*(0.7+0.3*planks);
    }else if(id>1.5 && id<2.5){
        vec2 book=vec2(floor(p.z/0.19),floor((p.y-0.16)/0.72));
        float seed=hash(book);
        base=mix(vec3(0.12,0.21,0.16),vec3(0.36,0.07,0.035),seed);
        float bands=1.0-smoothstep(0.012,0.022,abs(mod(p.y-0.16,0.72)-0.49));
        bands=max(bands,1.0-smoothstep(0.008,0.014,abs(mod(p.y-0.16,0.72)-0.13)));
        float titles=(1.0-smoothstep(0.003,0.012,abs(mod(p.y*1.0,0.055)-0.027)))
                    *step(0.025,mod(p.z,0.19))*step(mod(p.z,0.19),0.16);
        base=mix(base,vec3(0.63,0.44,0.17),max(bands,titles*0.35));
    }
    if(id>2.5)return vec3(3.5,1.9,0.65);
    vec3 lightColor=vec3(0.12,0.075,0.036);
    float spec=0.0;
    float nearest=floor((p.z+2.4)/4.8)*4.8;
    for(int i=-1;i<=1;i++){
        vec3 light=vec3(0,4.65,nearest+float(i)*4.8)-p;
        float distance=length(light);vec3 l=light/distance;
        float diffuse=max(dot(n,l),0.0);
        lightColor+=vec3(2.4,1.45,0.65)*(0.1+diffuse)/(1.0+distance*distance*0.22);
        spec+=pow(max(dot(n,normalize(l-rd)),0.0),48.0)/(1.0+distance*distance*0.2);
    }
    float ao=clamp(scene(p+n*0.12).x/0.12,0.15,1.0);
    return base*lightColor*(0.5+0.5*ao)+vec3(0.24,0.14,0.05)*spec;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(0.18*sin(u_time*0.07),2.45,-2.0+u_time*0.10);
    vec3 rd=normalize(vec3(uv.x,uv.y+0.015,1.12));
    float distance=0.0;vec2 hit=vec2(0);
    for(int i=0;i<155;i++){
        hit=scene(ro+rd*distance);
        if(hit.x<0.0025 || distance>45.0)break;
        distance+=max(hit.x*0.72,0.003);
    }
    vec3 col=vec3(0.035,0.018,0.008);
    if(distance<45.0 && hit.x<0.01){
        vec3 p=ro+rd*distance;
        col=shade(p,normalAt(p),rd,hit.y);
        col=mix(col,vec3(0.10,0.055,0.023),1.0-exp(-distance*0.036));
    }
    for(int i=0;i<24;i++){
        float f=float(i);
        vec2 center=vec2((hash(vec2(f,2.0))-0.5)*1.7,(hash(vec2(f,4.0))-0.5)*1.0);
        center+=0.015*vec2(sin(u_time*0.16+f),cos(u_time*0.12+f));
        col+=vec3(0.25,0.15,0.05)*exp(-dot(uv-center,uv-center)*90000.0)*(0.5+0.5*sin(f+u_time*0.2));
    }
    col*=mix(1.0,0.83,u_night);
    fragColor=vec4(pow(col/(vec3(1.0)+col),vec3(0.4545)),1.0);
}
