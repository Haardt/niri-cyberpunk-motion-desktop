from pathlib import Path
import re
root=Path(__file__).resolve().parents[1]
raw=root/'third_party/user-submitted';shaders=root/'shaders'
HEADER='''#version 330 core
// Adapted from user-supplied Shadertoy source; see THIRD_PARTY.md.
// Silent: no audio input, output, recording or audio texture sampling.
uniform float u_time;
uniform vec2 u_resolution;
#define iTime u_time
#define iGlobalTime u_time
#define iResolution vec3(u_resolution,1.)
#define iMouse vec4(0.)
#define iTimeDelta 0.0
#define iFrame 0
out vec4 fragColor;
'''
def adapt(name,src):
 (raw/(name+'.glsl')).write_text(src)
 (shaders/(name+'.frag')).write_text(HEADER+src+'\nvoid main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}\n')
# Original livecode version includes unused FFT declarations; strip all of them.
s=(raw/'corroded-beasts.glsl').read_text()
s=s[s.index('#define time'):].replace('fGlobalTime','u_time').replace('v2Resolution','u_resolution').replace('out_color','fragColor')
s=s.replace('void main(void)','void main()').replace('0.5+0.1*rand','0.55').replace('0.5 + 0.1*rand','0.55')
(shaders/'corroded-beasts.frag').write_text(HEADER+'// Nusan, Corroded Beasts, https://lezanu.fr/LiveCode/CorrodedBeasts.glsl\n'+s)
# The GNOME port is by the original author and avoids the documented driver array issue.
s=(raw/'cube-lines.glsl').read_text().replace('#define SHADOW_ALPHA','//#define SHADOW_ALPHA').replace('#define USE_COLOR','//#define USE_COLOR')
s=s.replace('p = 0.5 * p / dot(p, p);','p = 0.5 * p / max(dot(p, p),1e-8);')
presets='\nconst float cam_shape=.5,cam_static=0.,cam_far=2.,custom_posx_shift=0.,custom_posy_shift=0.;\nconst bool use_alpha=false,use_shadow=false;\n#define ROTATION_SPEED .8999\n'
(shaders/'cube-lines.frag').write_text(HEADER+presets+s+'\nvoid main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}\n')
# Cartoon without audio-reactive sun sizing or the missing external Nyan sprite.
s=(raw/'fractal-cartoon.glsl').read_text()
s=s.replace('texture(iChannel0,vec2(.6,.2)).x','0.0').replace('texture2D(iChannel0,vec2(.6,.2)).x','0.0')
def remove_function(s,name):
 start=s.find('vec4 '+name+'(')
 if start<0:return s
 body=s.index('{',start);depth=1;end=body+1
 while depth:
  if s[end]=='{':depth+=1
  elif s[end]=='}':depth-=1
  end+=1
 return s[:start]+s[end:]
s=remove_function(s,'nyan')
s='vec4 nyan(vec2 p);\n'+s+'\n'+(raw/'generated-cat.glsl').read_text()
s=re.sub(r'float fov=.*?;', 'float fov=.8;',s)
s=s.replace('iGlobalTime*1.5','iGlobalTime*.35').replace('t*70.0','t*12.0').replace('t*70.','t*12.')
(shaders/'fractal-cartoon.frag').write_text(HEADER+s+'\nvoid main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}\n')
for name in ['cloud-orbit','cloud-sunlit','neon-menger','blue-rectangles','seascape-tdm','mandelbulb','synthwave-sunset','sky-path']:
 path=raw/(name+'.glsl')
 if path.exists():
  src=path.read_text()
  if name=='sky-path':
   src=src.replace('float dither=.8+.1*rnd(uv);','float dither=.85;')
  if name=='seascape-tdm':
   src=src.replace('vec3 ang=vec3(sin(time*3.)*.1,sin(time)*.2+.3,time)', 'vec3 ang=vec3(.015*sin(time*.3),.26+.018*sin(time*.2),0.)')
  if name=='neon-menger':
   src=src.replace('k=2.*pow(abs(sin(.5*iTime)),.2)', 'k=1.5+.45*sin(.12*iTime)')
  (shaders/(name+'.frag')).write_text(HEADER+src+'\nvoid main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}\n')

