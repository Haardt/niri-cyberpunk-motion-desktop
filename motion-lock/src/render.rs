use crate::{buffer::Buffer, interface::Interface};
use anyhow::{ensure, Result};
use glow::HasContext;
use motion_render::{egl, gpu::Gpu};
use std::path::Path;
use wayland_client::Connection;

const VERT: &str = "#version 330 core\nout vec2 uv;void main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);uv=vec2(p.x,1.-p.y);gl_Position=vec4(p*2.-1.,0,1);}";
const FRAG: &str = "#version 330 core\nin vec2 uv;out vec4 fragColor;uniform sampler2D tex;void main(){fragColor=texture(tex,uv);}";
pub const UI: (u32, u32) = (800, 580);
pub struct Renderer {
    pub display: egl::Display,
    graphics: Option<Graphics>,
    shader: std::path::PathBuf,
    interface: Interface,
    pixels: Vec<u8>,
    dirty: bool,
}
struct Graphics {
    gl: glow::Context,
    gpu: Gpu,
    program: glow::Program,
    vao: glow::VertexArray,
    texture: glow::Texture,
    background: glow::Texture,
    fbo: glow::Framebuffer,
    bg_size: (i32, i32),
}
pub fn panel_rect(w: f64, h: f64, scale: f64) -> (f64, f64, f64, f64) {
    let k = scale
        .min(w * 0.88 / UI.0 as f64)
        .min(h * 0.84 / UI.1 as f64);
    let pw = UI.0 as f64 * k;
    let ph = UI.1 as f64 * k;
    ((w - pw) / 2., (h - ph) / 2., pw, ph)
}
impl Renderer {
    pub fn new(conn: &Connection, shader: &Path) -> Result<Self> {
        Ok(Self {
            display: egl::Display::new(conn)?,
            graphics: None,
            shader: shader.to_owned(),
            interface: Interface::new(),
            pixels: vec![0; (UI.0 * UI.1 * 4) as usize],
            dirty: true,
        })
    }
    /// Compile GLSL on an unroled, unmapped surface before requesting a session lock.
    /// A broken shader must not strand a user behind a newly acquired lock.
    pub fn prepare(
        &mut self,
        surface: &wayland_client::protocol::wl_surface::WlSurface,
    ) -> Result<()> {
        let temporary = self.display.surface(surface, 1, 1)?;
        self.display.current(&temporary)?;
        self.graphics = Some(unsafe { Graphics::new(self.display.gl(), &self.shader)? });
        self.dirty = true;
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn update(
        &mut self,
        question: &str,
        answer: &str,
        secret: bool,
        status: &str,
        preview: bool,
        config: &crate::config::Config,
        pending: bool,
        fingerprint: bool,
    ) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let mut buf = Buffer::new(&mut self.pixels, UI);
        self.interface.draw(
            &mut buf,
            config,
            crate::interface::Access {
                question,
                answer,
                secret,
                error: status,
                preview,
                waiting: pending,
                fingerprint,
                cancelling: false,
            },
        )?;
        if let Some(g) = &self.graphics {
            unsafe {
                g.gl.bind_texture(glow::TEXTURE_2D, Some(g.texture));
                g.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    UI.0 as i32,
                    UI.1 as i32,
                    0,
                    glow::BGRA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&self.pixels)),
                );
            }
        }
        self.dirty = false;
        Ok(())
    }
    pub fn draw(
        &mut self,
        surface: &egl::Surface,
        w: i32,
        h: i32,
        scale: i32,
        time: f32,
        brightness: f32,
        ui_scale: f32,
    ) -> Result<()> {
        self.display.current(surface)?;
        if self.graphics.is_none() {
            self.graphics = Some(unsafe { Graphics::new(self.display.gl(), &self.shader)? });
            self.dirty = true;
        }
        let g = self.graphics.as_mut().unwrap();
        // Half-resolution background, native-resolution UI; no CPU frame readback.
        let bw = (w / 2).max(1);
        let bh = (h / 2).max(1);
        unsafe {
            if g.bg_size != (bw, bh) {
                g.gl.bind_texture(glow::TEXTURE_2D, Some(g.background));
                g.gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    bw,
                    bh,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                g.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(g.fbo));
                g.gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(g.background),
                    0,
                );
                ensure!(
                    g.gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                    "background framebuffer incomplete"
                );
                g.bg_size = (bw, bh);
            }
            g.gpu
                .draw_target(&g.gl, Some(g.fbo), bw, bh, time, false, brightness)?;
            g.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(g.fbo));
            g.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            g.gl.blit_framebuffer(
                0,
                0,
                bw,
                bh,
                0,
                0,
                w,
                h,
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            g.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            let (x, y, pw, ph) = panel_rect(
                w as f64 / scale as f64,
                h as f64 / scale as f64,
                ui_scale as f64,
            );
            g.gl.viewport(
                (x * scale as f64) as i32,
                h - ((y + ph) * scale as f64) as i32,
                (pw * scale as f64) as i32,
                (ph * scale as f64) as i32,
            );
            g.gl.enable(glow::BLEND);
            g.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            g.gl.use_program(Some(g.program));
            g.gl.bind_vertex_array(Some(g.vao));
            g.gl.active_texture(glow::TEXTURE0);
            g.gl.bind_texture(glow::TEXTURE_2D, Some(g.texture));
            g.gl.draw_arrays(glow::TRIANGLES, 0, 3);
            g.gl.disable(glow::BLEND);
        }
        surface.swap()?;
        Ok(())
    }
}
impl Graphics {
    unsafe fn new(gl: glow::Context, path: &Path) -> Result<Self> {
        let gpu = Gpu::shader(&gl, path, false)?;
        let program = gl.create_program().map_err(anyhow::Error::msg)?;
        for (kind, src) in [(glow::VERTEX_SHADER, VERT), (glow::FRAGMENT_SHADER, FRAG)] {
            let s = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
            gl.shader_source(s, src);
            gl.compile_shader(s);
            ensure!(
                gl.get_shader_compile_status(s),
                "UI shader compilation: {}",
                gl.get_shader_info_log(s)
            );
            gl.attach_shader(program, s);
            gl.delete_shader(s);
        }
        gl.link_program(program);
        ensure!(gl.get_program_link_status(program), "UI shader link");
        let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
        let texture = gl.create_texture().map_err(anyhow::Error::msg)?;
        let background = gl.create_texture().map_err(anyhow::Error::msg)?;
        for t in [texture, background] {
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            for p in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, glow::LINEAR as i32);
            }
            for p in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, glow::CLAMP_TO_EDGE as i32);
            }
        }
        let fbo = gl.create_framebuffer().map_err(anyhow::Error::msg)?;
        Ok(Self {
            gl,
            gpu,
            program,
            vao,
            texture,
            background,
            fbo,
            bg_size: (0, 0),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panel_fits_small_and_hidpi_outputs() {
        for (w, h, s) in [
            (1440., 960., 1.5),
            (640., 480., 2.),
            (320., 240., 1.),
            (3840., 2160., 2.5),
        ] {
            let (x, y, pw, ph) = panel_rect(w, h, s);
            assert!(x >= 0. && y >= 0. && x + pw <= w && y + ph <= h);
            // The same transform drives hit-testing and rendering.
            let click_x = x + pw * (200. / 800.);
            let design_x = (click_x - x) / pw * 800.;
            assert!((design_x - 200.).abs() < 0.001);
        }
    }
}
