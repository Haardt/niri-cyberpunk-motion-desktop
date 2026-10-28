#version 330 core
// Original procedural green chrome cube sea, MIT. No textures or audio.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1,311.7))) * 43758.5453); }
mat2 rotate(float a) { return mat2(cos(a),-sin(a),sin(a),cos(a)); }
vec3 safeDirection(vec3 d) { return mix(vec3(-1.0), vec3(1.0), step(vec3(0.0),d)) * max(abs(d),vec3(0.00001)); }

// Exact box intersections inside a 2D grid keep both primary and reflected rays bounded.
// Each stack uses the same footprint and yaw, so every cube has full support.
float trace(vec3 ro, vec3 rd, float limit, out vec3 normal, out float material) {
    normal=vec3(0,1,0); material=0.0;
    float best=limit;
    if(rd.y < -0.00001) {
        float floorHit=-ro.y/rd.y;
        if(floorHit>0.001 && floorHit<best) best=floorHit;
    }
    vec2 cell=floor(ro.xz/3.0);
    vec2 stepCell=mix(vec2(-1.0),vec2(1.0),step(vec2(0.0),rd.xz));
    vec2 inv=1.0/safeDirection(rd).xz;
    vec2 boundary=(cell+step(vec2(0.0),rd.xz))*3.0;
    vec2 next=(boundary-ro.xz)*inv;
    vec2 delta=abs(3.0*inv);
    float entry=0.0;
    for(int i=0;i<64;i++) {
        if(entry>best || entry>limit) break;
        float seed=hash(cell);
        float size=0.38+1.30*hash(cell+7.3);
        float yaw=(hash(cell+2.1)-0.5)*0.9;
        mat2 rot=rotate(yaw);
        vec2 center=(cell+0.5)*3.0;
        center+=(vec2(hash(cell+1.2),hash(cell+5.4))-0.5)*0.40;
        int count=seed>0.81?3:(seed>0.55?2:1);
        vec3 direction=rd;
        direction.xz=rot*direction.xz;
        vec3 reciprocal=1.0/safeDirection(direction);
        for(int j=0;j<3;j++) {
            if(j>=count) break;
            vec3 origin=ro-vec3(center.x,size*(float(j)+0.5),center.y);
            origin.xz=rot*origin.xz;
            vec3 halfSize=vec3(size*0.5);
            vec3 a=(-halfSize-origin)*reciprocal;
            vec3 b=(halfSize-origin)*reciprocal;
            vec3 nearP=min(a,b),farP=max(a,b);
            float start=max(nearP.x,max(nearP.y,nearP.z));
            float end=min(farP.x,min(farP.y,farP.z));
            if(start>0.002 && start<=end && start<best) {
                best=start;
                vec3 q=origin+direction*start;
                // Exact face normals keep reflected rays outside the solid cube.
                vec3 n=vec3(0.0);
                if(nearP.x>=nearP.y && nearP.x>=nearP.z) n.x=-sign(direction.x);
                else if(nearP.y>=nearP.z) n.y=-sign(direction.y);
                else n.z=-sign(direction.z);
                normal=n;
                normal.xz=transpose(rot)*normal.xz;
                material=1.0;
            }
        }
        if(next.x<next.y) {entry=next.x; next.x+=delta.x; cell.x+=stepCell.x;}
        else {entry=next.y; next.y+=delta.y; cell.y+=stepCell.y;}
    }
    return best;
}
vec3 studio(vec3 d) {
    vec3 color=mix(vec3(0.045,0.062,0.055),vec3(0.21,0.27,0.25),smoothstep(-0.1,0.9,d.y));
    // Broad rectangular softboxes reflected in the green metal.
    float panel=smoothstep(0.69,0.73,d.y)*(1.0-smoothstep(0.91,0.95,d.y))
               *(1.0-smoothstep(0.20,0.26,abs(d.x+0.16)));
    float strip=exp(-pow((d.x-0.65)*19.0,2.0))*smoothstep(-0.05,0.20,d.y);
    float rim=pow(max(dot(d,normalize(vec3(-0.8,0.35,-0.5))),0.0),40.0);
    float broad=exp(-pow((d.y-0.48)*4.5,2.0))*smoothstep(-0.7,0.1,d.z);
    color+=vec3(2.6,3.0,2.8)*broad;
    color+=vec3(3.8,4.1,3.9)*panel+vec3(2.3,2.8,2.5)*strip+vec3(1.8,2.2,1.9)*rim;
    return color;
}
vec3 surface(vec3 p, vec3 n, vec3 rd, float material, vec3 reflection) {
    vec3 light=normalize(vec3(-0.45,0.9,-0.3));
    float diffuse=max(dot(n,light),0.0);
    if(material<0.5) {
        float tile=0.5+0.5*sin(p.x*1.0472)*sin(p.z*1.0472);
        vec3 base=mix(vec3(0.045,0.055,0.048),vec3(0.075,0.085,0.072),tile);
        return base*(0.45+0.55*diffuse)+reflection*0.22;
    }
    float facing=max(dot(n,-rd),0.0);
    vec3 metal=vec3(0.07,0.72,0.20);
    vec3 fresnel=metal+(vec3(0.92)-metal)*pow(1.0-facing,5.0);
    vec3 halfDir=normalize(light-rd);
    float highlight=pow(max(dot(n,halfDir),0.0),180.0);
    float contact=mix(0.55,1.0,smoothstep(0.0,0.25,p.y));
    return (reflection*fresnel+metal*(0.035+0.055*diffuse)+vec3(1.6,2.0,1.65)*highlight)*contact;
}
void main() {
    vec2 uv=(gl_FragCoord.xy-0.5*u_resolution)/max(u_resolution.y,1.0);
    float t=u_time*0.12;
    vec3 ro=vec3(1.5+0.65*sin(t*0.3),6.8,-8.0+t);
    vec3 target=ro+vec3(0.45,-3.9,8.0);
    vec3 forward=normalize(target-ro);
    vec3 right=normalize(cross(forward,vec3(0,1,0)));
    vec3 up=cross(right,forward);
    vec3 rd=normalize(forward*1.55+right*uv.x+up*uv.y);
    vec3 n; float material;
    float distance=trace(ro,rd,85.0,n,material);
    vec3 col=studio(rd);
    if(distance<85.0) {
        vec3 p=ro+rd*distance;
        vec3 reflected=reflect(rd,n);
        vec3 rn; float rm;
        float reflectionDistance=trace(p+n*0.008,reflected,32.0,rn,rm);
        vec3 bounce=studio(reflected);
        if(reflectionDistance<32.0) {
            vec3 rp=p+n*0.008+reflected*reflectionDistance;
            bounce=surface(rp,rn,reflected,rm,studio(reflect(reflected,rn)));
            bounce=mix(bounce,studio(reflected),1.0-exp(-reflectionDistance*0.012));
        }
        col=surface(p,n,rd,material,bounce);
        col=mix(col,vec3(0.17,0.23,0.19),1.0-exp(-distance*0.009));
    }
    col*=mix(1.0,0.65,u_night);
    col=col/(vec3(1.0)+col);
    col=pow(max(col,vec3(0.0)),vec3(0.4545));
    fragColor=vec4(col,1.0);
}
