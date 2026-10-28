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
// Created by evilryu. CC BY-NC-SA 3.0 Unported.
// Port changes: safe denominators, initialized miss result, opaque wallpaper alpha.
float stime,ctime,pixel_size=0.;
void ry(inout vec3 p,float a){float c=cos(a),s=sin(a);vec3 q=p;p.x=c*q.x+s*q.z;p.z=-s*q.x+c*q.z;}
vec3 mb(vec3 p){
 p.xyz=p.xzy;vec3 z=p;float power=8.,r=1.,theta,phi,dr=1.,t0=1.;
 for(int i=0;i<7;i++){
  r=max(length(z),1e-7);if(r>2.)break;
  theta=atan(z.y/(abs(z.x)<1e-7?1e-7:z.x));phi=asin(clamp(z.z/r,-1.,1.));
  dr=pow(r,power-1.)*dr*power+1.;r=pow(r,power);theta*=power;phi*=power;
  z=r*vec3(cos(theta)*cos(phi),sin(theta)*cos(phi),sin(phi))+p;t0=min(t0,r);
 }
 return vec3(.5*log(max(r,1e-7))*r/dr,t0,0.);
}
vec3 f(vec3 p){ry(p,iTime*.2);return mb(p);}
float softshadow(vec3 ro,vec3 rd,float k){float akuma=1.,h=0.,t=.01;for(int i=0;i<50;i++){h=f(ro+rd*t).x;if(h<.001)return .02;akuma=min(akuma,k*h/t);t+=clamp(h,.01,2.);}return akuma;}
vec3 nor(vec3 pos){vec3 e=vec3(.001,0,0);return normalize(vec3(f(pos+e.xyy).x-f(pos-e.xyy).x,f(pos+e.yxy).x-f(pos-e.yxy).x,f(pos+e.yyx).x-f(pos-e.yyx).x)+vec3(0.,1e-8,0.));}
vec3 intersect(vec3 ro,vec3 rd){
 float t=1.,res_t=0.,max_error=1000.,d=1.,pd=100.,os=0.,step=0.,error=1000.;vec3 c,res_c=vec3(0.);
 for(int i=0;i<48;i++){
  if(error<pixel_size*.5||t>20.)break;
  c=f(ro+rd*t);d=c.x;
  if(d>os){os=.4*d*d/pd;step=d+os;pd=d;}else{step=-os;os=0.;pd=100.;d=1.;}
  error=d/t;if(error<max_error){max_error=error;res_t=t;res_c=c;}t+=step;
 }
 if(t>20.)res_t=-1.;return vec3(res_t,res_c.y,res_c.z);
}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 q=fragCoord/iResolution.xy,uv=-1.+2.*q;uv.x*=iResolution.x/iResolution.y;
 pixel_size=1./(iResolution.x*3.);stime=.7+.3*sin(iTime*.4);ctime=.7+.3*cos(iTime*.4);
 vec3 ta=vec3(0.),ro=vec3(0.,3.*stime*ctime,3.*(1.-stime*ctime));
 vec3 cf=normalize(ta-ro),cs=normalize(cross(cf,vec3(0,1,0))),cu=normalize(cross(cs,cf)),rd=normalize(uv.x*cs+uv.y*cu+3.*cf);
 vec3 sundir=normalize(vec3(.1,.8,.6)),sun=vec3(1.64,1.27,.99),skycolor=vec3(.6,1.5,1.),bg=exp(uv.y-2.)*vec3(.4,1.6,1.);
 float halo=clamp(dot(normalize(-ro),rd),0.,1.);vec3 col=bg+vec3(1.,.8,.4)*pow(halo,17.),res=intersect(ro,rd);
 if(res.x>0.){
  vec3 p=ro+res.x*rd,n=nor(p);float shadow=softshadow(p,sundir,10.);
  float dif=max(0.,dot(n,sundir)),sky=.6+.4*max(0.,dot(n,vec3(0,1,0))),bac=max(.3+.7*dot(vec3(-sundir.x,-1.,-sundir.z),n),0.),spe=pow(clamp(dot(sundir,reflect(rd,n)),0.,1.),10.);
  vec3 lin=4.5*sun*dif*shadow+.8*bac*sun+.6*sky*skycolor*shadow+3.*spe*shadow;
  res.y=pow(clamp(res.y,0.,1.),.55);vec3 tc0=.5+.5*sin(3.+4.2*res.y+vec3(0.,.5,1.));col=lin*vec3(.9,.8,.6)*.2*tc0;col=mix(col,bg,1.-exp(-.001*res.x*res.x));
 }
 col=pow(clamp(col,0.,1.),vec3(.45));col=col*.6+.4*col*col*(3.-2.*col);col=mix(col,vec3(dot(col,vec3(.33))),-.5);col*=.5+.5*pow(16.*q.x*q.y*(1.-q.x)*(1.-q.y),.7);
 fragColor=vec4(col,1.);
}

void main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}
