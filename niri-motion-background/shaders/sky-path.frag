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
/* Nusan, SkyPath, coded live on Twitch https://www.twitch.tv/nusan_fx
 * Original: http://lezanu.fr/LiveCode/SkyPath.glsl
 * Supplied by the user. Silent wallpaper adaptation: constant camera travel,
 * fixed focal length and softer camera banking; scene geometry preserved.
 */
#define MARCH_STEPS 100
#define time iTime
float PI=acos(-1.0);
float box(vec3 p,vec3 s){vec3 ap=abs(p)-s;return length(max(vec3(0),ap))+min(0.,max(ap.x,max(ap.y,ap.z)));}
mat2 rot(float a){float ca=cos(a),sa=sin(a);return mat2(ca,sa,-sa,ca);}
vec3 tunnel(vec3 p){vec3 off=vec3(0);off.x+=sin(p.z*.6)*.7+sin(p.z*.17)*1.5;off.y+=cos(p.z*.27)*.5+cos(p.z*.34)*.25;return off;}
float stair(vec3 p,float s1,float s2){p.z=(fract(p.z/s1-.5)-.5)*s1;p.yz*=rot(PI*.25);return box(p,vec3(s2,s1,s1));}
float map(vec3 p){
 float rep=10.;p.y-=max(0.,abs(p.x)-10.)*.3;p.x=(fract(p.x/rep-.5)-.5)*rep;p+=tunnel(p);
 vec3 rp=p;float boxrep=10.;rp.z=(fract(rp.z/boxrep-.5)-.5)*boxrep;
 vec3 rp2=p;rp2.x=abs(rp2.x)-.4;rp2.z=(fract(rp2.z-.5)-.5);
 float b=box(rp+vec3(0,-9,0),vec3(.6,10.5,.6));vec3 rp3=rp+vec3(0,1.5,0);rp3.xy*=rot(PI*.3);rp3.yz*=rot(PI*.3);
 float b2=max(box(rp3,vec3(.7)),p.y+1.5);b=min(b,b2);
 float st=stair(p,.1,.4),st2=stair(p+vec3(0,.7,0),.6,.4);b=max(b,-st2);
 float c=box(rp2+vec3(0,.3,0),vec3(.05,.3,.2));rp2.y=abs(rp2.y+.43)-.1;c=min(c,box(rp2,vec3(.03,.03,1.)));
 return min(c,min(b,st));
}
vec3 norm(vec3 p){vec2 off=vec2(.01,0);return normalize(map(p)-vec3(map(p-off.xyy),map(p-off.yxy),map(p-off.yyx))+vec3(0.,1e-8,0.));}
float rnd(vec2 uv){return fract(dot(sin(uv*723.588+uv.yx*413.877),vec2(9865.535)));}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 uv=fragCoord/iResolution.xy-.5;uv/=vec2(iResolution.y/iResolution.x,1.);
 float dither=.85;
 vec3 s=vec3(0,-1,0),t=vec3(0,-.3,3),tar=vec3(0,-.7,3);
 // A steady glide replaces the original accelerating/stopping curve2 camera.
 float motion=time*.9;s.z+=motion;t.z+=motion;tar.z+=motion+3.+.35*sin(time*.12);
 s-=tunnel(s);vec3 offt=tunnel(t);t-=offt;tar-=tunnel(tar);
 vec3 cz=normalize(t-s),cx=normalize(cross(cz,vec3(0,1,0)-offt*.06)),cy=normalize(cross(cz,cx));
 // Fixed focal length removes the repeated wide/telephoto zoom.
 vec3 r=normalize(cx*uv.x+cy*uv.y+cz*.82),col=vec3(0),l=normalize(vec3(-.7,-1.,-.5));
 vec3 back=mix(vec3(.1,.3,1.)*.1,vec3(1.,.3,.5)*.5,pow(r.y*.5+.5,2.));
 vec3 p=s;float dd=0.,at=0.,at2=0.;int i=0;
 for(i=0;i<MARCH_STEPS;i++){
  float d=map(p)*dither;
  if(d<.001){
   vec3 n=norm(p);float fog=clamp(1.-dd*.018,0.,1.),bot=pow(clamp(-(p.y-10.)*.1,0.,1.),2.)*fog;
   float aodist=.3,ao=mix(clamp(map(p+n*aodist)/aodist,0.,1.),1.,.3),f=pow(1.-dot(n,-r),2.);
   col+=vec3(.3,.4,.7)*max(0.,dot(n,l))*bot*ao;
   col+=4.*f*back*2.*(-n.y*.5+.5)*bot*ao;dd=mix(100.,d,bot);break;
  }
  if(dd>100.){dd=100.;break;}p+=d*r;dd+=d;at+=exp(-length(tar-p)*7.);at2+=exp(-d*.1)*.05;
 }
 if(i>=MARCH_STEPS)dd=100.;
 col+=vec3(1.,.3,.5)*at*3.3+vec3(1.,.3,.5)*at2*.2+back*.05*exp(dd*.05);
 // Soft highlight rolloff avoids clipped, glaring pink/blue on the desktop.
 fragColor=vec4(1.-exp(-max(col,vec3(0.))*.8),1.);
}

void main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}
