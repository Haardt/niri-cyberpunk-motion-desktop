#version 330 core
// Original translucent galleries containing divergent landscapes. MIT.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.0-2.0*f);return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),mix(hash(i+vec2(0,1)),hash(i+1.0),f.x),f.y);}
float ridge(float x,float seed){return noise(vec2(x*2.8,seed))*0.3+noise(vec2(x*7.5,seed+9.0))*0.11+0.06*sin(x*4.0+seed);}
vec3 world(vec2 p,float seed){
    float alignment=pow(0.5+0.5*sin(u_time*0.13),8.0);
    seed*=1.0-alignment;
    float warm=0.5+0.5*sin(seed*1.7);
    vec3 skyTop=mix(vec3(0.045,0.20,0.30),vec3(0.19,0.12,0.30),warm);
    vec3 skyBottom=mix(vec3(0.50,0.74,0.72),vec3(0.94,0.52,0.30),warm);
    vec3 c=mix(skyBottom,skyTop,smoothstep(-0.1,1.0,p.y));
    float sun=length((p-vec2(0.25*sin(seed),0.47))*vec2(1.0,1.0));
    c+=vec3(0.9,0.65,0.3)*(1.0-smoothstep(0.070,0.077,sun))*0.7;
    c+=vec3(0.25,0.14,0.06)*exp(-sun*7.0);
    float y=p.y;
    bool water=p.y<-0.18;
    if(water)y=-0.36-p.y+0.008*sin(p.x*75.0+p.y*30.0+u_time*0.45);
    for(int i=0;i<4;i++){
        float layer=float(i);
        float h=ridge(p.x+layer*0.21,seed+layer*3.7)-0.03-layer*0.15;
        vec3 tint=mix(vec3(0.08,0.19,0.24),vec3(0.19,0.09,0.12),warm);
        tint=mix(tint,skyBottom,0.55-layer*0.11);
        c=mix(c,tint,1.0-smoothstep(h-0.006,h+0.006,y));
    }
    if(water){
        c*=vec3(0.62,0.78,0.82);
        float glint=pow(0.5+0.5*sin(p.x*50.0+p.y*110.0+u_time*0.5),12.0);
        c+=vec3(0.07,0.1,0.11)*glint;
    }
    return c;
}
float roundBox(vec2 p,vec2 b,float r){vec2 q=abs(p)-b+r;return length(max(q,0.0))+min(max(q.x,q.y),0.0)-r;}
vec3 panes(vec3 ro,vec3 rd,vec2 screen){
    vec3 col=vec3(0.008,0.016,0.025);
    float align=pow(0.5+0.5*sin(u_time*0.13),8.0);
    for(int i=9;i>=0;i--){
        float f=float(i),layer=floor(f*0.5),side=mod(f,2.0)<0.5?-1.0:1.0;
        float yaw=side*(0.15+0.07*sin(u_time*0.06+layer));
        vec3 center=vec3(side*(1.15+0.1*sin(layer)),1.66,1.2-layer*3.3);
        vec3 right=vec3(cos(yaw),0,sin(yaw)),normal=vec3(-sin(yaw),0,cos(yaw));
        float denom=dot(rd,normal);
        if(abs(denom)<0.0001)continue;
        float t=dot(center-ro,normal)/denom;
        if(t<0.01 || t>70.0)continue;
        vec3 p=ro+rd*t;
        vec2 q=vec2(dot(p-center,right),p.y-center.y);
        float d=roundBox(q,vec2(0.84,1.50),0.045);
        float aa=max(0.002,t*0.8/u_resolution.y);
        float mask=1.0-smoothstep(-aa,aa,d);
        if(mask<0.001)continue;
        vec2 local=q*0.68;
        vec2 view=mix(local,screen*3.0,align);
        view.x+=rd.x*0.12*(1.0-align);
        vec3 scene=world(view,f*1.91);
        float facing=abs(dot(rd,normal));
        float fresnel=0.04+0.8*pow(1.0-facing,5.0);
        float border=1.0-smoothstep(0.005,0.018,abs(d+0.025));
        float edge=1.0-smoothstep(0.001,0.007,abs(d+0.005));
        vec3 glass=scene*0.88+vec3(0.08,0.18,0.23)*fresnel;
        glass+=mix(vec3(0.18,0.45,0.53),vec3(0.50,0.34,0.18),mod(f,2.0))*(border*0.3+edge*1.4);
        glass+=vec3(0.15,0.22,0.24)*exp(-pow((q.x+0.42+q.y*0.17)*23.0,2.0))*0.25;
        float fog=1.0-exp(-t*0.025);
        glass=mix(glass,vec3(0.025,0.05,0.065),fog);
        col=mix(col,glass,mask*0.86);
    }
    return col;
}
void main(){
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    vec3 ro=vec3(0.14*sin(u_time*0.06),1.65,7.0);
    float framing=min(1.0,(u_resolution.x/u_resolution.y)/1.45);
    vec3 rd=normalize(vec3(uv.x,uv.y,-1.18*framing));
    vec3 col=panes(ro,rd,uv);
    if(rd.y<0.0){
        float t=-ro.y/rd.y;
        if(t<40.0){
            vec3 p=ro+rd*t;
            vec3 n=normalize(vec3(0.0015*sin(p.z*3.0),1.0,0.001*sin(p.x*4.0)));
            vec3 reflection=panes(p+vec3(0,0.005,0),reflect(rd,n),uv);
            vec3 floorColor=vec3(0.012,0.021,0.03)+reflection*0.38;
            // The nearest pane is above the floor; blend only the lower stage.
            float floorMask=1.0-smoothstep(-0.30,-0.22,uv.y);
            col=mix(col,floorColor,floorMask);
        }
    }
    col+=vec3(0.007,0.014,0.022)*exp(-abs(uv.x)*4.0);
    col*=mix(1.0,0.80,u_night);
    fragColor=vec4(pow(vec3(1.0)-exp(-col*1.35),vec3(0.68)),1.0);
}
