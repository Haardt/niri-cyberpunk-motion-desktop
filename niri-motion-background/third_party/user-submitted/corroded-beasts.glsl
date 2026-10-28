#version 410 core

uniform float fGlobalTime; // in seconds
uniform vec2 v2Resolution; // viewport resolution (in pixels)

uniform sampler1D texFFT; // towards 0.0 is bass / lower freq, towards 1.0 is higher / treble freq
uniform sampler1D texFFTSmoothed; // this one has longer falloff and less harsh transients
uniform sampler1D texFFTIntegrated; // this is continually increasing
uniform sampler2D texChecker;
uniform sampler2D texNoise;
uniform sampler2D texTex1;
uniform sampler2D texTex2;
uniform sampler2D texTex3;
uniform sampler2D texTex4;

layout(location = 0) out vec4 out_color; // out_color must be written in order to see anything

#define time fGlobalTime
float PI = acos(-1.0);

float cyl(vec2 p, float r) {
  return length(p)-r;
}

vec3 moda(vec3 p, float rep, float off) {
  vec2 rp = vec2(atan(p.z,p.x)/(2*PI), length(p.xz));
  rp.x=(fract(rp.x*rep-0.5+off)-0.5)/rep;
  rp.x *= 2*PI;
  return vec3(cos(rp.x)*rp.y,p.y,sin(rp.x)*rp.y);
}

mat2 rot(float a) {
  float ca=cos(a);
  float sa=sin(a);
  return mat2(ca,sa,-sa,ca);
}

float noise(vec3 p) {
  vec3 ip=floor(p);
  p=fract(p);
  p=smoothstep(0,1,p);
  vec3 st=vec3(7,37,289);
  vec4 pos=dot(ip,st) + vec4(0,st.y,st.z,st.y+st.z);
  vec4 val=mix(fract(sin(pos)*7894.552), fract(sin(pos+st.x)*7894.552), p.x);
  vec2 val2=mix(val.xz,val.yw, p.y);
  return mix(val2.x,val2.y, p.z);
}

float fractal(vec3 p) {

  float d=0.5;
  float f=0.0;
  for(int i=0; i<5; ++i) {
    f+=noise(p/d)*d;
    d*=.5;
  }
  return f;
}

float smin(float a, float b, float h) {
  float k=clamp((a-b)/h*0.5+0.5,0,1);
  return mix(a,b,k)-k*(1-k)*h;
}

vec3 tunnel(vec3 p){
  vec3 off=vec3(0);
  off.x += sin(p.z*0.2)*1.5;
  off.y += sin(p.z*0.3)*1.3;
  return off;
}

float map(vec3 p) {

  
  float t1 = time;
  float o = noise(p*3 + t1*0.2);
  p += (noise(p*10.0 + t1*0.7)-0.5)*0.1;
  p += tunnel(p);
  
  float s=10;
  p.xy = (fract(p.xy/s-0.5)-0.5)*s;

  vec3 p1=p;
  p1.xy *= rot(p.z*0.2+sin(p.z*1.8)*0.2 + t1*0.1);
  
  p1=moda(p1.xzy, 5, 0.0);
  float d = cyl(p1.xz-vec2(1,0),0.0+o*0.3-0.1)+0.1;
  
  vec3 p2=p;
  p2.xy *= rot(p.z*1.2+sin(p.z*0.8)*0.3 + t1*0.13);
  
  p2=moda(p2.xzy, 9,sin(p2.z)*0.9);
  float d2 = cyl(p2.xz-vec2(1.3,0),0.05);
  
  d=smin(d, d2, 0.9);
  
  return d;
}

vec3 norm(vec3 p) {
  vec2 off=vec2(0.01,0.0);
  return normalize(map(p)-vec3(map(p-off.xyy), map(p-off.yxy), map(p-off.yyx))+0.000001);
}

float getao(vec3 p, vec3 n, float dist) {
  return clamp(map(p+n*dist)/dist,0,1);
}

float getsss(vec3 p, vec3 r, float dist) {
  return clamp(map(p+r*dist)*3.0,0,1);
}

