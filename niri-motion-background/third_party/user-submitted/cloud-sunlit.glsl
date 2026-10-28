// User-supplied volumetric cloud. Fixes: initialized cross-product and jitter seed.
#define J normalize
#define F float
#define V vec3
#define R return
#define L length
F N(F a,F b,F k){F h=max(k-abs(a-b),0.)/k;R min(a,b)-h*h*k*.25;}
F m(V q,int n){
 mat3 m=mat3(2,-2,1,1,2,2,-2,-1,2)/3.;
 V Q=q*V(.8),p=m*Q,u=Q+V(1.8),w,c;
 F t=.2*iTime,s=1.5,d=L(p)-1.,k;
 for(int i=0;i<n;i++){
  w=u+t;w=sin(w+1.9*sin(.8*w.yzx+V(1,2.3,.4)));
  k=1.4/(.8+.1*sin(dot(u,V(.7,-.5,.6))-t));
  p=abs(p+.12*w.yzx)-V(.5);w*=.1;
  c=cross(w,p); // The supplied version used an uninitialized c here.
  p=m*(p+2./(1.+dot(w,w))*(c+cross(w,c)))*k;
  s*=k;d=N(d,(L(p)-.9)/s,.3/s);
  u=mat3(0,.8,.6,-.8,.4,-.5,-.6,-.5,.6)*u*1.8+1.3;t*=1.5;
 }
 R max(-N(-d,q.y+.2,.2),L(Q)-3.);
}
F D(F d,F h){R 40.*clamp((.4*h-d)/(h+.04),0.,1.);}
V H(V p,F f){
 F a=0.,t=0.,h=.012,b=clamp(.5+2.5*m(p+V(0,.2,0),4),0.,1.);
 for(int j=0;j<7;j++)a+=D(m(p+V(.5)*(t+.5*h),max(8-max(j,1),3)),h)*h,t+=h,h+=h;
 R V(1.3)*(f*exp(-a)+.45*exp(-.3*a)+.15*exp(-.08*a))
  +V(.58,.66,.8)*(.3+.7*b)*(.4+.6*clamp(.5+.9*m(p+V(0,.7,0),3),0.,1.))+.1-.06*b;
}
void mainImage(out vec4 O,vec2 U){
 vec2 r=iResolution.xy,u=(U+U-r)/r.y;
 F a=.2*iTime,v=1.,q,e,h,t,n=fract(sin(dot(U,vec2(12.9898,78.233)))*43758.5453),f;
 V o=V(5.6*sin(a),-.8,5.6*cos(a)),w=J(V(0,.65,0)-o),x=J(V(-w.z,0,w.x)),
 d=J(u.x*x+u.y*cross(x,w)+2.*w),Q=V(.8,1,.8),A=o*Q,P=d*Q,C=V(0);
 q=dot(P,P);e=dot(A,P);h=e*e-q*(dot(A,A)-7.);
 t=max((-e-sqrt(abs(h)))/q,0.);e=h<0.?-1.:(-e+sqrt(max(h,0.)))/q;
 f=.75+.26/pow(1.25-dot(d,V(.5)),1.5);
 for(int i=0;i<128&&t<e&&v>.01;i++){
  V p=o+d*t;F s=m(p,7),z=max(.6*s,.015*(1.+.01*F(i))*(.6+.8*fract(n+.6*F(i))));
  if(s<0.){h=1.-exp(-D(s,0.)*z);C+=v*h*H(p,f);v-=v*h;}t+=z;
 }
 O=atan(vec4(V(.2,.4,.7)*v+C,1));O.a=1.;
}
