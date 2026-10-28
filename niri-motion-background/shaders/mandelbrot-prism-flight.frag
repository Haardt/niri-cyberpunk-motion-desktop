#version 330 core
#extension GL_ARB_gpu_shader_fp64 : enable
// Original cinematic Mandelbrot flight, MIT. Genuine z <- z*z+c iteration,
// native fp64 when available, float-float fallback, distance gradients and neon chrome.
// Continuous logarithmic approach/orbit/retreat; no resets, assets or audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
vec2 cmul(vec2 a,vec2 b){return vec2(a.x*b.x-a.y*b.y,a.x*b.y+a.y*b.x);}
// Double-single arithmetic keeps the filaments stable beyond ordinary pixel precision.
vec2 dsAdd(vec2 a,vec2 b){
    float sum=a.x+b.x,v=sum-a.x;
    float error=(a.x-(sum-v))+(b.x-v)+a.y+b.y;
    float high=sum+error;return vec2(high,error-(high-sum));
}
vec2 split(float a){float c=4097.0*a,high=c-(c-a);return vec2(high,a-high);}
vec2 dsMul(vec2 a,vec2 b){
    float product=a.x*b.x;vec2 aa=split(a.x),bb=split(b.x);
    float error=((aa.x*bb.x-product)+aa.x*bb.y+aa.y*bb.x)+aa.y*bb.y+a.x*b.y+a.y*b.x;
    float high=product+error;return vec2(high,error-(high-product));
}
vec2 coordinate(vec2 base,float offset){
#ifdef GL_ARB_gpu_shader_fp64
    double value=double(base.x)+double(base.y)+double(offset);
    float high=float(value);
    return vec2(high,float(value-double(high)));
#else
    return dsAdd(base,vec2(offset,0.0));
#endif
}
struct Fractal {float inside;float mu;float logDistance;vec2 gradient;float trap;};
Fractal mandelbrot(vec2 cx,vec2 cy){
    vec2 c=vec2(cx.x+cx.y,cy.x+cy.y);float x=c.x-0.25,q=x*x+c.y*c.y;
    if(q*(q+x)<0.25*c.y*c.y-0.000001 || dot(c-vec2(-1,0),c-vec2(-1,0))<0.062499){
        return Fractal(1.0,420.0,-50.0,vec2(0),0.3);
    }
    vec2 zr=vec2(0),zi=vec2(0),derivative=vec2(0),z=vec2(0);
#ifdef GL_ARB_gpu_shader_fp64
    dvec2 exactC=dvec2(double(cx.x)+double(cx.y),double(cy.x)+double(cy.y));
    dvec2 exactZ=dvec2(0.0);
#endif
    float logScale=0.0,trap=10.0;int iteration=420;bool escaped=false;
    for(int i=0;i<420;i++){
        #ifdef GL_ARB_gpu_shader_fp64
        z=vec2(exactZ);
#else
        z=vec2(zr.x+zr.y,zi.x+zi.y);
#endif
        float r2=dot(z,z);
        if(r2>256.0){iteration=i;escaped=true;break;}
        trap=min(trap,abs(z.y)+0.15*abs(z.x));
        derivative=2.0*cmul(z,derivative)+vec2(exp(-logScale),0.0);
        if(dot(derivative,derivative)>1e22){derivative*=1e-10;logScale+=23.02585092994;}
        #ifdef GL_ARB_gpu_shader_fp64
        exactZ=dvec2(exactZ.x*exactZ.x-exactZ.y*exactZ.y+exactC.x,2.0*exactZ.x*exactZ.y+exactC.y);
#else
        vec2 rr=dsMul(zr,zr),ii=dsMul(zi,zi),ri=dsMul(zr,zi);
        zr=dsAdd(dsAdd(rr,-ii),cx);
        zi=dsAdd(ri*2.0,cy);
#endif
    }
    if(!escaped)return Fractal(1.0,420.0,-50.0,vec2(0),trap);
    float r2=max(dot(z,z),1.001),logR=0.5*log(r2);
    float mu=float(iteration)+1.0-log2(logR);
    float logDerivative=log(max(length(derivative),1e-25))+logScale;
    float logDistance=logR+log(max(logR,1e-6))-logDerivative-0.69314718056;
    vec2 quotient=cmul(derivative,vec2(z.x,-z.y))/r2;
    vec2 gradient=vec2(quotient.x,-quotient.y);
    gradient/=max(length(gradient),1e-25);
    return Fractal(0.0,mu,logDistance,gradient,trap);
}
vec3 studio(vec3 d){
    vec3 color=vec3(0.035,0.045,0.085);
    float key=pow(max(dot(d,normalize(vec3(-0.6,0.7,0.75))),0.0),16.0);
    float stripe=exp(-pow((d.x+0.34)*11.0,2.0))*smoothstep(-0.2,0.7,d.y);
    float rim=pow(max(dot(d,normalize(vec3(0.75,-0.15,0.4))),0.0),28.0);
    return color+vec3(2.3,2.3,2.25)*key+vec3(0.20,0.85,1.40)*stripe+vec3(1.2,0.15,0.55)*rim;
}
vec3 render(vec2 uv){
    float phase=u_time*0.022,dolly=0.5-0.5*cos(phase);
    float eased=dolly;
    float span=3.1*exp(-8.35*eased);
#ifndef GL_ARB_gpu_shader_fp64
    span=max(span,0.003);
#endif
    float orbit=u_time*0.038+0.30*sin(u_time*0.014);
    float bank=0.24*sin(phase)+0.38*sin(u_time*0.012);
    float tilt=0.20+0.12*sin(u_time*0.016);
    float perspective=1.0+uv.y*tilt;
    vec2 plane=rot(bank)*(uv/perspective);
    // Analytic pixel Jacobian also works for adaptive, nonuniform filter taps.
    vec2 dx=rot(bank)*vec2(1.0/perspective,0.0)/u_resolution.y;
    vec2 dy=rot(bank)*vec2(-uv.x*tilt,1.0)/(perspective*perspective*u_resolution.y);
    float approach=1.0-smoothstep(0.02,0.30,dolly);
    vec2 drift=span*(0.12+0.05*dolly)*vec2(cos(orbit),sin(orbit)*0.72);
    drift+=span*vec2(-0.10,0.12)*smoothstep(0.08,0.40,dolly);
    vec2 offset=vec2(0.243643887037151,-0.131825904205330)*approach+drift+plane*span;
    vec2 cx=coordinate(vec2(-0.7436438798904419,-7.146709113392546e-9),offset.x);
    vec2 cy=coordinate(vec2(0.1318259090185165,-4.813186543772829e-9),offset.y);
    Fractal f=mandelbrot(cx,cy);
    vec3 view=normalize(vec3(-uv*0.35,1.6));
    if(f.inside>0.5){
        vec2 c=vec2(cx.x+cx.y,cy.x+cy.y);
        vec3 n=normalize(vec3(0.19*sin(c.y*4.0+u_time*0.018),0.15*cos(c.x*3.0-u_time*0.012),1.0));
        vec3 obsidian=vec3(0.006,0.009,0.023)+studio(reflect(-view,n))*vec3(0.025,0.022,0.045);
        return obsidian;
    }
    float footprint=max(max(length(dx),length(dy)),0.35/u_resolution.y);
    float pixelDistance=exp(clamp(f.logDistance-log(span)-log(footprint),-20.0,15.0));
    vec2 screenGradient=vec2(dot(f.gradient,dx),dot(f.gradient,dy));
    screenGradient/=max(length(screenGradient),1e-12);
    float fineFilter=exp(-0.30/(pixelDistance*pixelDistance+0.04));
    float relief=(0.20+0.65*cos(f.mu*0.30))*fineFilter;
    vec3 n=normalize(vec3(-screenGradient*relief,1.0));
    float facing=max(dot(n,view),0.0),fresnel=0.10+0.72*pow(1.0-facing,4.0);
    float band=f.mu*0.075+u_time*0.020+0.14*sin(f.mu*0.04);
    vec3 iterationColor=0.52+0.48*cos(vec3(0.0,2.094395,4.18879)+band);
    float coastalPhase=plane.x*1.55+plane.y*1.10+0.14*sin(plane.y*2.0)+u_time*0.018;
    vec3 coastalColor=0.52+0.48*cos(vec3(0.0,2.094395,4.18879)+coastalPhase);
    vec3 neon=mix(coastalColor,iterationColor,smoothstep(0.5,3.0,pixelDistance));
    vec3 chrome=studio(reflect(-view,n));
    float terraces=0.5+0.5*cos(f.mu*0.36)*fineFilter;
    vec3 color=mix(vec3(0.035,0.045,0.075),neon*0.24,0.65);
    color+=chrome*mix(vec3(0.65),neon,0.53)*(0.24+0.36*terraces+fresnel*0.30);
    vec3 key=normalize(vec3(-0.6,0.7,0.75));
    float spec=pow(max(dot(n,normalize(key+view)),0.0),44.0);
    color+=vec3(0.85,0.82,0.75)*spec;
    float core=exp(-pixelDistance*0.42),halo=exp(-pixelDistance*0.075);
    color+=neon*(core*1.10+halo*0.30);
    return color;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 color=vec3(0.0);
    // A fixed Gaussian footprint avoids content-dependent thresholds.
    // All pixels use the same filter throughout the complete flight.
    for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++){
        float weight=(x==0?2.0:1.0)*(y==0?2.0:1.0);
        color+=render(uv+vec2(x,y)*1.25/u_resolution.y)*weight;
    }
    color*=mix(1.0,0.87,u_night)/16.0;
    fragColor=vec4(pow(vec3(1.0)-exp(-max(color,vec3(0.0))*1.15),vec3(0.72)),1.0);
}
