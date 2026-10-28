// User-supplied blue noise field with floating rectangles; author/license not supplied.
vec3 bgColor=vec3(.01,.16,.42),rectColor=vec3(.01,.26,.57);
const float noiseIntensity=2.8,noiseDefinition=.6;
const vec2 glowPos=vec2(-2.,0.);
const float total=60.,minSize=.03,maxSize=.08-minSize,yDistribution=.5;
float random(vec2 co){return fract(sin(dot(co,vec2(12.9898,78.233)))*43758.5453);}
float noise(vec2 p){p*=noiseIntensity;vec2 i=floor(p),f=fract(p),u=f*f*(3.-2.*f);return mix(mix(random(i),random(i+vec2(1,0)),u.x),mix(random(i+vec2(0,1)),random(i+vec2(1,1)),u.x),u.y);}
float fbm(vec2 uv){uv*=5.;mat2 m=mat2(1.6,1.2,-1.2,1.6);float f=.5*noise(uv);uv=m*uv;f+=.25*noise(uv);uv=m*uv;f+=.125*noise(uv);uv=m*uv;f+=.0625*noise(uv);return .5+.5*f;}
vec3 bg(vec2 uv){
 float velocity=iTime/1.6,intensity=sin(uv.x*3.+velocity*2.)*1.1+1.5;uv.y-=2.;vec2 bp=uv+glowPos;uv*=noiseDefinition;
 float rb=fbm(vec2(uv.x*.5-velocity*.03,uv.y))*.1;uv+=rb;
 float rz=fbm(uv*.9+vec2(velocity*.35,0.));rz*=dot(bp*intensity,bp)+1.2;
 float denom=.1-rz;denom=sign(denom)*max(abs(denom),1e-5);if(denom==0.)denom=1e-5;
 return sqrt(abs(bgColor/denom));
}
float rectangle(vec2 uv,vec2 pos,float width,float height,float blur){pos=(vec2(width,height)+.01)/2.-abs(uv-pos);pos=smoothstep(0.,max(blur,1e-5),pos);return pos.x*pos.y;}
mat2 rotate2d(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
void mainImage(out vec4 fragColor,in vec2 fragCoord){
 vec2 uv=fragCoord/iResolution.xy*2.-1.;uv.x*=iResolution.x/iResolution.y;
 vec3 color=bg(uv)*(2.-abs(uv.y*2.));float velX=-iTime/8.,velY=iTime/10.;
 for(float i=0.;i<total;i++){
  float index=i/total,rnd=random(vec2(index));vec3 pos=vec3(fract(velX*rnd+index)*4.-2.,sin(index*rnd*1000.+velY)*yDistribution,maxSize*rnd+minSize);
  vec2 uvRot=uv-pos.xy+pos.z/2.;uvRot=rotate2d(i+iTime/2.)*uvRot;uvRot+=pos.xy+pos.z/2.;
  float rect=rectangle(uvRot,pos.xy,pos.z,pos.z,(maxSize+minSize-pos.z)/2.);color+=rectColor*rect*pos.z/maxSize;
 }
 fragColor=vec4(max(color,vec3(0.)),1.);
}
