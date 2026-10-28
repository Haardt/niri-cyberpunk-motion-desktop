#version 330 core
// Original magnetic black liquid sculpture, MIT. Analytic height field.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float heightAt(vec2 p) {
    vec2 magnet=vec2(sin(u_time*0.22),cos(u_time*0.17))*0.44;
    vec2 q=p-magnet;
    float field=exp(-dot(q,q)*0.24);
    float rows=floor(p.y/0.57);
    float spikes=0.0;
    for(int j=-1;j<=1;j++) {
        float row=rows+float(j);
        float shift=mod(row,2.0)*0.33;
        float column=floor((p.x-shift)/0.66+0.5);
        vec2 d=p-vec2(column*0.66+shift,row*0.57);
        spikes+=exp(-dot(d,d)*39.0);
    }
    float pulse=0.62+0.21*sin(u_time*0.42)+0.08*sin(length(q)*4.0-u_time*0.5);
    return -0.18+field*(0.16+spikes*pulse*1.3);
}
vec2 scene(vec3 p) {
    float fluid=max((p.y-heightAt(p.xz))*0.20,length(p.xz)-2.8);
    float ring=length(vec2(length(p.xz)-2.9,p.y+0.14))-0.07;
    float floorD=p.y+0.38;
    return fluid<ring && fluid<floorD?vec2(fluid,1.0):(ring<floorD?vec2(ring,2.0):vec2(floorD,0.0));
}
vec3 normalAt(vec3 p) {
    vec2 e=vec2(0.003,0.0);
    return normalize(vec3(scene(p+e.xyy).x-scene(p-e.xyy).x,scene(p+e.yxy).x-scene(p-e.yxy).x,scene(p+e.yyx).x-scene(p-e.yyx).x));
}
vec3 studio(vec3 d) {
    vec3 c=vec3(0.035,0.045,0.065);
    float strip=exp(-pow((d.x+0.45)*8.0,2.0))*smoothstep(0.0,0.5,d.y);
    float panel=smoothstep(0.3,0.34,d.y)*(1.0-smoothstep(0.72,0.76,d.y))
               *(1.0-smoothstep(0.4,0.45,abs(d.z-0.15)));
    float warm=pow(max(dot(d,normalize(vec3(0.8,0.5,-0.3))),0.0),24.0);
    return c+vec3(2.0,2.7,3.2)*strip+vec3(3.5)*panel+vec3(1.4,0.65,0.25)*warm;
}
void main() {
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/u_resolution.y;
    float angle=0.16*sin(u_time*0.06);
    vec3 ro=vec3(6.5*sin(angle),4.1,6.5*cos(angle));
    vec3 f=normalize(vec3(0.0,0.1,0.0)-ro),r=normalize(cross(f,vec3(0,1,0))),up=cross(r,f);
    vec3 rd=normalize(f*1.7+r*uv.x+up*uv.y);
    float distance=0.0;vec2 hit=vec2(0.0);
    for(int i=0;i<140;i++) {
        hit=scene(ro+rd*distance);
        if(hit.x<0.002 || distance>22.0) break;
        distance+=max(hit.x*0.85,0.002);
    }
    vec3 col=vec3(0.025,0.032,0.046);
    if(distance<22.0) {
        vec3 p=ro+rd*distance,n=normalAt(p);
        float fresnel=0.1+0.8*pow(1.0-max(dot(n,-rd),0.0),4.0);
        if(hit.y>0.5) {
            col=studio(reflect(rd,n))*(hit.y>1.5?0.7:fresnel+0.26);
            col+=vec3(0.009,0.013,0.02);
            float ao=clamp(scene(p+n*0.12).x/0.024,0.25,1.0);
            col*=0.45+0.55*ao;
        } else {
            col=vec3(0.045,0.055,0.075)*(0.65+0.35*max(n.y,0.0));
            col*=0.5+0.5*smoothstep(2.5,4.3,length(p.xz));
        }
    }
    col*=mix(1.0,0.78,u_night);
    fragColor=vec4(pow(col/(vec3(1.0)+col),vec3(0.4545)),1.0);
}
