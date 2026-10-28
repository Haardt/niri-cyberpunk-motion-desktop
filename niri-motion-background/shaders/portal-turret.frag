#version 330 core
// Original procedural fan-art of a Portal sentry turret; no game assets/textures.
// Deterministic bounded tracing, broad reflections and filtered laser, no sound.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
mat2 turn(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float ellipsoid(vec3 p,vec3 r){float k0=length(p/r),k1=length(p/(r*r));return k0*(k0-1.)/max(k1,1e-5);}
float capsule(vec3 p,vec3 a,vec3 b,float radius){vec3 pa=p-a,ba=b-a;return length(pa-ba*clamp(dot(pa,ba)/dot(ba,ba),0.,1.))-radius;}
float cylinderZ(vec3 p,float radius,float halfDepth){vec2 d=vec2(length(p.xy)-radius,abs(p.z)-halfDepth);return min(max(d.x,d.y),0.)+length(max(d,0.));}
vec2 add(vec2 a,vec2 b){return a.x<b.x?a:b;}
float yaw(){return .09*sin(u_time*.17);}
vec3 localPoint(vec3 p){p.xz=turn(yaw())*p.xz;return p;}
vec2 turret(vec3 world){
    vec3 p=localPoint(world);
    // The black chassis shows through a continuous central shell seam.
    vec2 d=vec2(ellipsoid(p-vec3(0,1.55,0),vec3(.36,.78,.34)),2.);
    float shell=ellipsoid(p-vec3(0,1.55,0),vec3(.49,.88,.39));
    shell=max(shell,.055-abs(p.x));
    d=add(d,vec2(shell,1.));
    // Side gun housings, stems and paired dark barrels.
    vec3 side=vec3(abs(p.x),p.y,p.z);
    d=add(d,vec2(capsule(side,vec3(.30,1.54,0),vec3(.62,1.54,0),.055),2.));
    d=add(d,vec2(ellipsoid(side-vec3(.61,1.55,0),vec3(.145,.36,.24)),1.));
    for(int i=0;i<2;i++){
        float y=1.47+.15*float(i);
        d=add(d,vec2(cylinderZ(side-vec3(.61,y,.28),.036,.11),2.));
    }
    // Three legs: two forward and one rear. Broad enough to stay visible.
    d=add(d,vec2(capsule(side,vec3(.18,.86,.015),vec3(.54,.28,.28),.045),3.));
    d=add(d,vec2(capsule(side,vec3(.54,.28,.28),vec3(.68,.065,.57),.041),1.));
    d=add(d,vec2(length(side-vec3(.54,.28,.28))-.061,2.));
    d=add(d,vec2(capsule(p,vec3(0,.83,-.15),vec3(0,.24,-.58),.048),3.));
    d=add(d,vec2(capsule(p,vec3(0,.24,-.58),vec3(0,.06,-.83),.041),1.));
    d=add(d,vec2(ellipsoid(side-vec3(.68,.055,.57),vec3(.07,.045,.13)),2.));
    d=add(d,vec2(ellipsoid(p-vec3(0,.05,-.84),vec3(.07,.04,.12)),2.));
    // Recessed red eye, dark lens bezel and glass over the emitter.
    d=add(d,vec2(cylinderZ(p-vec3(0,1.70,.38),.117,.052),2.));
    d=add(d,vec2(ellipsoid(p-vec3(0,1.70,.447),vec3(.070,.095,.025)),4.));
    return d;
}
vec3 normalAt(vec3 p){
    const vec2 e=vec2(.004,0.);
    return normalize(vec3(turret(p+e.xyy).x-turret(p-e.xyy).x,turret(p+e.yxy).x-turret(p-e.yxy).x,turret(p+e.yyx).x-turret(p-e.yyx).x));
}
vec2 bounds(vec3 ro,vec3 rd){
    vec3 safe=sign(rd)*max(abs(rd),vec3(1e-5));
    safe+=vec3(equal(safe,vec3(0.)))*1e-5;
    vec3 a=(vec3(-1.02,.0,-1.04)-ro)/safe,b=(vec3(1.02,2.50,.80)-ro)/safe;
    vec3 nearP=min(a,b),farP=max(a,b);
    return vec2(max(nearP.x,max(nearP.y,nearP.z)),min(farP.x,min(farP.y,farP.z)));
}
vec2 trace(vec3 ro,vec3 rd,float limit,int steps){
    vec2 range=bounds(ro,rd);float distance=max(range.x,0.);
    float end=min(range.y,limit);
    if(end<distance)return vec2(-1.);
    for(int i=0;i<88;i++){
        if(i>=steps||distance>end)break;
        vec2 d=turret(ro+rd*distance);
        float epsilon=max(.0012,distance*.00018);
        if(d.x<epsilon)return vec2(distance,d.y);
        distance+=max(d.x*.72,epsilon*.5);
    }
    return vec2(-1.);
}
vec3 studio(vec3 ray){
    vec3 col=mix(vec3(.30,.32,.36),vec3(.12,.15,.20),smoothstep(-.15,.9,ray.y));
    col+=vec3(.18,.10,.13)*exp(-pow(ray.y+.05,2.)*8.);
    // Large continuous light panels, avoiding pinpoint specular fireflies.
    col+=vec3(.85,.84,.80)*exp(-pow(dot(ray,normalize(vec3(-.7,.9,.8)))-1.,2.)*28.);
    col+=vec3(.32,.38,.48)*exp(-pow(dot(ray,normalize(vec3(.9,.4,-.4)))-1.,2.)*48.);
    return col;
}
vec3 material(vec3 p,vec3 ray,float id){
    vec3 n=normalAt(p),v=-ray;
    vec3 key=normalize(vec3(-.7,.9,.8)),rim=normalize(vec3(.8,.45,-.4));
    float diffuse=max(dot(n,key),0.),side=max(dot(n,rim),0.),facing=clamp(dot(n,v),0.,1.);
    vec3 base=id<1.5?vec3(.92,.94,.96):(id<2.5?vec3(.022,.026,.034):vec3(.22,.25,.29));
    float fresnel=.07+.65*pow(1.-facing,4.);
    vec3 col=base*(.28+1.0*diffuse+.22*side);
    col+=studio(reflect(ray,n))*fresnel*.48;
    vec3 halfDir=normalize(key+v);
    float specular=pow(max(dot(n,halfDir),0.),id<1.5?48.:75.);
    col+=vec3(.75,.73,.69)*specular*.35;
    // A soft contact/AO sample, no random rays or expensive shadow loop.
    float ao=clamp(turret(p+n*.08).x/.08,.25,1.);
    col*=.65+.35*ao;
    if(id>3.5){
        vec3 q=localPoint(p)-vec3(0,1.70,.447);
        float core=exp(-dot(q.xy,q.xy)*450.);
        col=vec3(.65,.009,.018)+vec3(1.2,.045,.025)*core;
    }
    return col;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-.5*u_resolution)/u_resolution.y;
    float angle=.34+.16*sin(u_time*.08);
    vec3 ro=vec3(6.4*sin(angle),2.0,6.4*cos(angle)),target=vec3(0,1.18,0);
    vec3 forward=normalize(target-ro),right=normalize(cross(forward,vec3(0,1,0))),up=cross(right,forward);
    vec3 rd=normalize(forward*1.80+right*uv.x+up*uv.y);
    float floorT=rd.y<-.0001?-ro.y/rd.y:50.;
    vec2 hit=trace(ro,rd,min(floorT,30.),88);
    vec3 color=studio(rd)*.55;
    if(hit.x>0.)color=material(ro+rd*hit.x,rd,hit.y);
    else if(floorT>0.&&floorT<30.){
        vec3 p=ro+rd*floorT;
        float contact=exp(-dot(p.xz*vec2(1.1,1.),p.xz*vec2(1.1,1.))*.95);
        color=vec3(.19,.20,.23)*(1.-contact*.45);
        vec3 reflected=reflect(rd,vec3(0,1,0));
        vec2 mirror=trace(p+vec3(0,.012,0),reflected,15.,56);
        float fresnel=.10+.22*pow(1.-max(-rd.y,0.),3.);
        vec3 reflectedColor=studio(reflected)*.6;
        if(mirror.x>0.)reflectedColor=material(p+vec3(0,.012,0)+reflected*mirror.x,reflected,mirror.y);
        color=mix(color,reflectedColor,fresnel*exp(-length(p.xz)*.13));
        // Wide red reflected glow underneath the eye, never a sharp spark.
        color+=vec3(.08,.003,.006)*exp(-dot(p.xz-vec2(0,.4),p.xz-vec2(0,.4))*2.);
    }
    // Subtle, anti-aliased aiming beam. Its width includes the pixel footprint.
    vec3 eye=vec3(0,1.70,.48),beam=normalize(vec3(0,-.09,1.));
    eye.xz=turn(-yaw())*eye.xz;beam.xz=turn(-yaw())*beam.xz;
    vec3 delta=ro-eye;float b=dot(rd,beam),d=dot(rd,delta),e=dot(beam,delta),den=max(1.-b*b,.0001);
    float alongRay=(b*e-d)/den,alongBeam=(e-b*d)/den;
    if(alongRay>0.&&alongBeam>0.&&alongBeam<3.2&&(hit.x<0.||alongRay<hit.x)){
        float dist=length(delta+rd*alongRay-beam*alongBeam);
        float width=.005+alongRay/u_resolution.y;
        color+=vec3(.13,.001,.003)*exp(-dist*dist/(width*width))*exp(-alongBeam*.65);
    }
    color*=mix(1.,.85,u_night);
    color=1.35*color/(1.+color);
    color=pow(max(color,vec3(0.)),vec3(.82));
    color*=1.-.16*smoothstep(.3,1.,length(uv));
    fragColor=vec4(color,1.);
}