# Inercia 2022: preserve the reflective scene, remove twitchy presentation.
name='inercia-royale-2022'
path=raw/(name+'.glsl')
if path.exists():
 src=path.read_text().replace('#define SAMPLE_COUNT 10.','#define SAMPLE_COUNT 4.')
 src=src.replace('pow(smoothstep(0.,1.,fract(t)),10.)','smoothstep(0.,1.,fract(t))')
 src=src.replace('time=mod(iTime,300.);','time=iTime;')
 src=src.replace('uv.y-=curve(time*3.)*.1;uv*=1.+curve(time*3.+9.)*.3;','// No random camera bobbing or zoom for the wallpaper.')
 src=src.replace('float prog=time/6.-length(uv)*.05,style=mod(prog*.5,1.),scene=floor(prog);','float style=.5+.5*sin(time*.06);')
 src=src.replace('time=time*1.3+rnd(scene)*300.;','time=time*1.3;')
 src=src.replace('vec3(uv+7.+j,fract(time)+j*.2))*.5','vec3(uv+7.+j,j*.2))*.18')
 src=src.replace('sin(time/20.)*5.','sin(time*.025)*.6')
 src=src.replace('float fov=1.+sin(time*.25)*.5;','float fov=1.2;')
 src=src.replace('vec3 dof=rnd3(vec3(uv+7.+j,fract(time)+j*.2))*.5;dof.z=0.;', 'vec2 lens=vec2(cos(j*2.399963),sin(j*2.399963))*sqrt((j+.5)/count)*.12;vec3 dof=vec3(lens,0.);')
 src=src.replace('vec3 dof=rnd3(vec3(uv+7.+j,j*.2))*.18;dof.z=0.;', 'vec2 lens=vec2(cos(j*2.399963),sin(j*2.399963))*sqrt((j+.5)/count)*.12;vec3 dof=vec3(lens,0.);')
 src=src.replace('rnd3(vec3(uv+j,fract(time)+j*.1))', 'vec3(sin(loco.y*2.+j*2.399963),sin(loco.z*2.2+j*4.1),sin(loco.x*1.9+j*1.7))*.4')
 src=src.replace('step(style,.5)','smoothstep(.35,.65,style)')
 src=src.replace('normalize(d-vec3(map(p-off.xyy),map(p-off.yxy),map(p-off.yyx)))','normalize(d-vec3(map(p-off.xyy),map(p-off.yxy),map(p-off.yyx))+vec3(0.,1e-8,0.))')
 src=src.replace('float rough=.1+.4*rnd3(floor(loco*8.)).x;if(length(p)<3.1)rough=.2;', 'float rough=.12+.04*sin(dot(loco,vec3(.7,.9,.5)));if(length(p)<3.1)rough=.12;')
 src=src.replace('if(lum<.1)col+=vec3(3,.3,.3)*.0005/(.0001+abs(lum));', 'if(lum<.1)col+=alpha*vec3(1.,.2,.2)*.004/(.035+abs(lum));')
 src=src.replace('vec3 loco=loc;vec2 off=vec2(.01,0);','vec3 loco=loc;vec2 off=vec2(.008+.001*min(length(p-s),20.),0);')
 src=src.replace('normalize(d-vec3(map(p-off.xyy),map(p-off.yxy),map(p-off.yyx))+vec3(0.,1e-8,0.))', 'normalize(vec3(map(p+off.xyy)-map(p-off.xyy),map(p+off.yxy)-map(p-off.yxy),map(p+off.yyx)-map(p-off.yyx))+vec3(0.,1e-8,0.))')
 src='// Wallpaper port: 4 deterministic samples, smooth transitions, fixed lens, silent.\n'+src
 (shaders/(name+'.frag')).write_text(HEADER+src+'\nvoid main(){mainImage(fragColor,gl_FragCoord.xy);fragColor.a=1.;}\n')
