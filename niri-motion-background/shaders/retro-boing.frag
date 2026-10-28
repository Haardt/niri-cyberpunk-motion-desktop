#version 330 core
// Original 1980s demo-scene homage, MIT. Procedural red/white checker ball,
// antialiased checker floor, copper gradients and restrained CRT phosphor.
// No logos, textures, audio or copied demo assets.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
const float FLOOR=-1.12;
const float RADIUS=0.86;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
vec3 center(){
    float bounce=sin(u_time*0.25);
    float rise=0.46*(sqrt(bounce*bounce+0.0025)-0.05);
    return vec3(0.30*sin(u_time*0.11),FLOOR+RADIUS+0.055+rise,0.08*cos(u_time*0.09));
}
float sphere(vec3 ro,vec3 rd,vec3 c){vec3 q=ro-c;float b=dot(q,rd),d=b*b-dot(q,q)+RADIUS*RADIUS;if(d<0.0)return 100.0;float t=-b-sqrt(d);return t>0.001?t:100.0;}
vec2 triangle(vec2 p){return vec2(1.0)-abs(mod(p,2.0)-1.0);}
float filteredChecker(vec2 p,vec2 width){
    // Integrating the checker wave across a pixel avoids a noisy distant floor.
    width=clamp(width,vec2(0.001),vec2(1.0));
    vec2 average=(triangle(p+0.5*width)-triangle(p-0.5*width))/width;
    return clamp(0.5-0.5*average.x*average.y,0.0,1.0);
}
float checker(vec2 p){return filteredChecker(p,fwidth(p));}
vec3 sky(vec3 d){
    vec3 c=mix(vec3(0.045,0.025,0.10),vec3(0.075,0.06,0.16),smoothstep(-0.15,0.6,d.y));
    c+=vec3(0.07,0.025,0.13)*exp(-abs(d.y+0.04)*5.0);
    return c;
}
vec3 ball(vec3 p,vec3 rd){
    vec3 n=normalize(p-center()),q=n;
    q.xy=rot(0.32)*q.xy;q.xz=rot(u_time*0.10)*q.xz;
    vec2 uv=vec2((atan(q.z,q.x)+PI)/PI*8.0,(asin(clamp(q.y,-1.0,1.0))+PI*0.5)/PI*8.0);
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.4);
    float pixel=5.0/(u_resolution.y*1.43*framing*RADIUS);
    float grazing=max(abs(dot(n,-rd)),0.14);
    vec2 footprint=vec2(8.0/(PI*max(length(q.xz),0.12)),8.0/PI)*pixel/grazing;
    float pattern=filteredChecker(uv,footprint);
    vec3 base=mix(vec3(0.73,0.012,0.033),vec3(0.91,0.91,0.86),pattern);
    vec3 light=normalize(vec3(-0.55,0.85,0.65)),fill=normalize(vec3(0.8,0.3,-0.2));
    float diffuse=max(dot(n,light),0.0),side=max(dot(n,fill),0.0);
    float specular=pow(max(dot(n,normalize(light-rd)),0.0),62.0);
    float fresnel=pow(1.0-max(dot(n,-rd),0.0),4.0);
    vec3 col=base*(0.20+diffuse*0.72+side*0.13);
    col+=vec3(0.70,0.66,0.62)*specular+sky(reflect(rd,n))*fresnel*0.25;
    col+=vec3(0.05,0.025,0.10)*fresnel;
    return col;
}
float box(vec2 p,vec2 b,float r){vec2 q=abs(p)-b+r;return length(max(q,0.0))+min(max(q.x,q.y),0.0)-r;}
vec3 copper(vec2 uv){
    vec3 color=vec3(0.0);
    for(int i=0;i<7;i++){
        float f=float(i);
        // Visible slow raster choreography: travel, spreading and phase-shifted waves.
        // At the shipped speed the complete stack travels up/down in about 19 seconds.
        float travel=0.285+0.075*sin(u_time*0.65);
        float spacing=0.025+0.008*sin(u_time*0.43);
        float y=travel+(f-3.0)*spacing+0.018*sin(u_time*0.85+f*0.80);
        float d=abs(uv.y-y),radius=0.014;
        float roundness=sqrt(max(1.0-pow(d/radius,2.0),0.0));
        float shape=1.0-smoothstep(radius-0.0015,radius+0.0015,d);
        vec3 tint=0.5+0.5*cos(vec3(0.0,2.0944,4.1888)+f*0.77+0.25);
        color+=tint*shape*(0.12+roundness*0.40);
        color+=tint*exp(-d*150.0)*0.025;
    }
    return color;
}
vec3 render(vec2 screen){
    float aspect=u_resolution.x/u_resolution.y;
    vec2 uv=screen*(1.0+0.035*dot(screen,screen));
    float framing=min(1.0,aspect/1.4);
    vec3 ro=vec3(0.0,0.90,5.0),target=vec3(0,-0.08,0);
    vec3 forward=normalize(target-ro),right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*(1.43*framing)+right*uv.x+up*uv.y);
    vec3 background=sky(rd)+copper(uv/framing);
    vec3 col=background;float ballDistance=sphere(ro,rd,center());
    if(rd.y<-0.0001){
        float floorDistance=(FLOOR-ro.y)/rd.y;
        if(floorDistance>0.0 && floorDistance<ballDistance && floorDistance<30.0){
            vec3 p=ro+rd*floorDistance;
            float pattern=checker(p.xz*1.32);
            vec3 ground=mix(vec3(0.055,0.038,0.105),vec3(0.30,0.27,0.39),pattern);
            vec3 light=normalize(vec3(-0.55,0.85,0.65)),toCenter=center()-p;
            float along=dot(toCenter,light),distance=length(toCenter-light*along);
            float height=center().y-FLOOR-RADIUS;
            float shadow=smoothstep(RADIUS*0.72,RADIUS*1.12+height*0.3,distance);
            if(along>0.0)ground*=0.40+0.60*shadow;
            float contact=exp(-dot(p.xz-center().xz,p.xz-center().xz)/(0.43+height*0.35));
            ground*=1.0-contact*0.28;
            vec3 reflected=reflect(rd,vec3(0,1,0)),reflectionOrigin=p+vec3(0,0.004,0);
            float reflection=sphere(reflectionOrigin,reflected,center());
            if(reflection<100.0)ground+=ball(reflectionOrigin+reflected*reflection,reflected)*0.13;
            float haze=smoothstep(8.0,26.0,floorDistance);
            col=mix(ground,background,haze);
        }
    }
    if(ballDistance<100.0)col=ball(ro+rd*ballDistance,rd);
    // Slight tube curvature and a physical dark bezel, with static gentle scanlines.
    float border=box(screen,vec2(max(aspect*0.5-0.018,0.12),0.480),0.025);
    float display=1.0-smoothstep(-0.001,0.002,border);
    float scan=0.985+0.015*sin(gl_FragCoord.y*PI);
    col*=scan*(1.0-0.15*smoothstep(0.2,0.85,length(screen)));
    col*=mix(1.0,0.82,u_night);
    col=pow(vec3(1.0)-exp(-col*1.22),vec3(0.67));
    col=mix(vec3(0.008,0.009,0.016),col,display);
    col+=vec3(0.025,0.022,0.04)*exp(-abs(border)*520.0);
    return col;
}
void main(){
    vec2 screen=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 color=vec3(0.0);
    // Four bounded samples smooth the analytic sphere silhouette as well as the grid.
    for(int y=0;y<2;y++)for(int x=0;x<2;x++){
        vec2 offset=(vec2(x,y)-0.5)*0.5/u_resolution.y;
        color+=render(screen+offset);
    }
    fragColor=vec4(color*0.25,1.0);
}
