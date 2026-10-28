#version 330 core
// Original lightweight 2D accretion-disk illustration; not physical ray tracing.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
void main(){
    vec2 p=(gl_FragCoord.xy-.5*u_resolution)/u_resolution.y;
    float r=length(p), a=atan(p.y,p.x),t=u_time*.1;
    float disk=length(vec2(p.x,p.y*3.8));
    float flow=.7+.3*sin(a*16.-t+sin(a*9.+t)*1.3);
    float ring=exp(-abs(disk-.27)*35.)*flow;
    float lens=exp(-abs(r-.17)*110.);
    float horizon=smoothstep(.155,.171,r);
    float flare=exp(-abs(p.y)*70.)*exp(-abs(p.x)*3.5)*horizon;
    vec3 col=vec3(.002,.003,.008);
    col+=vec3(1.,.36,.07)*(ring*.65+lens*.65+flare*.4)*horizon;
    vec2 grid=p*110.;vec2 cell=floor(grid);
    float star=step(.993,hash(cell))*exp(-length(fract(grid)-.5)*18.);
    col+=vec3(.45,.55,.75)*star*smoothstep(.22,.4,r);
    col*=mix(1.,.65,u_night);
    fragColor=vec4(col,1.);
}
