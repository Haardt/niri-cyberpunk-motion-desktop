// User-supplied compact volumetric cloud; author/license not supplied.
#define V vec3
#define K mat3(2,-2,1,1,2,2,-2,-1,2)/3.
float M(V q,float n){
 q.xz*=.7;V p=q,w;
 float s=1.,d=length(p)-s,k,x,h;
 for(;n-->0.;){
  w=sin(q+1.3*sin(.4*q.yzx));k=2.-w.x/1.7;
  p=(abs(K*p+.2*w.yzx)-.6)*k;
  x=(length(p)-1.)/(s*=k);h=max(.6/s-abs(d-x),0.);
  d=min(d,x)-h*h*s/1.2;q=K*q*1.9+iTime*.15;
 }return d;
}
void mainImage(out vec4 O,vec2 U){
 float n=.2*iTime-.5,i=0.,T=1.,C=0.,t=0.,h,d,x,g,a,j;
 V R=iResolution,o=V(0,.6,6.),D=normalize(V(U+U-R.xy,-1.8*R.y)),p;
 mat2 m=mat2(cos(n+vec4(0,11,33,0)));o.xz*=m;D.xz*=m;
 n=.8+.2*pow(1.3-dot(D,V(.5)),-1.5);
 for(;i++<1e2&&t<1e1&&T>.01;t+=x){
  x=max(.6*(d=M(p=o+D*t,9.)),.02+i/7e3);
  if(d<0.){
   a=exp(x*max(8e2*d,-40.));
   for(g=j=0.,h=.02;j<6.;h+=h)g+=80.*h*clamp((.7*h-M(p+h*.5,7.-j++))/(h+.04),0.,1.);
   p.y+=.2;j=clamp(.7+1.8*M(p,4.),.3,1.);p.y+=.5;
   C+=(T-T*a)*(n*exp(-g)+.45*exp(-.3*g)+.15*exp(-.08*g)+j*clamp(.4+.3*M(p,3.),.2,.6));T*=a;
  }
 }
 O=vec4(C+T*V(.2,.4,.8),1.);
}
