#version 330 core
// Adapted from user-supplied Shadertoy source; see THIRD_PARTY.md.
// Silent: no audio input, output, recording or audio texture sampling.
uniform float u_time;
uniform vec2 u_resolution;
#define iTime u_time
#define iGlobalTime u_time
#define iResolution vec3(u_resolution,1.)
#define iMouse vec4(0.)
#define iTimeDelta 0.0
#define iFrame 0
out vec4 fragColor;
// User-supplied neon Menger-like reflective structure; author/license not supplied.
mat2 rot(float r){float c=cos(r),s=sin(r);return mat2(c,s,-s,c);}
float cube(vec3 p,vec3 s){vec3 q=abs(p)-s;return length(max(q,0.))+min(max(q.x,max(q.y,q.z)),0.);}
#define r(a) mat2(cos(a + asin(vec4(0,1,-1,0))))
#define M4(p) p=abs(p)-.5;p*=r(.785);p=abs(p)-.2;
#define K(p) p=abs(mod(p,2.)-1.);if(p.x<p.y)p=p.yx;
#define Q2(p) p=abs(p-1.)-abs(p+1.)+p;
vec4 map(vec3 p){
 p.y+=iTime*.1;p.xz*=rot(3.5);K(p.xy);Q2(p.xy);M4(p.xy);
 float d=cube(p,vec3(1.));vec3 col=vec3(0.);float s=1.,k=1.5+.45*sin(.12*iTime);
 float energyWave=sin(p.x*10.+iTime*4.)*sin(p.y*10.-iTime*2.)*sin(p.z*10.+iTime*3.);
 energyWave=smoothstep(.3,.9,energyWave);vec3 energyColor=vec3(0.,.4,1.)*energyWave*2.5;
 for(int m=0;m<4;m++){
  M4(p.xy);K(p.xy);vec3 a=mod(p*s,2.)-1.;K(p.xy);s*=3.;vec3 rr=abs(1.1-3.*abs(a));
  float da=max(rr.x,rr.y),db=max(rr.y,rr.z),dc=max(rr.z,rr.x),factor=abs(float(m)-k)/4.;
  if(rr.x<rr.y)col.z+=1.2*factor;if(rr.y>rr.z){col.y+=factor;col.z+=.2*factor;}if(rr.x>rr.z)col.x+=1.2*factor;
  d=max(d,(min(da,min(db,dc))-1.)/s);
 }
 return vec4(col+energyColor,d);
}
vec4 dist(vec3 p){vec4 sd=map(p);return vec4(1.04*sd.xyz*exp(-2.5*sd.w),sd.w);}
vec3 gn(vec3 p){vec2 e=vec2(.001,0.);return normalize(vec3(dist(p+e.xyy).w-dist(p-e.xyy).w,dist(p+e.yxy).w-dist(p-e.yxy).w,dist(p+e.yyx).w-dist(p-e.yyx).w)+vec3(0.,1e-8,0.));}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 p=(fragCoord-.5*iResolution.xy)/iResolution.y;
 vec3 ro=vec3(0.,1.,1.5)+1.,ta=vec3(1.),cdir=normalize(ta-ro);
 vec3 side=normalize(cross(cdir,vec3(0.,1.,0.))),up=cross(side,cdir),rd=normalize(p.x*side+p.y*up+1.2*cdir),finalColor=vec3(0.);
 for(int bounce=0;bounce<3;bounce++){
  float t=bounce==0?0.:.05;vec3 ac=vec3(0.);bool hit=false;
  for(int i=0;i<90;i++){
   vec4 rsd=dist(ro+rd*t);float d=max(rsd.w,0.);t+=d*.6;ac+=rsd.xyz;
   if(d<.001){hit=true;break;}
  }
  finalColor+=.015*ac;
  if(hit&&bounce<2){ro+=rd*t;rd=reflect(rd,gn(ro));}else break;
 }
 fragColor=vec4(pow(max(finalColor,vec3(0.)),vec3(1.1)),1.);
}

void main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}
