#version 330 core
// Original 3D heightfield ocean: four choppy swells, analytic normals, wind ripples.
// Six bounded secant steps instead of an open-ended ray march. No textures/buffers.
// Solar cycle: 240 shader seconds, scaled by the playlist entry's speed.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float TAU = 6.28318530718;
const float CAMERA_HEIGHT = 2.4;
const float WAVE_BOUND = 1.3;
float hash(vec2 p) {
    vec3 q=fract(vec3(p.xyx)*.1031);
    q+=dot(q,q.yzx+33.33);
    return fract((q.x+q.y)*q.z);
}
// Return height and its analytic x/z derivatives. Distance filtering prevents
// tiny distant waves from flickering at reduced buffer resolution.
vec3 swell(vec2 p, vec2 direction, float frequency, float amplitude, float speed, float phase, float footprint) {
    float a=dot(p,direction)*frequency-u_time*speed+phase;
    float s=sin(a), c=cos(a);
    float w=.5+.5*s;
    float lod=1./(1.+frequency*frequency*footprint*footprint*3.);
    float h=(w*w*w-.3125)*amplitude*lod;
    float derivative=1.5*w*w*c*amplitude*frequency*lod;
    return vec3(h,direction*derivative);
}
vec3 seaSurface(vec2 p, float footprint) {
    vec3 waves=swell(p,vec2(.8,.6),.65,.65,.48,.3,footprint);
    waves+=swell(p,vec2(-.48,.877),1.15,.33,.71,2.1,footprint);
    waves+=swell(p,vec2(.24,.971),2.20,.16,.93,4.3,footprint);
    waves+=swell(p,vec2(-.92,.392),3.40,.08,1.18,1.6,footprint);
    return waves;
}
vec2 windSlope(vec2 p,float footprint) {
    vec3 wind=swell(p,vec2(.93,.368),6.3,.035,1.42,.5,footprint);
    wind+=swell(p,vec2(-.36,.933),11.7,.018,1.85,3.8,footprint);
    wind+=swell(p,vec2(.66,.751),20.3,.008,2.2,2.6,footprint);
    return wind.yz;
}
vec3 skyColor(vec3 ray,vec3 sunDir,float daylight,float twilight,float visibility,vec3 sunlight,bool disc) {
    float height=clamp(ray.y*1.5,0.,1.);
    vec3 day=mix(vec3(.48,.64,.72),vec3(.07,.23,.43),pow(height,.6));
    vec3 night=mix(vec3(.025,.037,.075),vec3(.004,.009,.025),height);
    vec3 sky=mix(night,day,daylight);
    float horizon=exp(-max(ray.y,0.)*12.);
    sky=mix(sky,vec3(.82,.32,.14),twilight*horizon*.8);
    float separation=max(0.,1.-dot(ray,sunDir));
    sky+=sunlight*exp(-separation*35.)*.28*visibility;
    if(disc) {
        float edge=max(1.5/u_resolution.y,.0004);
        float distance=length(ray-sunDir);
        float sun=1.-smoothstep(.024-edge,.024+edge,distance);
        sky=mix(sky,sunlight*1.25,sun*visibility);
    }
    // Stable stars only above the horizon, without animated random sampling.
    vec2 starUV=ray.xz/max(ray.y+.5,.15)*150.;
    float stars=step(.997,hash(floor(starUV)))*exp(-length(fract(starUV)-.5)*16.);
    sky+=vec3(.4,.5,.65)*stars*(1.-daylight)*smoothstep(.04,.2,ray.y);
    return sky;
}
void main() {
    vec2 uv=gl_FragCoord.xy/u_resolution;
    float aspect=u_resolution.x/u_resolution.y;
    const float horizon=.48;
    vec3 ray=normalize(vec3((uv.x-.5)*aspect,(uv.y-horizon)*.9,1.3));
    float phase=u_time*TAU/240.+.10;
    float altitude=sin(phase);
    vec2 sunScreen=vec2(.5+.26*cos(phase),horizon+.32*altitude);
    vec3 sunDir=normalize(vec3((sunScreen.x-.5)*aspect,(sunScreen.y-horizon)*.9,1.3));
    float daylight=smoothstep(-.16,.23,altitude);
    float twilight=exp(-abs(altitude)*7.);
    float visibility=smoothstep(-.045,.035,altitude);
    vec3 sunlight=mix(vec3(1.,.36,.10),vec3(1.,.86,.62),smoothstep(0.,.65,altitude));
    vec3 color=skyColor(ray,sunDir,daylight,twilight,visibility,sunlight,true);

    if(ray.y<-.0005) {
        // Height is bounded inside +/- WAVE_BOUND, so the initial interval
        // brackets the surface. Fixed work, independent of wave/view angle.
        vec3 origin=vec3(0.,CAMERA_HEIGHT,0.);
        float nearT=(CAMERA_HEIGHT-WAVE_BOUND)/(-ray.y);
        float farT=(CAMERA_HEIGHT+WAVE_BOUND)/(-ray.y);
        float nearH=WAVE_BOUND-seaSurface((origin+ray*nearT).xz,nearT/u_resolution.y).x;
        float farH=-WAVE_BOUND-seaSurface((origin+ray*farT).xz,farT/u_resolution.y).x;
        float distance=-CAMERA_HEIGHT/ray.y;
        // Flatten the very distant horizon rather than evaluating unresolved geometry.
        if(distance<180.) {
            for(int i=0;i<6;i++) {
                float fraction=clamp(nearH/max(nearH-farH,.00001),.02,.98);
                distance=mix(nearT,farT,fraction);
                vec3 point=origin+ray*distance;
                float height=point.y-seaSurface(point.xz,distance/u_resolution.y).x;
                if(height>0.) {nearT=distance;nearH=height;}
                else {farT=distance;farH=height;}
            }
        }
        vec3 point=origin+ray*distance;
        float footprint=distance/u_resolution.y;
        vec3 waves=seaSurface(point.xz,footprint);
        vec2 slope=waves.yz+windSlope(point.xz,footprint);
        vec3 normal=normalize(vec3(-slope.x,1.,-slope.y));
        vec3 reflected=reflect(ray,normal);
        float fresnel=.025+.975*pow(1.-clamp(dot(normal,-ray),0.,1.),5.);
        vec3 reflection=skyColor(reflected,sunDir,daylight,twilight,visibility,sunlight,false);
        vec3 water=mix(vec3(.003,.014,.025),vec3(.008,.095,.13),daylight);
        float diffuse=max(dot(normal,sunDir),0.);
        vec3 sea=water*(.4+.6*diffuse);
        // Crest translucency and reflected sky make the swells read as volumes.
        float crest=clamp(waves.x+.28,0.,1.);
        sea+=vec3(.008,.095,.105)*crest*daylight*(.3+.7*diffuse);
        sea=mix(sea,reflection,fresnel);
        // Integrate distant glints into wider, softer highlights at low resolution.
        float shininess=mix(180.,35.,smoothstep(8.,90.,distance));
        float specular=pow(max(dot(reflect(-sunDir,normal),-ray),0.),shininess)*shininess/180.;
        sea+=sunlight*specular*visibility*2.2;
        // Sparse whitecaps, tied to real crest height rather than screen-space stripes.
        float foam=smoothstep(.33,.64,waves.x)*smoothstep(.74,.98,normal.y);
        foam*=.55+.45*sin(point.x*14.+point.z*17.-u_time);
        foam*=1.-smoothstep(6.,40.,distance);
        sea=mix(sea,mix(vec3(.025,.045,.065),vec3(.55,.65,.65),daylight),foam*.55);
        float haze=1.-exp(-distance*.006);
        vec3 horizonColor=skyColor(normalize(vec3(ray.x,.005,ray.z)),sunDir,daylight,twilight,visibility,sunlight,false);
        sea=mix(sea,horizonColor,haze*.75);
        float blend=1.-smoothstep(-.003,-.0005,ray.y);
        color=mix(color,sea,blend);
    }
    color*=mix(1.,.78,u_night);
    fragColor=vec4(clamp(color,0.,1.),1.);
}