float rnd(float t) {
  return fract(sin(t*789.451)*7541.223);
}

float rnd(vec2 t) {
  return fract(dot(sin(t*vec2(789.451)+t.yx*vec2(842.544)),vec2(7541.223)));
}

float dots(vec3 p, float j) {
  float v=0;
  
  p*=4+1*sin(j);
  p.x += rnd(floor(p.y));

  p*=PI;
  v += clamp(0.1-length(vec2(sin(p.x),cos(p.y))),0,1)*10.3;
  return v;
}

void main(void)
{
  vec2 uv = vec2(gl_FragCoord.x / v2Resolution.x, gl_FragCoord.y / v2Resolution.y);
  uv -= 0.5;
  uv /= vec2(v2Resolution.y / v2Resolution.x, 1);

  vec3 s=vec3(0.0,0.0,-3);
  vec3 t=vec3(0,0,0);
  float t2 = time*0.5;
  s.x += sin(t2*0.7)*0.5;
  s.y += sin(t2*0.9)*0.5;

  s.z += t2;
  t.z += t2;
  s -= tunnel(s);
  t -= tunnel(t);
  vec3 cz=normalize(t-s);
  vec3 cx=normalize(cross(cz,vec3(0,1,0)));
  vec3 cy=normalize(cross(cz,cx));
  vec3 r=normalize(cx*uv.x+cy*uv.y+cz*0.7);
  //vec3 r=normalize(vec3(-uv, 0.7));

  vec3 p=s;
  int i=0;
  float mask=1.0;
  float d = 10000;

  float rand=rnd(uv);
  float dither=0.5+0.1*rand;
  
  for(; i<100; ++i) {
    d=map(p);
    if((d)<0.001) {
      mask=0;
      break;
    }
    p+=r*d*dither;
  }

  vec3 col=vec3(0);
  vec3 n = norm(p);
  vec3 l =normalize(vec3(-0.2,0.2,0.5));
  float t1=sin(time*0.2);
  l.xz *= rot(t1);
  l.xy *= rot(t1*3.2);
  vec3 h = normalize(l-r);
  float f = pow(1.0-abs(dot(n,r)), 3);
  float fog = pow(1.0-i/100.0,2.0);

  float aodist=0.7;
  float ao = getao(p,n,aodist*0.2) * (getao(p,n,aodist*0.35)*0.5+0.5) * (getao(p,n,aodist*0.6)*0.25+0.75);
  float sss = (getsss(p,r,0.2)+getsss(p,r,0.5)*0.5)*0.9;

  vec3 back = mix(vec3(0.7,0.2,0.1), vec3(1,0.7,0.2), pow(max(0,dot(r,l)),5));

  float diff = fractal(p*15);
  //diff=abs(diff-0.3)*2;
  diff=pow(smoothstep(0.1,0.9,diff),5)*2.7+0.9;
  
  col += max(0,dot(n,l)*0.5+0.5) * fog * ao * (vec3(1,0.7,0.3) + pow(max(0,dot(n,h)),20));
  col += vec3(0.7,0.2,0.1)*0.5*pow(n.y*0.5+0.5,3);
  col += sss * fog * back * vec3(1,0.2,0.2) * 1.5 * diff;
  col += 3*f*(-n.y*0.5+0.5)*fog;
  
  float len = length(p-s);
  col += back * max(clamp(d,0,1), clamp(dot(p-s,p-s)/200.0,0,1));

  vec3 col2 = vec3(0);
  for(int j=1; j<20; ++j) {
    float dist = (j) * 0.2/r.z;
    if(dist>len) break;
    vec3 vp = vec3(s.x,s.y,0) + r*dist;
    vp.xy *=rot(sin(vp.z*10+time*0.2));
    
    col2 += dots(vp, j) * clamp(1-dist/20.0, 0,1);
  }
  col += col2 * back;

  col *= pow(clamp(1.2-length(uv),0,1)*1.3,1.7);
  //col = vec3(rnd(uv));
  
  out_color = vec4(col, 1);
}