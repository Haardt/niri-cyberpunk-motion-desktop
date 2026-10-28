use anyhow::{bail, ensure, Context, Result};
use glow::HasContext;
use std::path::Path;

const VERTEX: &str = "#version 330 core\nout vec2 v_uv;\nvoid main(){ vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2); v_uv=vec2(p.x,1.0-p.y); gl_Position=vec4(p*2.0-1.0,0,1); }";
const IMAGE: &str = "#version 330 core\nin vec2 v_uv; out vec4 fragColor; uniform sampler2D u_image; uniform vec2 u_resolution; uniform vec2 u_image_size; void main(){ float a=u_resolution.x/u_resolution.y; float b=u_image_size.x/u_image_size.y; vec2 s=vec2(min(a/b,1.0),min(b/a,1.0)); fragColor=texture(u_image,(v_uv-.5)*s+.5); }";
pub struct Gpu {
    program: glow::Program,
    vao: glow::VertexArray,
    texture: Option<glow::Texture>,
    image_size: (u32, u32),
    stabilizer: Option<Stabilizer>,
}
impl Gpu {
    pub fn shader(gl: &glow::Context, path: &Path, stabilize: bool) -> Result<Self> {
        let mut gpu = Self::compile(
            gl,
            &std::fs::read_to_string(path)
                .with_context(|| format!("read shader {}", path.display()))?,
        )?;
        if stabilize {
            match Stabilizer::new(gl) {
                Ok(filter) => gpu.stabilizer = Some(filter),
                Err(e) => {
                    gpu.destroy(gl);
                    return Err(e);
                }
            }
        }
        Ok(gpu)
    }
    pub fn image(gl: &glow::Context, path: &Path) -> Result<Self> {
        let img = image::open(path)?.into_rgba8();
        let mut gpu = Self::compile(gl, IMAGE)?;
        unsafe {
            let tex = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                img.width() as i32,
                img.height() as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(img.as_raw())),
            );
            gpu.texture = Some(tex);
            gpu.image_size = img.dimensions();
        }
        Ok(gpu)
    }
    fn compile(gl: &glow::Context, fragment: &str) -> Result<Self> {
        let fragment=format!("{}\nuniform float nmb_brightness;\nvoid main() {{ nmb_main(); fragColor.rgb *= nmb_brightness; }}",fragment.replace("void main(","void nmb_main("));
        unsafe {
            let program = gl.create_program().map_err(anyhow::Error::msg)?;
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, fragment.as_str()),
            ] {
                let shader = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let error = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    gl.delete_program(program);
                    bail!("GLSL compilation: {error}");
                }
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let error = gl.get_program_info_log(program);
                gl.delete_program(program);
                bail!("GLSL link: {error}");
            }
            let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
            Ok(Self {
                program,
                vao,
                texture: None,
                image_size: (1, 1),
                stabilizer: None,
            })
        }
    }
    pub fn draw(&mut self, gl: &glow::Context, w: i32, h: i32, time: f32, night: bool, brightness: f32) -> Result<()> {
        self.draw_target(gl, None, w, h, time, night, brightness)
    }
    pub fn draw_target(
        &mut self,
        gl: &glow::Context,
        target: Option<glow::Framebuffer>,
        w: i32,
        h: i32,
        time: f32,
        night: bool,
        brightness: f32,
    ) -> Result<()> {
        if let Some(filter) = &mut self.stabilizer {
            filter.prepare(gl, w, h, night, brightness)?;
        }
        unsafe {
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                self.stabilizer.as_ref().and_then(|f| f.framebuffer).or(target),
            );
            gl.viewport(0, 0, w, h);
            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::SCISSOR_TEST);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_night").as_ref(),
                if night { 1. } else { 0. },
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "nmb_brightness")
                    .as_ref(),
                brightness,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_time").as_ref(),
                time,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_resolution")
                    .as_ref(),
                w as f32,
                h as f32,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_image_size")
                    .as_ref(),
                self.image_size.0 as f32,
                self.image_size.1 as f32,
            );
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.texture);
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_image").as_ref(), 0);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        if let Some(filter) = &mut self.stabilizer {
            filter.finish(gl, w, h);
        }
        Ok(())
    }
    pub fn destroy(self, gl: &glow::Context) {
        if let Some(filter) = self.stabilizer {
            filter.destroy(gl);
        }
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
            if let Some(t) = self.texture {
                gl.delete_texture(t);
            }
        }
    }
}

