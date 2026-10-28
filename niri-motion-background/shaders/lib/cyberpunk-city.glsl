// Original cinematic aerial city: exact grid traversal, static lit windows,
// soft billboard lights, reflected wet road. No textures, audio or random jitter.
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_night;
out vec4 fragColor;
float hash(vec2 p){vec3 q=fract(vec3(p.xyx)*.1031);q+=dot(q,q.yzx+33.33);return fract((q.x+q.y)*q.z);}
vec3 neon(vec2 cell){
    float h=hash(cell+4.7);
    vec3 light=mix(vec3(.04,1.8,3.0),vec3(2.4,.60,.12),step(.68,h));
    return mix(light,vec3(3.1,.035,1.3),step(.80,h));
}
vec3 building(vec2 cell){
    float h=hash(cell+1.1);
    return vec3(2.15+.50*hash(cell+2.3),5.+h*20.+step(.83,h)*12.,2.9+.6*hash(cell+7.6));
}
vec2 intersectBox(vec3 ro,vec3 rd,vec3 center,vec3 halfSize,out vec3 normal){
    vec3 safe=sign(rd)*max(abs(rd),vec3(1e-6));safe+=vec3(equal(safe,vec3(0.)))*1e-6;
    vec3 inv=1./safe;
    vec3 a=(center-halfSize-ro)*inv,b=(center+halfSize-ro)*inv;
    vec3 lo=min(a,b),hi=max(a,b);
    float nearT=max(lo.x,max(lo.y,lo.z)),farT=min(hi.x,min(hi.y,hi.z));
    normal=-sign(rd)*step(lo.yzx,lo.xyz)*step(lo.zxy,lo.xyz);
    return vec2(nearT,farT);
}
bool city(vec3 ro,vec3 rd,float limit,int budget,out float hitT,out vec3 normal,out vec2 hitCell){
    vec2 size=vec2(6.,8.),cell=floor((ro.xz+vec2(3.,0.))/size),direction=sign(rd.xz);
    vec2 safe=sign(rd.xz)*max(abs(rd.xz),vec2(1e-6));safe+=vec2(equal(safe,vec2(0.)))*1e-6;
    vec2 boundary=vec2((cell.x+.5*direction.x)*6.,(cell.y+step(0.,direction.y))*8.);
    vec2 next=(boundary-ro.xz)/safe,delta=size/max(abs(rd.xz),vec2(1e-6));
    float entered=0.;hitT=-1.;normal=vec3(0.);hitCell=cell;
    for(int i=0;i<72;i++){
        if(i>=budget||entered>limit)break;
        float leave=min(next.x,next.y);
        if(mod(cell.x+10000.,5.)>.5 && mod(cell.y+12000.,6.)>.5){
            vec3 b=building(cell),n;
            vec2 hit=intersectBox(ro,rd,vec3(cell.x*6.,b.y*.5,cell.y*8.+4.),vec3(b.x,b.y*.5,b.z),n);
            if(hit.x>max(entered-.002,.001)&&hit.x<min(leave+.002,limit)&&hit.y>hit.x){hitT=hit.x;normal=n;hitCell=cell;return true;}
        }
        if(next.x<next.y){entered=next.x;next.x+=delta.x;cell.x+=direction.x;}
        else {entered=next.y;next.y+=delta.y;cell.y+=direction.y;}
    }
    return false;
}
float panel(vec2 p,vec2 size){vec2 q=abs(p)-size;return length(max(q,0.))+min(max(q.x,q.y),0.);}
vec3 sky(vec3 rd){
    float horizon=exp(-abs(rd.y)*6.);
    vec3 col=mix(vec3(.012,.025,.045),vec3(.15,.065,.028),horizon);
    col+=vec3(.022,.07,.10)*exp(-rd.x*rd.x*5.-abs(rd.y)*8.);
    float moon=length(rd-normalize(vec3(-.55,.36,1.)));
    col+=vec3(.11,.045,.15)*exp(-moon*moon*35.);
    col+=vec3(.24,.15,.32)*(1.-smoothstep(.065,.072,moon));
    return col;
}
vec3 facade(vec3 p,vec3 normal,vec2 cell,float distance){
    vec3 b=building(cell),light=neon(cell),center=vec3(cell.x*6.,b.y*.5,cell.y*8.+4.);
    float aa=min(.28,max(.006,distance/u_resolution.y*1.2));
    float side=step(.5,abs(normal.x));
    float u=mix(p.x-center.x,p.z-center.z,side);
    float halfWidth=mix(b.x,b.z,side);
    vec3 base=vec3(.018,.025,.045)*(1.+.3*hash(cell+9.));
    vec3 color=base*(.6+.4*max(dot(normal,normalize(vec3(-.5,.8,-.3))),0.));
    vec2 windows=vec2(u*.75,p.y*.66),tile=fract(windows);
    float window=smoothstep(.14,.14+aa,tile.x)*(1.-smoothstep(.79-aa,.79,tile.x));
    window*=smoothstep(.18,.18+aa,tile.y)*(1.-smoothstep(.72-aa,.72,tile.y));
    float on=step(.42,hash(floor(windows)+cell*vec2(17.,31.)));
    vec3 windowColor=mix(light,vec3(1.,.49,.18),step(.79,hash(cell+13.)));
    float distant=1.-smoothstep(75.,180.,distance);
    color+=windowColor*window*on*(.30+.24*hash(cell+23.))*distant;
    // Broad facade corner trim, never an unresolved single-pixel sparkle.
    float edge=abs(abs(u)-halfWidth);
    float strip=1.-smoothstep(.025,.025+aa*2.,edge);
    color+=light*strip*.38*distant;
    color+=light*.14*exp(-edge*5.)*distant;
    float roof=abs(p.y-b.y);
    color+=light*.65*(1.-smoothstep(.04,.04+aa*2.,roof))*distant;
    if(normal.y>.5){
        vec2 q=p.xz-center.xz;
        vec3 roofColor=vec3(.048,.052,.064)*(1.+.4*hash(cell+32.));
        float edgeRoof=min(abs(abs(q.x)-b.x),abs(abs(q.y)-b.z));
        roofColor+=light*.50*exp(-edgeRoof*6.)*distant;
        // Roof plant/skylights read from above, not just repeated facade walls.
        float pad=1.-smoothstep(.04,.04+aa,panel(q-vec2(.45,.6),vec2(.72,.9)));
        roofColor=mix(roofColor,vec3(.065,.075,.09),pad*.8);
        float ribs=.5+.5*cos(q.y*10.);
        roofColor+=vec3(.025,.03,.04)*ribs*pad*(1.-smoothstep(100.,210.,distance));
        float skylight=1.-smoothstep(.02,.02+aa,panel(q+vec2(.8,1.1),vec2(.42,.75)));
        roofColor+=light*skylight*.55;
        return roofColor;
    }
    // Large signs and simple luminous glyph-like bars, fixed to world geometry.
    if(hash(cell+29.)>.32){
        vec2 signP=vec2(u+.75,p.y-min(8.,b.y*.58));
        float sd=panel(signP,vec2(.55,1.8));
        float signAA=min(aa,.12);
        float border=1.-smoothstep(.025,.025+signAA,abs(sd));
        float inside=1.-smoothstep(-signAA,signAA,sd);
        color=mix(color,vec3(.018,.008,.035),inside*.9);
        float row=abs(fract((signP.y+1.8)*1.05)-.5);
        float glyph=(1.-smoothstep(.12,.12+signAA,row))*(1.-smoothstep(.12,.42,abs(signP.x)));
        color+=light*(border*1.6+glyph*inside*1.1);
        color+=light*.25*exp(-abs(sd)*4.);
    }
    return color;
}
vec3 fog(vec3 color,vec3 ray,float distance){
    vec3 haze=mix(vec3(.015,.035,.065),vec3(.11,.035,.085),smoothstep(65.,270.,distance));
    haze+=sky(ray)*.25;
    return mix(color,haze,1.-exp(-distance*(CITY_RAIN==1?.0075:.006)));
}

