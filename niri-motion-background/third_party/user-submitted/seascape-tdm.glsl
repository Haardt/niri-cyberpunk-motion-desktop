/* "Seascape" by Alexander Alekseev aka TDM - 2014.
 * Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported.
 * Contact: tdmaav@gmail.com. User-supplied source, ported for private local use.
 */
const int NUM_STEPS=32;
const float PI=3.141592,EPSILON=1e-3;
#define EPSILON_NRM (0.1/iResolution.x)
const int ITER_GEOMETRY=3,ITER_FRAGMENT=5;
const float SEA_HEIGHT=.6,SEA_CHOPPY=4.,SEA_SPEED=.8,SEA_FREQ=.16;
const vec3 SEA_BASE=vec3(0.,.09,.18),SEA_WATER_COLOR=vec3(.8,.9,.6)*.6;
#define SEA_TIME (1.0+iTime*SEA_SPEED)
const mat2 octave_m=mat2(1.6,1.2,-1.2,1.6);
mat3 fromEuler(vec3 ang){
 vec2 a1=vec2(sin(ang.x),cos(ang.x)),a2=vec2(sin(ang.y),cos(ang.y)),a3=vec2(sin(ang.z),cos(ang.z));mat3 m;
 m[0]=vec3(a1.y*a3.y+a1.x*a2.x*a3.x,a1.y*a2.x*a3.x+a3.y*a1.x,-a2.y*a3.x);
 m[1]=vec3(-a2.y*a1.x,a1.y*a2.y,a2.x);
 m[2]=vec3(a3.y*a1.x*a2.x+a1.y*a3.x,a1.x*a3.x-a1.y*a3.y*a2.x,a2.y*a3.y);return m;
}
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453123);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p),u=f*f*(3.-2.*f);return -1.+2.*mix(mix(hash(i),hash(i+vec2(1,0)),u.x),mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),u.x),u.y);}
float diffuse(vec3 n,vec3 l,float p){return pow(max(dot(n,l)*.4+.6,0.),p);}
float specular(vec3 n,vec3 l,vec3 e,float s){float nrm=(s+8.)/(PI*8.);return pow(max(dot(reflect(e,n),l),0.),s)*nrm;}
vec3 getSkyColor(vec3 e){e.y=(max(e.y,0.)*.8+.2)*.8;return vec3(pow(1.-e.y,2.),1.-e.y,.6+(1.-e.y)*.4)*1.1;}
float sea_octave(vec2 uv,float choppy){uv+=noise(uv);vec2 wv=1.-abs(sin(uv)),swv=abs(cos(uv));wv=mix(wv,swv,wv);return pow(max(0.,1.-pow(wv.x*wv.y,.65)),choppy);}
float map(vec3 p){float freq=SEA_FREQ,amp=SEA_HEIGHT,choppy=SEA_CHOPPY;vec2 uv=p.xz;uv.x*=.75;float d,h=0.;
 for(int i=0;i<ITER_GEOMETRY;i++){d=sea_octave((uv+SEA_TIME)*freq,choppy)+sea_octave((uv-SEA_TIME)*freq,choppy);h+=d*amp;uv*=octave_m;freq*=1.9;amp*=.22;choppy=mix(choppy,1.,.2);}return p.y-h;}
float map_detailed(vec3 p){float freq=SEA_FREQ,amp=SEA_HEIGHT,choppy=SEA_CHOPPY;vec2 uv=p.xz;uv.x*=.75;float d,h=0.;
 for(int i=0;i<ITER_FRAGMENT;i++){d=sea_octave((uv+SEA_TIME)*freq,choppy)+sea_octave((uv-SEA_TIME)*freq,choppy);h+=d*amp;uv*=octave_m;freq*=1.9;amp*=.22;choppy=mix(choppy,1.,.2);}return p.y-h;}
vec3 getSeaColor(vec3 p,vec3 n,vec3 l,vec3 eye,vec3 dist){
 float fresnel=clamp(1.-dot(n,-eye),0.,1.);fresnel=min(fresnel*fresnel*fresnel,.5);
 vec3 reflected=getSkyColor(reflect(eye,n)),refracted=SEA_BASE+diffuse(n,l,80.)*SEA_WATER_COLOR*.12;
 vec3 color=mix(refracted,reflected,fresnel);float atten=max(1.-dot(dist,dist)*.001,0.);
 color+=SEA_WATER_COLOR*(p.y-SEA_HEIGHT)*.18*atten;color+=specular(n,l,eye,600.*inversesqrt(max(dot(dist,dist),1e-6)));return color;}
vec3 getNormal(vec3 p,float eps){eps=max(eps,1e-5);vec3 n;n.y=map_detailed(p);n.x=map_detailed(p+vec3(eps,0,0))-n.y;n.z=map_detailed(p+vec3(0,0,eps))-n.y;n.y=eps;return normalize(n);}
float heightMapTracing(vec3 ori,vec3 dir,out vec3 p){float tm=0.,tx=1000.,hx=map(ori+dir*tx);if(hx>0.){p=ori+dir*tx;return tx;}float hm=map(ori);
 for(int i=0;i<NUM_STEPS;i++){float tmid=mix(tm,tx,hm/(hm-hx));p=ori+dir*tmid;float hmid=map(p);if(hmid<0.){tx=tmid;hx=hmid;}else{tm=tmid;hm=hmid;}if(abs(hmid)<EPSILON)break;}return mix(tm,tx,hm/(hm-hx));}
vec3 getPixel(vec2 coord,float time){
 vec2 uv=coord/iResolution.xy*2.-1.;uv.x*=iResolution.x/iResolution.y;
 vec3 ang=vec3(sin(time*3.)*.1,sin(time)*.2+.3,time),ori=vec3(0.,3.5,time*5.),dir=normalize(vec3(uv,-2.));dir.z+=length(uv)*.14;dir=normalize(dir)*fromEuler(ang);
 vec3 p;heightMapTracing(ori,dir,p);vec3 dist=p-ori,n=getNormal(p,dot(dist,dist)*EPSILON_NRM),light=normalize(vec3(0.,1.,.8));
 return mix(getSkyColor(dir),getSeaColor(p,n,light,dir,dist),pow(1.-smoothstep(-.02,0.,dir.y),.2));
}
void mainImage(out vec4 fragColor,in vec2 fragCoord){vec3 color=getPixel(fragCoord,iTime*.3);fragColor=vec4(pow(max(color,vec3(0.)),vec3(.65)),1.);}
