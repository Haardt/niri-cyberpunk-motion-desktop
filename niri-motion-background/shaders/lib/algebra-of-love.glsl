// Original scientific-terminal interpretation of code, mathematics and connection.
// Artistic sequence; the network is illustrative, the dynamics and Life are numerical.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
const float PI=3.14159265359;
const vec3 MINT=vec3(0.20,0.80,0.57);
const vec3 WHITE=vec3(0.73,0.88,0.79);
const vec3 AMBER=vec3(0.85,0.57,0.24);
mat2 rot(float a){return mat2(cos(a),-sin(a),sin(a),cos(a));}
float lineDistance(vec2 p,vec2 a,vec2 b){vec2 d=b-a;return length(p-a-d*clamp(dot(p-a,d)/max(dot(d,d),0.000001),0.0,1.0));}
float stroke(float d,float width){float aa=1.15/u_resolution.y;return 1.0-smoothstep(width,width+aa,d);}
vec3 ink(float d,vec3 color,float width){return color*(stroke(d,width)*0.82+exp(-d*170.0)*0.09);}
float roundedBox(vec2 p,vec2 b,float r){vec2 q=abs(p)-b+r;return length(max(q,0.0))+min(max(q.x,q.y),0.0)-r;}
float text(vec2 p,vec2 origin,float unit,int label){
    vec2 q=(p-origin)/unit;int column=int(floor(q.x/6.0));
    int x=int(floor(mod(q.x,6.0))),y=int(floor(-q.y));
    if(x<0||x>=5||y<0||y>=7||column<0)return 0.0;
    int c=labelChar(label,column);uvec2 bits=glyph(c);int bit=y*5+x;
    uint value=bit<32?((bits.x>>uint(bit))&1u):((bits.y>>uint(bit-32))&1u);
    vec2 f=vec2(fract(q.x),fract(-q.y))-0.5;
    float aa=clamp(0.75/(unit*u_resolution.y),0.1,0.7);
    return float(value)*(1.0-smoothstep(0.48-aa*0.5,0.48+aa*0.5,max(abs(f.x),abs(f.y))));
}
float weight(float phase,float center){float d=abs(phase-center);d=min(d,5.0-d);return 1.0-smoothstep(0.38,0.62,d);}
vec3 node(vec2 p,vec2 center,vec2 size,int label,int chars,float pulse){
    float d=roundedBox(p-center,size,0.009);
    vec3 c=MINT*(1.0-smoothstep(-0.001,0.002,d))*0.025;
    c+=ink(abs(d),mix(MINT,WHITE,pulse),0.0005);
    c+=WHITE*text(p,center+vec2(-float(chars)*0.0081,0.009),0.0027,label)*0.75;
    return c;
}
vec3 parseScene(vec2 p){
    vec3 c=vec3(0);vec2 root=vec2(0,0.18);
    for(int i=0;i<3;i++){
        float x=(float(i)-1.0)*0.40;vec2 a=vec2(0,0.135),b=vec2(x,0.015);
        float d=lineDistance(p,a,b);float progress=fract(u_time*0.10-float(i)*0.2);
        vec2 point=mix(a,b,progress);
        c+=ink(d,MINT*0.6,0.0005)+WHITE*exp(-length(p-point)*330.0)*0.6;
        float pulse=pow(max(sin(u_time*0.35-float(i)*1.4),0.0),4.0)*0.6;
        c+=node(p,vec2(x,-0.035),vec2(0.115,0.047),9+i,i==1?7:(i==0?5:2),pulse);
        c+=MINT*text(p,vec2(x-0.058,-0.114),0.0026,6+i)*0.65;
    }
    c+=node(p,root,vec2(0.10,0.04),24,4,0.25);
    c+=WHITE*text(p,vec2(-0.16,-0.225),0.0029,29)*0.55;
    return c;
}
vec3 geometryScene(vec2 p){
    vec3 c=vec3(0);vec3 points[8];float a=u_time*0.11;
    for(int i=0;i<8;i++){
        vec3 q=vec3((i&1)==0?-1.0:1.0,(i&2)==0?-1.0:1.0,(i&4)==0?-1.0:1.0)*0.13;
        q.xz=rot(a)*q.xz;q.yz=rot(0.4)*q.yz;points[i]=q;
    }
    vec2 center=vec2(-0.36,0.085);
    for(int i=0;i<8;i++)for(int axis=0;axis<3;axis++){
        int j=i^(1<<axis);if(j<=i)continue;
        c+=ink(lineDistance(p,center+points[i].xy,center+points[j].xy),mix(MINT,WHITE,points[i].z*2.0+0.5),0.0005);
    }
    c+=MINT*text(p,vec2(-0.56,0.285),0.0024,25)*0.65;
    // Analytic sine and its moving tangent.
    vec2 origin=vec2(0.29,0.10);vec2 prev=origin+vec2(-0.25,sin(-2.8)*0.07);
    float sineD=10.0;
    for(int j=1;j<=96;j++){
        float x=mix(-2.8,2.8,float(j)/96.0);vec2 next=origin+vec2(x*0.0893,sin(x)*0.07);
        sineD=min(sineD,lineDistance(p,prev,next));prev=next;
    }
    c+=ink(sineD,MINT,0.0008);
    float x=2.0*sin(u_time*0.15);vec2 at=origin+vec2(x*0.0893,sin(x)*0.07);
    vec2 tangent=normalize(vec2(0.0893,cos(x)*0.07));
    c+=ink(lineDistance(p,at-tangent*0.115,at+tangent*0.115),AMBER,0.0006);
    c+=WHITE*stroke(length(p-at),0.0032);
    c+=WHITE*text(p,vec2(0.05,0.285),0.0028,19)*0.65;
    c+=AMBER*text(p,vec2(0.05,0.235),0.0025,20)*0.6;
    // Circle circumference unrolls into a line, smoothly and periodically.
    float unroll=0.5-0.5*cos(u_time*0.17),radius=0.065;
    vec2 circle=vec2(-0.46+unroll*0.28,-0.15);
    c+=ink(abs(length(p-circle)-radius),MINT,0.0006);
    c+=ink(lineDistance(p,vec2(-0.46,-0.215),vec2(-0.46+unroll*2.0*PI*radius,-0.215)),AMBER,0.0005);
    c+=MINT*text(p,vec2(-0.56,-0.265),0.0022,26)*0.6;
    // An asymptotic approach stays finite on screen.
    float limitD=10.0;prev=vec2(0.06,-0.235);
    for(int j=1;j<=64;j++){
        float x=float(j)/64.0;vec2 next=vec2(0.06+x*0.47,-0.235+0.09*(1.0-exp(-x*5.0)));
        limitD=min(limitD,lineDistance(p,prev,next));prev=next;
    }
    c+=ink(limitD,MINT,0.0005);
    c+=ink(lineDistance(p,vec2(0.06,-0.145),vec2(0.53,-0.145)),WHITE*0.22,0.0003);
    c+=WHITE*text(p,vec2(0.05,-0.265),0.0022,21)*0.65;
    return c;
}
vec2 attractorPoint(vec3 q){q.xz=rot(0.3*sin(u_time*0.06))*q.xz;return vec2(-0.28,0.025)+q.xz*0.38;}
vec3 dynamicsScene(vec2 p,float clock){
    vec3 c=vec3(0);float d=10.0;vec2 prev=attractorPoint(LORENZ[0]);
    for(int i=1;i<768;i++){
        vec2 next=attractorPoint(LORENZ[i]);d=min(d,lineDistance(p,prev,next));prev=next;
    }
    c+=ink(d,MINT,0.00045);
    float head=clamp((clock-27.0)*40.0,0.0,766.9);int n=int(floor(head));
    vec2 moving=mix(attractorPoint(LORENZ[n]),attractorPoint(LORENZ[n+1]),fract(head));
    c+=WHITE*exp(-length(p-moving)*240.0)*0.8;
    c+=MINT*text(p,vec2(-0.59,0.285),0.0023,27)*0.7;
    c+=WHITE*text(p,vec2(-0.59,-0.26),0.0022,28)*0.55;
    // RK4 double-pendulum trajectory; restart is hidden outside this chapter.
    float sampleTime=clamp((clock-27.0)*20.0,0.0,382.9);
    int frame=int(floor(sampleTime));vec4 pend=mix(PENDULUM[frame],PENDULUM[frame+1],fract(sampleTime));
    vec2 anchor=vec2(0.37,0.235),a=anchor+pend.xy*0.105,b=anchor+pend.zw*0.105;
    c+=ink(lineDistance(p,anchor,a),WHITE*0.7,0.0007);
    c+=ink(lineDistance(p,a,b),MINT,0.0007);
    c+=WHITE*stroke(length(p-anchor),0.0035)+AMBER*stroke(length(p-a),0.007)+MINT*stroke(length(p-b),0.009);
    for(int i=1;i<22;i++){
        int j=max(0,frame-i);vec2 pa=anchor+PENDULUM[j].zw*0.105,pb=anchor+PENDULUM[j+1].zw*0.105;
        c+=ink(lineDistance(p,pa,pb),AMBER*(1.0-float(i)/22.0)*0.18,0.0002);
    }
    // Small Kepler-like orbital diagram, deliberately secondary to chaos.
    vec2 orbitCenter=vec2(0.37,-0.185);vec2 q=p-orbitCenter;
    c+=AMBER*stroke(length(q),0.007);
    for(int i=0;i<2;i++){
        float radius=0.074+float(i)*0.05;
        float ring=abs(length(q/vec2(1.0,0.53))-radius)*0.53;
        c+=ink(ring,MINT*0.28,0.0003);
        float angle=u_time*(0.35-float(i)*0.12)+float(i)*2.0;
        vec2 planet=orbitCenter+vec2(cos(angle),sin(angle)*0.53)*radius;
        c+=WHITE*stroke(length(p-planet),0.004);
    }
    return c;
}
float lifeAt(ivec2 cell,int generation){
    if(cell.x<0||cell.x>=32||cell.y<0||cell.y>=24)return 0.0;
    return float((LIFE[clamp(generation,0,63)*24+cell.y]>>uint(cell.x))&1u);
}
vec2 neuron(int layer,int index){return vec2(0.05+float(layer)*0.16,(float(index)-1.5)*0.095+0.04);}
vec3 lifeScene(vec2 p,float clock){
    vec3 c=vec3(0);vec2 q=(p-vec2(-0.585,-0.16))/0.015;
    ivec2 cell=ivec2(floor(q));float generation=clamp((clock-45.0)*1.6,0.0,62.9);
    int frame=int(floor(generation));float fade=smoothstep(0.0,0.35,fract(generation));
    float alive=mix(lifeAt(cell,frame),lifeAt(cell,frame+1),fade);
    float after=lifeAt(cell,max(frame-1,0))*(1.0-alive)*0.11;
    if(cell.x>=0&&cell.x<32&&cell.y>=0&&cell.y<24){
        vec2 f=abs(fract(q)-0.5);float shape=1.0-smoothstep(0.36,0.46,max(f.x,f.y));
        c+=MINT*shape*(alive*0.7+after+0.008);
        c+=WHITE*shape*alive*0.15;
    }
    c+=MINT*text(p,vec2(-0.59,0.285),0.0025,22)*0.75;
    c+=WHITE*text(p,vec2(0.04,0.285),0.0022,23)*0.65;
    float learned=smoothstep(45.0,59.0,clock);
    for(int layer=0;layer<3;layer++)for(int i=0;i<4;i++)for(int j=0;j<4;j++){
        vec2 a=neuron(layer,i),b=neuron(layer+1,j);
        float strength=0.10+0.26*(0.5+0.5*sin(float(i*7+j*3+layer)*1.7));
        float d=lineDistance(p,a,b);
        c+=ink(d,MINT*strength,0.0002);
        float flow=0.5+0.5*sin(u_time*0.9-float(layer)*1.1-float(i+j)*0.3);
        c+=ink(d,AMBER*pow(flow,12.0)*learned*0.1,0.0005);
    }
    for(int layer=0;layer<4;layer++)for(int i=0;i<4;i++){
        vec2 at=neuron(layer,i);float r=length(p-at);
        float signal=0.5+0.5*sin(u_time*0.8-float(layer)*0.9+float(i)*0.7);
        c+=ink(abs(r-0.007),mix(MINT,WHITE,signal*learned),0.0004);
        c+=MINT*exp(-r*280.0)*signal*0.22;
    }
    c+=mix(MINT,AMBER,learned)*text(p,vec2(0.25,-0.225),0.005,13)*(0.35+learned*0.6);
    return c;
}
vec3 heartScene(vec2 p,float clock){
    vec2 q=(p-vec2(0.0,0.0))*4.5;float x=q.x,y=q.y;
    float a=x*x+y*y-1.0;
    // Taking the real cube root preserves exactly the original zero set,
    // while avoiding its vanishing gradient at (+/-1,0).
    float x23=pow(max(abs(x),0.000001),2.0/3.0);
    float value=a-x23*y;
    vec2 gradient=vec2(2.0*x-y*(2.0/3.0)*sign(x)/pow(max(abs(x),0.00001),1.0/3.0),2.0*y-x23);
    float distance=abs(value)/max(length(gradient),0.035)/4.5;
    float angle=fract((atan(y-0.1,x)+PI)/(2.0*PI));
    float progress=clamp((clock-63.0)/9.0,0.0,1.0);
    float reveal=1.0-smoothstep(progress-0.025,progress+0.025,angle);
    vec3 color=mix(MINT,vec3(0.9,0.34,0.31),smoothstep(65.0,75.0,clock));
    float inside=1.0-smoothstep(-0.012,0.012,value);
    vec3 c=ink(distance,color,0.0011)*reveal;
    c+=color*inside*0.035*progress;
    c+=WHITE*text(p,vec2(-0.25,-0.28),0.00275,12)*0.78;
    // The cursor travels around the implicit curve's reveal angle, without flashes.
    c+=WHITE*text(p,vec2(-0.054,0.32),0.0045,13)*0.75*smoothstep(0.6,1.0,progress);
    return c;
}
void main(){
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.55);
    vec2 p=(gl_FragCoord.xy-0.5*u_resolution)/(u_resolution.y*framing);
    p*=1.0+0.012*dot(p,p);
    float clock=mod(u_time+9.0,90.0)-9.0;
    float phase=mod(u_time/18.0,5.0);
    vec3 col=vec3(0.003,0.009,0.007);
    float screen=1.0-smoothstep(-0.002,0.004,roundedBox(p,vec2(0.70,0.465),0.018));
    col+=vec3(0.003,0.009,0.006)*screen;
    col+=ink(abs(roundedBox(p,vec2(0.70,0.465),0.018)),MINT*0.24,0.0003);
    col+=MINT*text(p,vec2(-0.66,0.43),0.0038,0)*0.90;
    col+=ink(lineDistance(p,vec2(-0.66,0.382),vec2(0.66,0.382)),MINT*0.20,0.0002);
    for(int i=0;i<5;i++){
        float w=weight(phase,float(i));if(w<0.0001)continue;
        vec3 scene=vec3(0);
        if(i==0)scene=parseScene(p);
        else if(i==1)scene=geometryScene(p);
        else if(i==2)scene=dynamicsScene(p,clock);
        else if(i==3)scene=lifeScene(p,clock);
        else scene=heartScene(p,clock);
        col+=scene*w;
        col+=WHITE*text(p,vec2(0.12,0.427),0.0027,1+i)*w*0.70;
    }
    col+=ink(lineDistance(p,vec2(-0.66,-0.32),vec2(0.66,-0.32)),MINT*0.20,0.0002);
    for(int i=0;i<4;i++){
        float marker=0.5+0.5*sin(u_time*0.24-float(i)*1.4);
        col+=mix(MINT,WHITE,pow(marker,6.0))*text(p,vec2(-0.65,-0.345-float(i)*0.025),0.0022,14+i)*0.40;
    }
    col+=MINT*text(p,vec2(0.04,-0.355),0.0020,18)*0.50;
    float cursor=0.55+0.25*sin(u_time*1.5);
    col+=MINT*stroke(abs(roundedBox(p-vec2(0.059,-0.405),vec2(0.006,0.008),0.001)),0.0003)*cursor*0.5;
    // Static, subtle phosphor texture, never a brightness-modulating strobe.
    col*=0.992+0.008*sin(p.y*700.0);
    col*=mix(1.0,0.86,u_night);
    col*=1.0-0.16*smoothstep(0.35,0.9,length(p));
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.3),vec3(0.72)),1.0);
}