// One reflected city ray, shaded without recursive reflections. Wet rooftops
// and glass now mirror neighboring emissive buildings rather than just tinting.
vec3 wetSurface(vec3 p,vec3 n,vec2 cell,vec3 viewRay,float distance){
    vec3 color=facade(p,n,cell,distance);
    vec3 wetN=n;
    if(n.y>.5)wetN=normalize(n+vec3(.012*sin(p.z*.8+p.x),0.,.012*sin(p.x*.7-p.z)));
    vec3 r=reflect(viewRay,wetN);
    float floorLimit=r.y<-.0001?-p.y/r.y:150.;
    float t;vec3 rn;vec2 rc;
    vec3 reflected=sky(r)*1.5;
    if(city(p+n*.035,r,min(150.,floorLimit),36,t,rn,rc)){
        reflected=facade(p+n*.035+r*t,rn,rc,t+distance);
        reflected=fog(reflected,r,t*.5);
    }else if(floorLimit>0.&&floorLimit<150.){
        vec3 q=p+r*floorLimit;
        vec2 cellBelow=floor((q.xz+vec2(3.,0.))/vec2(6.,8.));
        reflected=neon(cellBelow)*.14+vec3(.008,.012,.022);
    }
    reflected+=vec3(.03,.32,.55)*exp(-pow(r.x+.5,2.)*4.-pow(r.z-.7,2.)*5.);
    reflected+=vec3(.44,.02,.23)*exp(-pow(r.x-.6,2.)*5.-pow(r.z-.5,2.)*4.);
    // Broad smooth reflected glow: no stochastic roughness or pinpoint samples.
    float facing=clamp(dot(-viewRay,wetN),0.,1.);
    float amount=(n.y>.5?.36:.24)+.45*pow(1.-facing,3.);
    return mix(color,reflected,amount);
}
float rainLayer(vec2 uv,float columns,float rows,float fall,float seed){
    vec2 p=vec2((uv.x+uv.y*.16+u_time*.018)*columns,(uv.y+u_time*fall)*rows);
    float column=floor(p.x);
    float phase=hash(vec2(column,seed));
    float center=.30+.40*hash(vec2(column+11.,seed));
    float width=max(.018,1.3*columns/u_resolution.y);
    float stroke=1.-smoothstep(width,width*2.,abs(fract(p.x)-center));
    float along=abs(fract(p.y+phase)-.5);
    float tail=1.-smoothstep(.18,.43,along);
    float visible=step(.28,hash(vec2(column+23.,seed)));
    return stroke*tail*visible;
}
vec3 rainVeil(vec2 uv){
    float nearRain=rainLayer(uv,30.,7.,.40,1.);
    float midRain=rainLayer(uv,48.,11.,.28,7.);
    float farRain=rainLayer(uv,70.,17.,.19,13.);
    vec3 tint=mix(vec3(.15,.62,.9),vec3(.8,.12,.65),smoothstep(-.6,.6,uv.x));
    return tint*(nearRain*.23+midRain*.12+farRain*.06);
}
void main(){
    vec2 uv=(gl_FragCoord.xy-.5*u_resolution)/u_resolution.y;
    float travel=u_time*1.35;
    vec3 ro=vec3(8.+u_time*.38+.45*sin(u_time*.025),54.+.7*sin(u_time*.035),travel);
    vec3 forward=normalize(vec3(.35+.015*sin(u_time*.018),-.60,1.));
    vec3 right=normalize(cross(vec3(0,1,0),forward)),up=cross(forward,right);
    vec3 rd=normalize(forward*1.22+right*uv.x+up*uv.y);
    float floorT=rd.y<-.0001?-ro.y/rd.y:320.;
    float hitT;vec3 normal;vec2 cell;
    vec3 color=sky(rd);
    if(city(ro,rd,min(floorT,310.),72,hitT,normal,cell)){
        color=fog(wetSurface(ro+rd*hitT,normal,cell,rd,hitT),rd,hitT);
    } else if(floorT>0.&&floorT<320.){
        vec3 p=ro+rd*floorT;
        // Low-amplitude coherent ripple: long reflected streaks without grain.
        vec3 n=normalize(vec3(.010*sin(p.z*.7+p.x*2.),1.,.007*sin(p.x*1.7-p.z*.35)));
        vec3 reflected=reflect(rd,n);float mirrorT;vec3 mirrorN;vec2 mirrorCell;
        vec3 reflection=sky(reflected);
        if(city(p+vec3(0,.02,0),reflected,90.,28,mirrorT,mirrorN,mirrorCell)){
            reflection=facade(p+vec3(0,.02,0)+reflected*mirrorT,mirrorN,mirrorCell,mirrorT+floorT);
            reflection=fog(reflection,reflected,mirrorT);
        }
        float fresnel=.36+.60*pow(1.-max(dot(-rd,n),0.),3.);
        color=mix(vec3(.008,.012,.024),reflection,fresnel*.96);
        vec2 junction=vec2(mod(p.x+15.,30.)-15.,mod(p.z+24.,48.)-24.);
        color+=vec3(.12,.38,.55)*exp(-dot(junction*vec2(.23,.16),junction*vec2(.23,.16)));
        float footprint=max(.025,floorT/u_resolution.y*1.4);
        float lane=1.-smoothstep(.025,.025+footprint,abs(mod(p.x+15.,30.)-15.));
        float dash=smoothstep(.1,.3,fract(p.z/4.))*(1.-smoothstep(.65,.85,fract(p.z/4.)));
        color+=vec3(.08,.6,.85)*lane*dash;
        float curb=1.-smoothstep(.04,.04+footprint,abs(mod(p.z+24.,48.)-24.));
        color+=vec3(.2,.7,1.1)*curb;
        color=fog(color,rd,floorT);
    }
#if CITY_RAIN
    color+=rainVeil(uv);
#endif
    color*=mix(1.,.85,u_night);
    color=1.-exp(-max(color,vec3(0.))*1.25);
    color=pow(color,vec3(.82));
    color*=1.-.16*smoothstep(.3,1.,length(uv));
    fragColor=vec4(color,1.);
}
