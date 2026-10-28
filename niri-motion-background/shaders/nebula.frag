#version 330 core
// Original 2D nebula: four noise octaves, no volumetric ray marching.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){vec3 q=fract(vec3(p.xyx)*.1031);q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<4;i++){v+=a*noise(p);p=mat2(1.6,1.2,-1.2,1.6)*p;a*=.5;}return v;}
void main(){
    vec2 p=(gl_FragCoord.xy-.5*u_resolution)/u_resolution.y;float t=u_time*.018;
    vec2 q=vec2(fbm(p*2.+t),fbm(p*2.+vec2(4.7,8.3)-t));
    float n=fbm(p*3.+q*2.5);float cloud=smoothstep(.23,.78,n);
    vec3 col=mix(vec3(.003,.006,.018),vec3(.07,.11,.32),cloud);
    col+=vec3(.35,.06,.25)*pow(cloud,2.5)*(0.5+q.x);
    vec2 cell=floor(p*130.);float star=step(.997,hash(cell))*exp(-length(fract(p*130.)-.5)*15.);
    col+=vec3(.7,.8,1.)*star;col*=mix(1.,.55,u_night);
    fragColor=vec4(col,1.);
}