// Spatial edge/noise filter with neighborhood-clamped history. All buffers stay
// on the GPU. History is reset on renderer/style/size changes; no timer is added.
const STABILIZE: &str = r#"#version 330 core
uniform sampler2D u_current;
uniform sampler2D u_history;
uniform vec2 u_resolution;
uniform float u_ready;
out vec4 fragColor;
float luma(vec3 c){return dot(c,vec3(.2126,.7152,.0722));}
void main(){
    vec2 uv=gl_FragCoord.xy/u_resolution,stepUV=1./u_resolution;
    vec3 center=texture(u_current,uv).rgb;
    vec3 total=vec3(0.),lo=vec3(1.),hi=vec3(0.);
    float meanLight=0.;
    for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++){
        vec3 c=texture(u_current,uv+vec2(x,y)*stepUV).rgb;
        float weight=(x==0&&y==0)?4.:1.;
        total+=c*weight;meanLight+=luma(c);lo=min(lo,c);hi=max(hi,c);
    }
    meanLight/=9.;
    float centerLight=luma(center);
    // Bound isolated fireflies instead of letting a one-pixel highlight flash.
    float ceiling=meanLight*1.8+.045;
    center*=min(1.,ceiling/max(centerLight,.0001));
    vec3 current=(total-4.*texture(u_current,uv).rgb+4.*center)/12.;
    if(u_ready>.5){
        vec3 previous=clamp(texture(u_history,uv).rgb,lo-.015,hi+.015);
        float difference=length(previous-current);
        // Keep fast/disoccluded edges responsive; average slow subpixel noise.
        float historyWeight=.70*(1.-smoothstep(.25,.7,difference));
        current=mix(current,previous,historyWeight);
    }
    fragColor=vec4(current,1.);
}
"#;
struct Stabilizer {
    program: glow::Program,
    vao: glow::VertexArray,
    framebuffer: Option<glow::Framebuffer>,
    source: Option<glow::Texture>,
    history: Option<glow::Texture>,
    size: (i32, i32),
    ready: bool,
    style: Option<(bool, f32)>,
}
impl Stabilizer {
    fn new(gl: &glow::Context) -> Result<Self> {
        let pass = Gpu::compile(gl, STABILIZE)?;
        Ok(Self {
            program: pass.program,
            vao: pass.vao,
            framebuffer: None,
            source: None,
            history: None,
            size: (0, 0),
            ready: false,
            style: None,
        })
    }
    fn texture(gl: &glow::Context, w: i32, h: i32) -> Result<glow::Texture> {
        unsafe {
            let tex = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::LINEAR as i32);
            }
            for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::CLAMP_TO_EDGE as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w,
                h,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            Ok(tex)
        }
    }
    fn prepare(
        &mut self,
        gl: &glow::Context,
        w: i32,
        h: i32,
        night: bool,
        brightness: f32,
    ) -> Result<()> {
        unsafe {
            if self.style != Some((night, brightness)) {
                self.style = Some((night, brightness));
                self.ready = false;
            }
            if self.size != (w, h) {
                self.release_buffers(gl);
                self.source = Some(Self::texture(gl, w, h)?);
                self.history = Some(Self::texture(gl, w, h)?);
                let framebuffer = gl.create_framebuffer().map_err(anyhow::Error::msg)?;
                self.framebuffer = Some(framebuffer);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    self.source,
                    0,
                );
                ensure!(
                    gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                    "stabilizer framebuffer incomplete"
                );
                self.size = (w, h);
                self.ready = false;
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            Ok(())
        }
    }
    fn finish(&mut self, gl: &glow::Context, w: i32, h: i32) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, w, h);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "nmb_brightness")
                    .as_ref(),
                1.,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_resolution")
                    .as_ref(),
                w as f32,
                h as f32,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_ready").as_ref(),
                if self.ready { 1. } else { 0. },
            );
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.source);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_current").as_ref(),
                0,
            );
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, self.history);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_history").as_ref(),
                1,
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            // Copy the displayed image into history without a CPU readback.
            gl.bind_texture(glow::TEXTURE_2D, self.history);
            gl.copy_tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, 0, 0, w, h);
            self.ready = true;
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
    fn release_buffers(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(fbo) = self.framebuffer.take() {
                gl.delete_framebuffer(fbo);
            }
            if let Some(tex) = self.source.take() {
                gl.delete_texture(tex);
            }
            if let Some(tex) = self.history.take() {
                gl.delete_texture(tex);
            }
        }
    }
    fn destroy(mut self, gl: &glow::Context) {
        self.release_buffers(gl);
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
        }
    }
}
