//! Real offscreen GPU regression, opt-in so ordinary tests work without a GPU.
use glow::HasContext;
use motion_render::gpu;
use std::{
    ffi::{c_void, CString},
    ptr,
};
type Handle = *mut c_void;
#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetPlatformDisplay(platform: u32, native: Handle, attrs: *const isize) -> Handle;
    fn eglInitialize(display: Handle, major: *mut i32, minor: *mut i32) -> u32;
    fn eglBindAPI(api: u32) -> u32;
    fn eglChooseConfig(
        display: Handle,
        attrs: *const i32,
        configs: *mut Handle,
        size: i32,
        count: *mut i32,
    ) -> u32;
    fn eglCreateContext(
        display: Handle,
        config: Handle,
        share: Handle,
        attrs: *const i32,
    ) -> Handle;
    fn eglCreatePbufferSurface(display: Handle, config: Handle, attrs: *const i32) -> Handle;
    fn eglMakeCurrent(display: Handle, draw: Handle, read: Handle, context: Handle) -> u32;
    fn eglGetProcAddress(name: *const std::ffi::c_char) -> *const c_void;
    fn eglDestroySurface(display: Handle, surface: Handle) -> u32;
    fn eglDestroyContext(display: Handle, context: Handle) -> u32;
    fn eglTerminate(display: Handle) -> u32;
}
struct Context {
    display: Handle,
    surface: Handle,
    context: Handle,
    gl: glow::Context,
}
impl Context {
    fn new() -> Self {
        unsafe {
            let display = eglGetPlatformDisplay(0x31DD, ptr::null_mut(), ptr::null());
            assert!(!display.is_null());
            assert_ne!(eglInitialize(display, ptr::null_mut(), ptr::null_mut()), 0);
            assert_ne!(eglBindAPI(0x30A2), 0);
            let attrs = [
                0x3033, 1, 0x3040, 8, 0x3024, 8, 0x3023, 8, 0x3022, 8, 0x3038,
            ];
            let mut config = ptr::null_mut();
            let mut count = 0;
            assert_ne!(
                eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut count),
                0
            );
            assert!(count > 0);
            let context = eglCreateContext(
                display,
                config,
                ptr::null_mut(),
                [0x3098, 3, 0x30FB, 3, 0x30FD, 1, 0x3038].as_ptr(),
            );
            assert!(!context.is_null());
            let surface = eglCreatePbufferSurface(
                display,
                config,
                [0x3057, 768, 0x3056, 512, 0x3038].as_ptr(),
            );
            assert!(!surface.is_null());
            assert_ne!(eglMakeCurrent(display, surface, surface, context), 0);
            let gl = glow::Context::from_loader_function(|name| {
                eglGetProcAddress(CString::new(name).unwrap().as_ptr())
            });
            Self {
                display,
                surface,
                context,
                gl,
            }
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(
                self.display,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            eglDestroySurface(self.display, self.surface);
            eglDestroyContext(self.display, self.context);
            eglTerminate(self.display);
        }
    }
}
fn frame(
    gpu: &mut gpu::Gpu,
    gl: &glow::Context,
    w: i32,
    h: i32,
    time: f32,
    brightness: f32,
) -> Vec<u8> {
    gpu.draw(gl, w, h, time, false, brightness).unwrap();
    let mut pixels = vec![0; (w * h * 4) as usize];
    unsafe {
        gl.read_pixels(
            0,
            0,
            w,
            h,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        );
        assert_eq!(gl.get_error(), glow::NO_ERROR);
    }
    pixels
}
fn difference(a: &[u8], b: &[u8]) -> f64 {
    a.iter()
        .zip(b)
        .enumerate()
        .filter(|(i, _)| i % 4 != 3)
        .map(|(_, (&a, &b))| (a as f64 - b as f64).abs())
        .sum::<f64>()
        / (a.len() * 3 / 4) as f64
}
#[test]
#[ignore = "requires EGL surfaceless OpenGL 3.3"]
fn spatial_temporal_filter_reduces_flashes_and_resets_history() {
    let ctx = Context::new();
    let gl = &ctx.gl;
    let path = std::env::temp_dir().join(format!("nmb-stability-{}.frag", std::process::id()));
    std::fs::write(&path,r#"#version 330 core
uniform float u_time;out vec4 fragColor;
void main(){vec2 p=floor(gl_FragCoord.xy);float h=fract(sin(dot(p+floor(u_time)*17.,vec2(12.9898,78.233)))*43758.5453);float spark=step(0.965,h);fragColor=vec4(vec3(0.12)+vec3(0.88)*spark,1.);}"#).unwrap();
    let mut raw = gpu::Gpu::shader(gl, &path, false).unwrap();
    let mut stable = gpu::Gpu::shader(gl, &path, true).unwrap();
    let mut previous_raw = frame(&mut raw, gl, 128, 96, 0., 1.);
    let mut previous_stable = frame(&mut stable, gl, 128, 96, 0., 1.);
    let mut noise_raw = 0.;
    let mut noise_stable = 0.;
    for i in 1..20 {
        let a = frame(&mut raw, gl, 128, 96, i as f32, 1.);
        let b = frame(&mut stable, gl, 128, 96, i as f32, 1.);
        noise_raw += difference(&a, &previous_raw);
        noise_stable += difference(&b, &previous_stable);
        previous_raw = a;
        previous_stable = b;
    }
    println!(
        "mean pixel change: raw={:0.3}, stabilized={:0.3}",
        noise_raw / 19.,
        noise_stable / 19.
    );
    assert!(
        noise_stable < noise_raw * 0.5,
        "flashing should be materially reduced"
    );
    // A changed brightness must not inherit the previous exposure.
    let changed = frame(&mut stable, gl, 128, 96, 20., 0.5);
    let mut fresh = gpu::Gpu::shader(gl, &path, true).unwrap();
    let expected = frame(&mut fresh, gl, 128, 96, 20., 0.5);
    assert!(difference(&changed, &expected) < 0.1);
    // Reallocation must clear history and keep all pixels finite/nonblack.
    let resized = frame(&mut stable, gl, 192, 128, 21., 0.5);
    let mut fresh_size = gpu::Gpu::shader(gl, &path, true).unwrap();
    let expected = frame(&mut fresh_size, gl, 192, 128, 21., 0.5);
    assert!(difference(&resized, &expected) < 0.1);
    assert!(resized.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    raw.destroy(gl);
    stable.destroy(gl);
    fresh.destroy(gl);
    fresh_size.destroy(gl);
    std::fs::remove_file(path).unwrap();
    let scene =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders/inercia-royale-2022.frag");
    let mut raw_scene = gpu::Gpu::shader(gl, &scene, false).unwrap();
    let mut stable_scene = gpu::Gpu::shader(gl, &scene, true).unwrap();
    let mut previous_a = frame(&mut raw_scene, gl, 634, 422, 2.0, 0.75);
    let mut previous_b = frame(&mut stable_scene, gl, 634, 422, 2.0, 0.75);
    let mut raw_change = 0.;
    let mut stable_change = 0.;
    for i in 1..24 {
        let t = 2.0 + i as f32 * 0.045 / 12.0;
        let a = frame(&mut raw_scene, gl, 634, 422, t, 0.75);
        let b = frame(&mut stable_scene, gl, 634, 422, t, 0.75);
        raw_change += difference(&a, &previous_a);
        stable_change += difference(&b, &previous_b);
        previous_a = a;
        previous_b = b;
    }
    println!(
        "real scene pixel change: raw={:.3}, stabilized={:.3}",
        raw_change / 23.,
        stable_change / 23.
    );
    if let Some(dir) = std::env::var_os("NMB_CAPTURE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        image::save_buffer(
            dir.join("nmb-inercia-raw.png"),
            &previous_a,
            634,
            422,
            image::ColorType::Rgba8,
        )
        .unwrap();
        image::save_buffer(
            dir.join("nmb-inercia-stable.png"),
            &previous_b,
            634,
            422,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    raw_scene.destroy(gl);
    stable_scene.destroy(gl);
}
