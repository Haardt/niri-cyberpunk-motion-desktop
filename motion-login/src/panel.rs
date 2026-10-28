//! GPU presentation of the cached UI, scaled and briefly distorted without CPU readback.
use crate::config::Glitch;
use anyhow::{ensure, Result};
use glow::HasContext;
use motion_render::egl;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use wayland_client::{protocol::wl_surface::WlSurface, Connection, Proxy};

const VERTEX: &str = "#version 330 core\nout vec2 uv; void main(){vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);uv=vec2(p.x,1.-p.y);gl_Position=vec4(p*2.-1.,0,1);}";
const FRAGMENT: &str = include_str!("../shaders/panel.frag");
pub struct Panel {
    surfaces: HashMap<u32, egl::Surface>,
    gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    texture: glow::Texture,
    display: egl::Display,
}
impl Panel {
    /// Present on surfaces owned by the frontend; drop before their layer roles.
    pub fn new(conn: &Connection, surfaces: &[WlSurface], size: (i32, i32)) -> Result<Self> {
        let display = egl::Display::with_alpha(conn)?;
        let mut windows = HashMap::new();
        for surface in surfaces {
            let window = display.surface(surface, size.0, size.1)?;
            display.current(&window)?;
            windows.insert(surface.id().protocol_id(), window);
        }
        ensure!(!windows.is_empty(), "no login surfaces");
        let gl = display.gl();
        unsafe {
            let program = gl.create_program().map_err(anyhow::Error::msg)?;
            for (kind, source) in [
                (glow::VERTEX_SHADER, VERTEX),
                (glow::FRAGMENT_SHADER, FRAGMENT),
            ] {
                let shader = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                ensure!(
                    gl.get_shader_compile_status(shader),
                    "panel shader: {}",
                    gl.get_shader_info_log(shader)
                );
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            ensure!(
                gl.get_program_link_status(program),
                "panel link: {}",
                gl.get_program_info_log(program)
            );
            let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
            let texture = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for pname in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, pname, glow::LINEAR as i32);
            }
            for pname in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, pname, glow::CLAMP_TO_EDGE as i32);
            }
            Ok(Self {
                surfaces: windows,
                gl,
                program,
                vao,
                texture,
                display,
            })
        }
    }
    pub fn draw(
        &self,
        source: Option<(&[u8], (u32, u32))>,
        size: (i32, i32),
        effect: f32,
        phase: f32,
    ) -> Result<()> {
        for surface in self.surfaces.values() {
            surface.resize(size.0, size.1);
            self.display.current(surface)?;
            unsafe {
                let gl = &self.gl;
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                if let Some((bytes, (w, h))) = source {
                    gl.tex_image_2d(
                        glow::TEXTURE_2D,
                        0,
                        glow::RGBA8 as i32,
                        w as i32,
                        h as i32,
                        0,
                        glow::BGRA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(Some(bytes)),
                    );
                }
                gl.viewport(0, 0, size.0, size.1);
                gl.use_program(Some(self.program));
                gl.bind_vertex_array(Some(self.vao));
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.program, "amount").as_ref(),
                    effect,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.program, "phase").as_ref(),
                    phase,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
                ensure!(
                    gl.get_error() == glow::NO_ERROR,
                    "panel GPU rendering error"
                );
            }
            surface.swap()?;
        }
        Ok(())
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        if let Some(surface) = self.surfaces.values().next() {
            let _ = self.display.current(surface);
            unsafe {
                self.gl.delete_texture(self.texture);
                self.gl.delete_vertex_array(self.vao);
                self.gl.delete_program(self.program);
            }
        }
        self.surfaces.clear();
    }
}
/// Sleeps between short bursts; input defers a burst so it cannot distract while typing.
pub struct GlitchClock {
    config: Glitch,
    next: Instant,
}
impl GlitchClock {
    pub fn new(config: Glitch, now: Instant) -> Self {
        let next = now + Duration::from_secs(config.start_delay_seconds.into());
        Self { config, next }
    }
    pub fn trigger(&mut self, now: Instant) {
        self.next = now;
    }
    pub fn input(&mut self, now: Instant) {
        self.next = now + Duration::from_secs(self.config.interval_seconds.into());
    }
    pub fn sample(&mut self, now: Instant) -> (f32, f32, i32) {
        if !self.config.enabled || self.config.strength == 0. {
            return (0., 0., -1);
        }
        if now < self.next {
            return (
                0.,
                0.,
                (self.next - now).as_millis().max(1).min(i32::MAX as u128) as i32,
            );
        }
        let elapsed = now.duration_since(self.next).as_secs_f32();
        let duration = self.config.duration_ms as f32 / 1000.;
        if elapsed >= duration {
            self.next = now + Duration::from_secs(self.config.interval_seconds.into());
            return (0., 1., (self.config.interval_seconds * 1000) as i32);
        }
        let phase = elapsed / duration;
        (
            self.config.strength * (phase * std::f32::consts::PI).sin(),
            phase,
            33,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bursts_are_bounded_and_input_defers_them() {
        let start = Instant::now();
        let mut clock = GlitchClock::new(Glitch::default(), start);
        assert_eq!(clock.sample(start).0, 0.);
        assert_eq!(clock.sample(start).2, 3000);
        assert!(clock.sample(start + Duration::from_millis(3090)).0 > 0.);
        assert_eq!(clock.sample(start + Duration::from_millis(3400)).0, 0.);
        clock.input(start + Duration::from_secs(50));
        assert_eq!(clock.sample(start + Duration::from_secs(51)).0, 0.);
        assert_eq!(clock.sample(start + Duration::from_secs(51)).2, 9000);
        clock.trigger(start + Duration::from_secs(60));
        assert!(clock.sample(start + Duration::from_millis(60090)).0 > 0.);
        let config = Glitch {
            enabled: false,
            ..Glitch::default()
        };
        assert_eq!(GlitchClock::new(config, start).sample(start).2, -1);
    }
}
