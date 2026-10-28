#version 330 core
// Original abstract autumn pond, MIT. Koi are curved ink-and-light ribbons,
// with translucent flowing tails, rather than anatomical or polygonal fish.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float centerline(float s,float beat){return 0.019*sin(s*4.4-beat)*s*s;}
void main(){
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.45);
    vec2 p=(gl_FragCoord.xy-0.5*u_resolution)/(u_resolution.y*framing);
    float t=u_time*0.22,aa=1.25/(u_resolution.y*framing);
    vec2 surface=p+0.0018*vec2(sin(p.y*22.0+t*2.0),sin(p.x*19.0-t*1.5));
    vec3 col=mix(vec3(0.008,0.043,0.052),vec3(0.018,0.10,0.10),0.5+0.5*sin(p.x*2.7+p.y*2.0));
    float caustic=sin(surface.x*28.0+sin(surface.y*16.0+t))*sin(surface.y*24.0+sin(surface.x*13.0-t));
    col+=vec3(0.012,0.038,0.027)*pow(max(caustic,0.0),5.0);
    // Continuous orbital paths: the tangent sets the swimming direction.
    for(int i=0;i<5;i++){
        float f=float(i),phase=t*(0.15+f*0.012)+f*1.35;
        vec2 center=vec2(sin(phase)*(0.32+f*0.052),cos(phase*1.09+f*0.4)*0.27);
        vec2 tangent=vec2(cos(phase)*(0.32+f*0.052),-sin(phase*1.09+f*0.4)*0.2943);
        float angle=atan(tangent.y,tangent.x);
        float size=0.76+0.17*hash(vec2(f,2.0));
        vec2 q=rot(angle)*(surface-center)/size;
        float pixel=aa/size,beat=t*2.2+f*1.7;
        float s=clamp((0.115-q.x)/0.275,0.0,1.0);
        float spine=centerline(s,beat);
        float halfWidth=0.039*pow(max(sin(PI*s),0.0),0.58)*(1.0-0.52*s);
        float bodyD=max(abs(q.y-spine)-halfWidth,max(q.x-0.115,-0.160-q.x));
        float body=1.0-smoothstep(-pixel,pixel,bodyD);
        vec3 pearl=vec3(0.70,0.78,0.68);
        vec3 pigment=i<2?vec3(0.67,0.18,0.09):(i<4?vec3(0.22,0.56,0.54):vec3(0.76,0.49,0.20));
        float wash=0.5+0.5*sin(s*4.7+f*1.9);
        vec3 tint=mix(pearl,pigment,wash*0.75);
        float crossSection=clamp(1.0-abs(q.y-spine)/max(halfWidth,0.001),0.0,1.0);
        tint*=0.42+0.58*sqrt(crossSection);
        // Soft translucent pectoral strokes, integrated into the same curve.
        float finS=clamp((-q.x+0.035)/0.15,0.0,1.0);
        float finWidth=0.048*pow(max(sin(PI*finS),0.0),0.8);
        float finD=max(abs(q.y-spine)-finWidth,max(q.x-0.035,-0.115-q.x));
        float fins=(1.0-smoothstep(-pixel,pixel,finD))*0.23;
        col=mix(col,mix(pearl,pigment,0.5)*0.6,fins);
        col=mix(col,tint,body*0.82);
        float edge=exp(-abs(bodyD)/max(pixel*1.1,0.0005));
        col+=mix(pearl,pigment,0.25)*edge*0.15;
        // Paired tail filaments curve outwards and fade to hairline tips.
        float tailS=clamp((-q.x-0.13)/0.15,0.0,1.0);
        float tailBase=centerline(1.0,beat);
        float bend=0.023*sin(tailS*3.0-beat)*tailS;
        float spread=0.043*sin(PI*tailS*0.8);
        float thickness=0.008*pow(max(sin(PI*tailS),0.0),0.8)+0.001;
        float tailD=min(abs(q.y-tailBase-bend-spread),abs(q.y-tailBase-bend+spread))-thickness;
        float envelope=smoothstep(0.0,0.10,tailS)*(1.0-smoothstep(0.82,1.0,tailS));
        envelope*=1.0-smoothstep(0.275,0.285,-q.x);
        float tail=(1.0-smoothstep(-pixel,pixel,tailD))*envelope;
        col=mix(col,mix(pigment,pearl,tailS)*0.66,tail*0.58);
        // Fine interior flow lines replace eyes, scales and patchwork markings.
        for(int j=-1;j<=1;j++){
            float line=abs(q.y-spine-float(j)*halfWidth*0.48);
            float stroke=1.0-smoothstep(pixel*0.25,pixel*1.4,line);
            col+=mix(pearl,vec3(0.85,0.61,0.26),s)*stroke*body*0.08;
        }
        float wake=exp(-pow((q.y-tailBase-bend)*42.0,2.0))*smoothstep(0.18,0.29,-q.x)*(1.0-smoothstep(0.29,0.42,-q.x));
        col+=vec3(0.015,0.075,0.075)*wake;
    }
    // A few quiet autumn accents keep the existing pond identity.
    for(int i=0;i<5;i++){
        float f=float(i),seed=hash(vec2(f,8.0));
        vec2 center=vec2((hash(vec2(f,1.0))-0.5)*1.55,(seed-0.5)*0.80);
        center+=0.012*vec2(sin(t*0.25+f),cos(t*0.3+f));
        vec2 q=rot(f*2.7+0.07*sin(t+f))*(p-center);
        float a=atan(q.y,q.x),r=length(q);
        float lobes=0.70+0.21*cos(a*5.0)+0.09*cos(a*10.0);
        float radius=0.018+0.008*seed;
        float leaf=1.0-smoothstep(radius*lobes-aa,radius*lobes+aa,r);
        vec3 tint=mix(vec3(0.33,0.065,0.025),vec3(0.67,0.29,0.055),seed);
        col=mix(col,tint,leaf*0.85);
    }
    for(int i=0;i<5;i++){
        float f=float(i),age=mod(t*0.35+f*0.71,3.8);
        vec2 center=vec2((hash(vec2(f,4.0))-0.5)*1.5,(hash(vec2(f,7.0))-0.5)*0.8);
        float r=length(p-center);
        float ring=sin((r-age*0.12)*180.0)*exp(-abs(r-age*0.12)*110.0);
        col+=vec3(0.025,0.045,0.045)*ring*sin(age*0.826)*exp(-age);
    }
    col*=mix(1.0,0.76,u_night);
    fragColor=vec4(pow(max(col,vec3(0.0)),vec3(0.78)),1.0);
}
