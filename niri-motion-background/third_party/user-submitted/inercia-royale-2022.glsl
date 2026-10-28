// Live coded for Inercia Shader Royale 2022 - 3rd place
// User supplied. Author/license were not included in the supplied snippet.
#define SAMPLE_COUNT 10.
#define RAYMARCH_DEPTH 100
float time;
float box(vec3 p,vec3 s){p=abs(p)-s;return max(p.x,max(p.y,p.z));}
mat2 rot(float a){float ca=cos(a),sa=sin(a);return mat2(ca,sa,-sa,ca);}
vec3 rnd3(vec3 p){return fract(sin(p.xyz*724.524+p.yzx*534.824+p.zxy*381.254)*vec3(413.644,388.521,924.532));}
float rnd(float t){return fract(sin(t*542.521)*824.588);}
float curve(float t){return mix(rnd(floor(t)),rnd(floor(t)+1.),pow(smoothstep(0.,1.,fract(t)),10.));}
vec3 loc;
float lum;
float map(vec3 p){
 vec3 p2=p;
 for(float i=0.;i<3.;i++){
  p.xz*=rot(time*.2+i*1.3+curve(time/3.)*4.+sin(p.y*.1)*.4);
  p.yz*=rot(time*.3-i*.3-curve(time/6.+7.)*4.+sin(p.x*.2)*.4);
  p.xz=abs(p.xz)-vec2(1.3,1.5)+i*.3-.5-sin(time*.3+vec2(7,3));
 }
 float d=box(p,vec3(1,.7,.1));
 vec3 p3=p;p3.z=max(0.,p3.z-2.);lum=length(p3)-.1;d=min(d,lum);
 p.xy*=rot(.785);d=min(d,max(abs(box(p,vec3(.4,.4,3)))-.04,abs(p.z)-1.4));loc=p;
 p.xz=abs(p.xz)-.3;d=min(d,length(p.xz)-.07);d=min(d,length(p.yz)-.07);d=min(d,length(p2)-3.);
 return d*.7;
}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 uv=fragCoord/iResolution.xy-.5;uv/=vec2(iResolution.y/iResolution.x,1);
 time=mod(iTime,300.);
 uv.y-=curve(time*3.)*.1;uv*=1.+curve(time*3.+9.)*.3;
 vec3 mumu=vec3(1);
 float prog=time/6.-length(uv)*.05,style=mod(prog*.5,1.),scene=floor(prog);
 time=time*1.3+rnd(scene)*300.;
 vec3 col=vec3(0),coco=vec3(0,.4,.8);float t2=time*.2;
 const float count=SAMPLE_COUNT;
 for(float j=0.;j<count;j++){
  vec3 dof=rnd3(vec3(uv+7.+j,fract(time)+j*.2))*.5;dof.z=0.;
  vec3 s=vec3(sin(time/20.)*5.,0,-15);s+=dof;
  float fov=1.+sin(time*.25)*.5;
  vec3 r=normalize(vec3(uv,fov));r-=dof*.08;
  vec3 p=s;float alpha=1.;
  for(int i=0;i<RAYMARCH_DEPTH;i++){
   float d=map(p);
   if(lum<.1)col+=vec3(3,.3,.3)*.0005/(.0001+abs(lum));
   if(d<.001){
    vec3 loco=loc;vec2 off=vec2(.01,0);
    vec3 n=normalize(d-vec3(map(p-off.xyy),map(p-off.yxy),map(p-off.yyx)));
    float rough=.1+.4*rnd3(floor(loco*8.)).x;if(length(p)<3.1)rough=.2;
    vec3 n2=normalize(n+rough*rnd3(vec3(uv+j,fract(time)+j*.1)));
    r=reflect(r,n2);d=.1;alpha*=.3+.6*pow(1.-abs(dot(n,r)),3.);
   }
   if(d>100.)break;p+=r*d;
  }
  vec3 sky=5.*max(vec3(0),-.4+sin(r.y*3.+coco+time+sin(r.x*4.+time*.3)));
  sky=mix(sky,vec3(.9),step(style,.5));col+=alpha*sky;
 }
 col/=count;
 col=mix(col,vec3(dot(col,vec3(.333))),vec3(.6+sin(time+abs(uv.x)*4.)*.3));
 col=pow(col,mumu);col*=1.2-length(uv);col=smoothstep(0.,1.,col);col=pow(col,vec3(.4545));
 fragColor=vec4(col,1.);
}
