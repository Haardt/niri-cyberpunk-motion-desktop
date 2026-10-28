#version 330 core
// Original extravagant slow-motion mirror ball, MIT. No textures or audio.
// Analytic geometry, beveled spherical mirror tiles, continuously moving light.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
const vec3 CENTER=vec3(0.0,0.22,0.0);
const float RADIUS=1.10;
const float FLOOR_Y=-1.52;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
vec3 palette(float i){return 0.55+0.45*cos(vec3(0.0,2.0944,4.1888)+i*1.73);}
vec3 toLocal(vec3 p){p.xz=rot(u_time*0.038)*p.xz;return p;}
vec3 toWorld(vec3 p){p.xz=rot(-u_time*0.038)*p.xz;return p;}
float sphere(vec3 ro,vec3 rd,vec3 center,float radius){
    vec3 q=ro-center;float b=dot(q,rd),c=dot(q,q)-radius*radius;
    float disc=b*b-c;if(disc<0.0)return 100.0;
    float t=-b-sqrt(disc);return t>0.001?t:100.0;
}
vec3 club(vec3 d){
    vec3 c=mix(vec3(0.024,0.031,0.050),vec3(0.07,0.10,0.14),smoothstep(-0.2,0.9,d.y));
    c+=vec3(0.09,0.015,0.13)*exp(-pow((d.x-0.6)*3.0,2.0));
    c+=vec3(0.015,0.10,0.14)*exp(-pow((d.x+0.6)*3.0,2.0));
    // Finite broad light sources give whole facets sustained glints, not flashes.
    for(int i=0;i<16;i++){
        float f=float(i),a=f*2.39996;
        vec3 light=normalize(vec3(cos(a),0.15+0.7*sin(f*1.31),sin(a)));
        float size=0.045+0.045*hash(vec2(f,3.0));
        float source=exp(-(1.0-dot(d,light))/(size*size));
        vec3 tint=i%4==0?vec3(1.0):palette(f);
        c+=tint*source*5.0;
    }
    vec3 key=normalize(vec3(-0.5,0.7,0.8));
    c+=vec3(2.8,2.9,3.0)*pow(max(dot(d,key),0.0),55.0);
    return c;
}
vec3 mirrorBall(vec3 p,vec3 rd){
    vec3 local=normalize(toLocal(p-CENTER));
    float latitude=asin(clamp(local.y,-1.0,1.0));
    float row=(latitude+PI*0.5)/PI*30.0;
    float rowCenter=(floor(row)+0.5)/30.0*PI-PI*0.5;
    float columns=max(6.0,floor(60.0*cos(rowCenter)+0.5));
    float longitude=atan(local.z,local.x);
    float column=(longitude+PI)/(2.0*PI)*columns;
    float colCenter=(floor(column)+0.5)/columns*2.0*PI-PI;
    vec2 cell=fract(vec2(column,row))-0.5;
    vec3 facet=vec3(cos(rowCenter)*cos(colCenter),sin(rowCenter),cos(rowCenter)*sin(colCenter));
    float edge=max(abs(cell.x),abs(cell.y));
    float bevel=smoothstep(0.37,0.46,edge);
    vec3 normal=normalize(toWorld(normalize(mix(facet,local,bevel*0.85))));
    // Explicit filtering keeps the dark grout and bevels stable in small buffers.
    vec2 pixel=clamp(fwidth(vec2(column,row)),vec2(0.02),vec2(0.22));
    vec2 coverage=vec2(1.0)-smoothstep(vec2(0.46)-pixel*0.45,vec2(0.50)+pixel*0.45,abs(cell));
    float tile=coverage.x*coverage.y;
    vec3 reflected=club(reflect(rd,normal))*vec3(0.90,0.95,1.0);
    float glint=pow(max(dot(normal,normalize(normalize(vec3(-0.5,0.7,0.8))-rd)),0.0),120.0);
    reflected+=vec3(2.0,1.9,1.8)*glint;
    return mix(vec3(0.004,0.007,0.014),reflected,tile);
}
vec3 geometry(vec3 ro,vec3 rd,out float distance){
    distance=sphere(ro,rd,CENTER,RADIUS);
    vec3 color=vec3(0.0);
    if(distance<100.0)color=mirrorBall(ro+rd*distance,rd);
    float cap=sphere(ro,rd,CENTER+vec3(0,RADIUS+0.02,0),0.058);
    if(cap<distance){
        distance=cap;vec3 n=normalize(ro+rd*cap-CENTER-vec3(0,RADIUS+0.02,0));
        color=club(reflect(rd,n))*0.45;
    }
    // A fine chrome suspension cable connects the ball to the dark ceiling.
    float a=dot(rd.xz,rd.xz),b=dot(ro.xz,rd.xz),c=dot(ro.xz,ro.xz)-0.0001;
    float disc=b*b-a*c;
    if(disc>0.0 && a>0.00001){
        float t=(-b-sqrt(disc))/a;float y=ro.y+rd.y*t;
        if(t>0.001 && t<distance && y>CENTER.y+RADIUS && y<4.0){
            distance=t;color=vec3(0.09,0.12,0.16)+club(reflect(rd,normalize(vec3((ro+rd*t).x,0,(ro+rd*t).z))))*0.12;
        }
    }
    return color;
}
float shaft(vec3 ro,vec3 rd,vec3 source,vec3 direction,float limit,float width){
    vec3 q=ro-source;float a=dot(rd,direction),b=dot(rd,q),c=dot(direction,q);
    float den=max(1.0-a*a,0.0001);
    float ray=(a*c-b)/den,along=c+a*ray;
    if(ray<0.0 || ray>limit || along<0.0 || along>8.0)return 0.0;
    float radius=0.012+width*along;
    vec3 delta=q+rd*ray-direction*along;
    float haze=exp(-dot(delta,delta)/(radius*radius));
    float facing=0.18+0.82*pow(max(dot(direction,-rd),0.0),3.0);
    return haze*facing*exp(-along*0.12)*smoothstep(0.0,0.22,along)*(1.0-smoothstep(6.0,8.0,along));
}
vec3 beams(vec3 ro,vec3 rd,float limit){
    vec3 color=vec3(0.0);
    for(int i=0;i<24;i++){
        float f=float(i);
        float latitude=-0.85+1.7*hash(vec2(f,1.3));
        float longitude=0.18+2.78*hash(vec2(f,7.1));
        vec3 n=toWorld(vec3(cos(latitude)*cos(longitude),sin(latitude),cos(latitude)*sin(longitude)));
        vec3 source=CENTER+n*(RADIUS+0.018);
        vec3 lamp=vec3(-3.0+6.0*hash(vec2(f,2.9)),2.8,3.8);
        vec3 incoming=normalize(source-lamp),direction=reflect(incoming,n);
        float strength=max(dot(n,-incoming),0.0);
        color+=palette(f)*shaft(ro,rd,source,direction,limit,0.032)*strength*0.65;
    }
    color+=vec3(0.05,0.45,0.70)*shaft(ro,rd,vec3(-3.4,2.7,1.0),normalize(CENTER-vec3(-3.4,2.7,1.0)),limit,0.10)*0.30;
    color+=vec3(0.65,0.04,0.35)*shaft(ro,rd,vec3(3.2,2.2,-1.0),normalize(CENTER-vec3(3.2,2.2,-1.0)),limit,0.10)*0.30;
    return color;
}
vec3 floorLights(vec3 p){
    vec2 grid=p.xz/0.62,cell=floor(grid);vec3 color=vec3(0.0);
    // Stable projected mirror patches drift gently across the polished stage.
    for(int x=-1;x<=1;x++)for(int y=-1;y<=1;y++){
        vec2 id=cell+vec2(x,y);float seed=hash(id);
        vec2 center=id+vec2(0.15+0.70*seed,0.15+0.7*hash(id+3.1));
        center+=0.09*vec2(sin(u_time*0.05+seed*6.0),cos(u_time*0.04+seed*9.0));
        vec2 q=rot(seed*6.0+u_time*0.017)*(grid-center);
        vec2 size=vec2(0.035+seed*0.06,0.035+seed*0.10);
        vec2 outside=max(abs(q)-size,0.0);
        float patch=exp(-dot(outside,outside)*2100.0);
        float halo=exp(-dot(q,q)*22.0)*0.07;
        float slow=0.35+0.65*pow(0.5+0.5*sin(u_time*0.08+seed*8.0),2.0);
        color+=palette(seed*13.0)*(patch+halo)*slow*0.58;
    }
    return color*exp(-length(p.xz)*0.10);
}
// Optical bloom follows the broad studio lights rather than random flashing.
vec3 lensGlints(vec2 uv,vec3 ro,vec3 forward,vec3 right,vec3 up,float focal){
    vec3 color=vec3(0.0),view=normalize(ro-CENTER);
    for(int i=0;i<8;i++){
        float f=float(i),angle=f*2.39996;
        vec3 light=normalize(vec3(cos(angle),0.15+0.7*sin(f*1.31),sin(angle)));
        vec3 ideal=normalize(light+view);
        if(dot(ideal,view)<0.15)continue;
        vec3 q=toLocal(ideal);
        float latitude=asin(clamp(q.y,-1.0,1.0));
        float row=floor((latitude+PI*0.5)/PI*30.0);
        float theta=(row+0.5)/30.0*PI-PI*0.5;
        float columns=max(6.0,floor(60.0*cos(theta)+0.5));
        float column=floor((atan(q.z,q.x)+PI)/(2.0*PI)*columns);
        float phi=(column+0.5)/columns*2.0*PI-PI;
        vec3 facet=toWorld(vec3(cos(theta)*cos(phi),sin(theta),cos(theta)*sin(phi)));
        float energy=exp(-(1.0-dot(ideal,facet))*480.0);
        vec3 point=CENTER+ideal*RADIUS-ro;
        float depth=dot(point,forward);if(depth<0.01)continue;
        vec2 center=vec2(dot(point,right),dot(point,up))*focal/depth;
        vec2 d=rot(0.35+f*0.25)*(uv-center);
        float width=max(0.0009,1.0/u_resolution.y);
        float rays=exp(-pow(d.x/width,2.0))*exp(-abs(d.y)*38.0)
                  +exp(-pow(d.y/width,2.0))*exp(-abs(d.x)*38.0);
        float halo=exp(-dot(d,d)*2800.0);
        vec3 tint=i%4==0?vec3(1.0):mix(palette(f),vec3(1.0),0.5);
        color+=tint*(rays*0.20+halo*0.12)*energy;
    }
    return color;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.35);
    vec3 ro=vec3(0.15*sin(u_time*0.025),0.65,5.7);
    vec3 forward=normalize(CENTER-ro),right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*(1.55*framing)+right*uv.x+up*uv.y);
    float distance;vec3 object=geometry(ro,rd,distance);
    vec3 color=vec3(0.003,0.004,0.013);
    color+=vec3(0.022,0.004,0.032)*exp(-length(uv-vec2(0.5,0.04))*3.0);
    color+=vec3(0.004,0.022,0.037)*exp(-length(uv-vec2(-0.5,0.1))*3.0);
    if(rd.y<-0.0001){
        float floorDistance=(FLOOR_Y-ro.y)/rd.y;
        if(floorDistance>0.0 && floorDistance<distance && floorDistance<30.0){
            vec3 p=ro+rd*floorDistance;float reflectedDistance;
            vec3 reflection=geometry(p+vec3(0,0.005,0),reflect(rd,vec3(0,1,0)),reflectedDistance);
            vec3 floorColor=vec3(0.006,0.007,0.018)+reflection*0.44+floorLights(p);
            color=mix(color,floorColor,1.0-smoothstep(10.0,30.0,floorDistance));
            distance=floorDistance;object=color;
        }
    }
    if(distance<100.0)color=object;
    color+=beams(ro,rd,min(distance,18.0));
    color+=lensGlints(uv,ro,forward,right,up,1.55*framing);
    color*=mix(1.0,0.86,u_night);
    color=vec3(1.0)-exp(-color*1.15);
    fragColor=vec4(pow(max(color,vec3(0.0)),vec3(0.62)),1.0);
}
