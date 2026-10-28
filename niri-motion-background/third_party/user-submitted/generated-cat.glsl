// Original procedural pixel sprite replacing the missing iChannel1 image.
// Drawn entirely on the GPU, no external image, sampler or audio dependency.
vec4 nyan(vec2 p){
 vec2 uv=p*vec2(.4,1.);
 vec2 q=(uv-vec2(-.3,-.3))/vec2(.5,.6);
 if(any(lessThan(q,vec2(0.)))||any(greaterThanEqual(q,vec2(1.))))return vec4(0.);
 vec2 px=floor(q*vec2(24,16));float x=px.x,y=px.y;
 vec3 color=vec3(0.);bool fill=false;
 if(x>=1.&&x<=7.&&y>=6.&&y<=8.){fill=true;color=vec3(.12);if(y==7.)color=vec3(.55);}
 float feet=mod(floor(iTime*6.),2.);
 if(((x>=7.&&x<=9.)||(x>=14.&&x<=16.))&&y>=2.+feet&&y<=5.){fill=true;color=vec3(.13);}
 if(x>=5.&&x<=17.&&y>=4.&&y<=13.){
  fill=true;color=vec3(.13);
  if(x>5.&&x<17.&&y>4.&&y<13.)color=vec3(.88,.65,.39);
  if(x>6.&&x<16.&&y>5.&&y<12.){color=vec3(1.,.55,.75);if(mod(x*3.+y*5.,7.)<1.)color=vec3(.8,.2,.4);}
 }
 if((x>=16.&&x<=22.&&y>=5.&&y<=10.)||((x==17.||x==21.)&&y>=10.&&y<=12.)){
  fill=true;color=vec3(.16);if(x>=17.&&x<=21.&&y>=6.&&y<=10.)color=vec3(.6);
  if((x==18.||x==21.)&&y==8.)color=vec3(.05);if(x==20.&&y==7.)color=vec3(.18);if((x==17.||x==22.)&&y==6.)color=vec3(.98,.45,.6);
 }
 return fill?vec4(color,1.):vec4(0.);
}
