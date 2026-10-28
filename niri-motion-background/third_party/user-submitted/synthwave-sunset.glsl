// "another synthwave sunset thing", stduhpf; supplied by the user.
// Silent variant: sound textures disabled; no audio sampling or motion-blur jitter.
#define speed 10.
#define wave_thing
#define disable_sound_texture_sampling
#define textureMirror(a,b) vec4(0)
float jTime;
float amp(vec2 p){return smoothstep(1.,8.,abs(p.x));}
float pow512(float a){a*=a;a*=a;a*=a;a*=a;a*=a;a*=a;a*=a;a*=a;return a*a;}
float pow1d5(float a){return a*sqrt(a);}
float hash21(vec2 co){return fract(sin(dot(co,vec2(1.9898,7.233)))*45758.5433);}
float hash(vec2 uv){float a=amp(uv),w=a>0.?(1.-.4*pow512(.51+.49*sin((.02*(uv.y+.5*uv.x)-jTime)*2.))):0.;return a>0.?a*pow1d5(hash21(uv))*w:0.;}
float edgeMin(float dx,vec2 da,vec2 db,vec2 uv){return min(min((1.-dx)*db.y,da.x),da.y);}
vec2 trinoise(vec2 uv){
 const float sq=sqrt(3./2.);uv.x*=sq;uv.y-=.5*uv.x;vec2 d=fract(uv);uv-=d;bool c=dot(d,vec2(1))>1.;vec2 dd=1.-d,da=c?dd:d,db=c?d:dd;
 float nn=hash(uv+float(c)),n2=hash(uv+vec2(1,0)),n3=hash(uv+vec2(0,1)),nmid=mix(n2,n3,d.y),ns=mix(nn,c?n2:n3,da.y),dx=da.x/max(db.y,1e-6);
 return vec2(mix(ns,nmid,dx),edgeMin(dx,da,db,uv+d));
}
vec2 map(vec3 p){vec2 n=trinoise(p.xz);return vec2(p.y-2.*n.x,n.y);}
vec3 grad(vec3 p){const vec2 e=vec2(.005,0.);float a=map(p).x;return vec3(map(p+e.xyy).x-a,map(p+e.yxy).x-a,map(p+e.yyx).x-a)/e.x;}
vec2 intersect(vec3 ro,vec3 rd){float d=0.,h=0.;for(int i=0;i<160;i++){vec3 p=ro+d*rd;vec2 s=map(p);h=s.x;d+=h*.5;if(abs(h)<.003*d)return vec2(d,s.y);if(d>150.||p.y>2.)break;}return vec2(-1);}
void addsun(vec3 rd,vec3 ld,inout vec3 col){float sun=1.-smoothstep(.2,.21,distance(rd,ld));if(sun>0.){float yd=rd.y-ld.y,a=sin(3.1*exp(-yd*14.));sun*=smoothstep(-.8,0.,a);col=mix(col,vec3(1.,.8,.4)*.75,sun);}}
float starnoise(vec3 rd){
 float c=0.;vec3 p=normalize(rd)*300.;for(float i=0.;i<4.;i++){vec3 q=fract(p)-.5,id=floor(p);float c2=1.-smoothstep(0.,.5,length(q));c2*=step(hash21(id.xz/(abs(id.y)<1e-5?1e-5:id.y)),.06-i*i*.005);c+=c2;p=p*.6+.5*p*mat3(.6,0,.8,0,1,0,-.8,0,.6);}
 c*=c;float g=dot(sin(rd*10.512),cos(rd.yzx*10.512));c*=smoothstep(-3.14,-.9,g)*.5+.5*smoothstep(-.3,1.,g);return c*c;
}
vec3 gsky(vec3 rd,vec3 ld,bool mask){float haze=exp2(-5.*(abs(rd.y)-.2*dot(rd,ld))),st=mask?starnoise(rd)*(1.-min(haze,1.)):0.;vec3 back=vec3(.4,.1,.7),col=clamp(mix(back,vec3(.7,.1,.4),haze)+st,0.,1.);if(mask)addsun(rd,ld,col);return col;}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 uv=(2.*fragCoord-iResolution.xy)/iResolution.y;jTime=mod(iTime,4000.);
 vec3 ro=vec3(0.,1.,-20000.+jTime*speed),rd=normalize(vec3(uv,4./3.));vec2 i=intersect(ro,rd);float d=i.x;
 vec3 ld=normalize(vec3(0,.125+.05*sin(.1*jTime),1)),fog=d>0.?exp2(-d*vec3(.14,.1,.28)):vec3(0.),sky=gsky(rd,ld,d<0.);
 vec3 p=ro+d*rd,n=normalize(grad(p));float diff=dot(n,ld)+.1*n.y;vec3 col=vec3(.1,.11,.18)*diff;
 vec3 rfd=reflect(rd,n),rfcol=gsky(rfd,ld,true);col=mix(col,rfcol,.05+.95*pow(max(1.+dot(rd,n),0.),5.));
 col=mix(col,vec3(.8,.1,.92),1.-smoothstep(0.,.05,i.y));col=mix(sky,col,fog);fragColor=vec4(clamp(col,0.,1.),1.);
}
