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
                [0x3057, 1280, 0x3056, 800, 0x3038].as_ptr(),
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
#[ignore = "requires surfaceless EGL/OpenGL 3.3; optionally writes previews to MOTION_PREVIEW_DIR"]
fn concept_quality_preview() {
    let ctx = Context::new();
    let names = [
        "almost-touching",
        "threshold-of-self",
        "disco-slowmo",
        "retro-boing",
        "oneiric-silk",
        "silk-continuum",
        "mandelbrot-prism-flight",
        "algebra-of-love",
        "world-execute",
        "autumn-koi-pond",
        "tangent-garden",
        "memory-of-you",
        "strange-simulation",
    ];
    for name in names {
        if let Ok(selected) = std::env::var("MOTION_PREVIEW_SCENES") {
            if !selected.split(',').any(|requested| requested == name) {
                continue;
            }
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("shaders/{name}.frag"));
        let mut scene =
            gpu::Gpu::shader(&ctx.gl, &path, name == "mandelbrot-prism-flight").unwrap();
        let times: &[f32] = if name == "mandelbrot-prism-flight" {
            &[0.0, 35.0, 75.0, 143.0, 210.0, 280.0]
        } else if name == "algebra-of-love" {
            &[0.0, 18.0, 36.0, 54.0, 72.0, 78.0]
        } else {
            &[0.0, 5.0, 20.0, 35.0, 65.0]
        };
        for &time in times {
            if name == "mandelbrot-prism-flight" {
                for _ in 0..5 {
                    scene.draw(&ctx.gl, 1280, 800, time, false, 1.0).unwrap();
                }
            }
            let a = frame(&mut scene, &ctx.gl, 1280, 800, time, 1.0);
            assert!(a.chunks_exact(4).all(|p| p[3] == 255), "{name}: alpha");
            if name == "mandelbrot-prism-flight" {
                let light = |x: usize, y: usize| {
                    let i = (y * 1280 + x) * 4;
                    (54 * a[i] as u32 + 183 * a[i + 1] as u32 + 19 * a[i + 2] as u32) / 256
                };
                let mut isolated_dark = 0;
                for y in 1..799 {
                    for x in 1..1279 {
                        if light(x, y) >= 26 {
                            continue;
                        }
                        let mut neighbors = 0;
                        for yy in y - 1..=y + 1 {
                            for xx in x - 1..=x + 1 {
                                if xx != x || yy != y {
                                    neighbors += light(xx, yy);
                                }
                            }
                        }
                        if neighbors > 8 * 92 {
                            isolated_dark += 1;
                        }
                    }
                }
                println!("{name} at {time}s: {isolated_dark} isolated dark pixels");
                assert!(
                    isolated_dark <= 4,
                    "isolated black-pixel sparkle regression: {isolated_dark}"
                );
            }

            if name == "silk-continuum" {
                // Prevent the fullscreen environment regressing into a lit object
                // surrounded by a mostly empty, dark background.
                let lit = a
                    .chunks_exact(4)
                    .filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 72)
                    .count();
                let coverage = lit as f64 / (1280.0 * 800.0);
                assert!(coverage > 0.97, "fullscreen field coverage: {coverage}");
            }

            let min = a
                .chunks_exact(4)
                .map(|p| p[0] as u32 + p[1] as u32 + p[2] as u32)
                .min()
                .unwrap();
            let max = a
                .chunks_exact(4)
                .map(|p| p[0] as u32 + p[1] as u32 + p[2] as u32)
                .max()
                .unwrap();
            assert!(max - min > 65, "{name}: insufficient contrast");
            let mut img = image::RgbaImage::from_raw(1280, 800, a).unwrap();
            image::imageops::flip_vertical_in_place(&mut img);
            if let Some(folder) = std::env::var_os("MOTION_PREVIEW_DIR") {
                let folder = std::path::PathBuf::from(folder);
                std::fs::create_dir_all(&folder).unwrap();
                img.save(folder.join(format!("{name}-{time:.0}-preview.png")))
                    .unwrap();
            }
        }
        let motion_time = if name == "mandelbrot-prism-flight" {
            143.0
        } else {
            20.0
        };
        let a = frame(&mut scene, &ctx.gl, 640, 400, motion_time, 1.0);
        let b = frame(&mut scene, &ctx.gl, 640, 400, motion_time + 0.04, 1.0);
        let c = frame(&mut scene, &ctx.gl, 640, 400, motion_time + 5.0, 1.0);
        assert!(difference(&a, &c) > 0.02, "{name}: no motion");
        assert!(difference(&a, &b) < 4.0, "{name}: abrupt frame transition");
        let mut previous = b.clone();
        for i in 2..26 {
            let next = frame(
                &mut scene,
                &ctx.gl,
                640,
                400,
                motion_time + i as f32 * 0.04,
                1.0,
            );
            assert!(
                difference(&previous, &next) < 4.0,
                "{name}: flash in consecutive frames"
            );
            previous = next;
        }
        println!(
            "{name}: short delta {:.3}; 5s delta {:.3}",
            difference(&a, &b),
            difference(&a, &c)
        );
        let bench = std::time::Instant::now();
        for i in 0..16 {
            scene
                .draw(&ctx.gl, 640, 400, 20.0 + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
        }
        println!(
            "{name}: {:.2}ms/frame at 640x400",
            bench.elapsed().as_secs_f64() * 1000.0 / 16.0
        );
        let mut stable = gpu::Gpu::shader(&ctx.gl, &path, true).unwrap();
        stable.draw(&ctx.gl, 400, 640, 35.0, true, 0.65).unwrap();
        stable.draw(&ctx.gl, 400, 640, 35.04, true, 0.65).unwrap();
        if let Some(folder) = std::env::var_os("MOTION_PREVIEW_DIR") {
            let mut pixels = vec![0u8; 400 * 640 * 4];
            unsafe {
                ctx.gl.read_pixels(
                    0,
                    0,
                    400,
                    640,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut pixels)),
                );
            }
            assert!(
                pixels.chunks_exact(4).all(|p| p[3] == 255),
                "{name}: night alpha"
            );
            let mut img = image::RgbaImage::from_raw(400, 640, pixels).unwrap();
            image::imageops::flip_vertical_in_place(&mut img);
            img.save(std::path::PathBuf::from(folder).join(format!("{name}-portrait-night.png")))
                .unwrap();
        }

        unsafe {
            assert_eq!(ctx.gl.get_error(), glow::NO_ERROR);
        }
        stable.destroy(&ctx.gl);
        scene.destroy(&ctx.gl);
    }
}

#[test]
#[ignore = "requires a real EGL/OpenGL GPU; reports chapter render costs"]
fn algebra_chapter_render_costs() {
    let ctx = Context::new();
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders/algebra-of-love.frag");
    let mut scene = gpu::Gpu::shader(&ctx.gl, &path, false).unwrap();
    for time in [0.0, 18.0, 36.0, 54.0, 72.0] {
        for i in 0..2 {
            scene
                .draw(&ctx.gl, 922, 614, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
        }
        let start = std::time::Instant::now();
        for i in 0..8 {
            scene
                .draw(&ctx.gl, 922, 614, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
            assert_eq!(ctx.gl.get_error(), glow::NO_ERROR);
        }
        println!(
            "chapter at {time}s: {:.2} ms/frame at 922x614",
            start.elapsed().as_secs_f64() * 1000.0 / 8.0
        );
    }
    scene.destroy(&ctx.gl);
}

#[test]
#[ignore = "requires EGL/OpenGL; checks visible copper-bar travel independently of the ball"]
fn copper_bars_visibly_travel() {
    let ctx = Context::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders/retro-boing.frag");
    let mut scene = gpu::Gpu::shader(&ctx.gl, &path, false).unwrap();
    let centroid = |pixels: &[u8]| {
        let (mut weight, mut position) = (0.0, 0.0);
        // Left upper background: no ball, floor, bezel or moving shadow.
        for y in 400..780 {
            for x in 80..180 {
                let i = (y * 1280 + x) * 4;
                let green =
                    (2.0 * pixels[i + 1] as f64 - pixels[i] as f64 - pixels[i + 2] as f64).max(0.0);
                weight += green;
                position += green * y as f64;
            }
        }
        assert!(weight > 1000.0, "green copper bars must be visible");
        position / weight
    };
    let a = frame(&mut scene, &ctx.gl, 1280, 800, 0.0, 1.0);
    let b = frame(&mut scene, &ctx.gl, 1280, 800, 2.4, 1.0);
    let travel = (centroid(&a) - centroid(&b)).abs();
    println!("copper stack travels {travel:.1} pixels in 4.8 real seconds at configured speed");
    assert!(
        travel > 18.0,
        "copper movement must be clearly visible, measured {travel}"
    );
    scene.destroy(&ctx.gl);
}

#[test]
#[ignore = "requires EGL/OpenGL; reports the fullscreen environment at a native wallpaper buffer size"]
fn silk_continuum_render_costs() {
    let ctx = Context::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders/silk-continuum.frag");
    let mut scene = gpu::Gpu::shader(&ctx.gl, &path, false).unwrap();
    for time in [0.0, 20.0, 65.0] {
        for i in 0..2 {
            scene
                .draw(&ctx.gl, 1152, 768, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
        }
        let start = std::time::Instant::now();
        for i in 0..8 {
            scene
                .draw(&ctx.gl, 1152, 768, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
            assert_eq!(ctx.gl.get_error(), glow::NO_ERROR);
        }
        println!(
            "silk-continuum at {time}s: {:.2} ms/frame at 1152x768",
            start.elapsed().as_secs_f64() * 1000.0 / 8.0
        );
    }
    scene.destroy(&ctx.gl);
}

#[test]
#[ignore = "requires EGL/OpenGL; compares high-precision Mandelbrot membership to f64 and profiles flight phases"]
fn mandelbrot_precision_and_costs() {
    let ctx = Context::new();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("shaders/mandelbrot-prism-flight.frag");
    let code = std::fs::read_to_string(&source).unwrap();
    let prefix = code.split("vec3 studio(").next().unwrap();
    let debug = format!("{prefix}\nvoid main(){{vec2 p=gl_FragCoord.xy/vec2(32,32)-0.5;p.x*=1.5;vec2 cx=coordinate(vec2(-0.7436438798904419,-7.146709113392546e-9),p.x*u_time);vec2 cy=coordinate(vec2(0.1318259090185165,-4.813186543772829e-9),p.y*u_time);Fractal f=mandelbrot(cx,cy);fragColor=vec4(f.inside,0,0,1);}}\n");
    let path =
        std::env::temp_dir().join(format!("nmb-mandel-precision-{}.frag", std::process::id()));
    std::fs::write(&path, debug).unwrap();
    let mut probe = gpu::Gpu::shader(&ctx.gl, &path, false).unwrap();
    for span in [1.0_f32 / 1024.0, 1.0_f32 / 131072.0] {
        let pixels = frame(&mut probe, &ctx.gl, 32, 32, span, 1.0);
        let mut different = 0;
        for y in 0..32 {
            for x in 0..32 {
                let cr =
                    -0.743643887037151_f64 + ((x as f64 + 0.5) / 32.0 - 0.5) * 1.5 * span as f64;
                let ci = 0.131825904205330_f64 + ((y as f64 + 0.5) / 32.0 - 0.5) * span as f64;
                let (mut zr, mut zi, mut escaped) = (0.0_f64, 0.0_f64, false);
                for _ in 0..420 {
                    if zr * zr + zi * zi > 256.0 {
                        escaped = true;
                        break;
                    }
                    let next = zr * zr - zi * zi + cr;
                    zi = 2.0 * zr * zi + ci;
                    zr = next;
                }
                let inside_gpu = pixels[(y * 32 + x) * 4] > 127;
                if inside_gpu == escaped {
                    different += 1;
                }
            }
        }
        println!("Mandelbrot span {span}: {different}/1024 GPU/f64 classification differences");
        assert!(
            different <= 10,
            "Mandelbrot precision regression: {different} differences"
        );
    }
    probe.destroy(&ctx.gl);
    std::fs::remove_file(path).unwrap();
    let mut scene = gpu::Gpu::shader(&ctx.gl, &source, false).unwrap();
    for time in [35.0, 75.0, 143.0, 210.0] {
        for i in 0..2 {
            scene
                .draw(&ctx.gl, 1152, 768, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
        }
        let start = std::time::Instant::now();
        for i in 0..8 {
            scene
                .draw(&ctx.gl, 1152, 768, time + i as f32 * 0.04, false, 1.0)
                .unwrap();
        }
        unsafe {
            ctx.gl.finish();
            assert_eq!(ctx.gl.get_error(), glow::NO_ERROR);
        }
        println!(
            "Mandelbrot flight at {time}s: {:.2} ms/frame at 1152x768",
            start.elapsed().as_secs_f64() * 1000.0 / 8.0
        );
    }
    scene.destroy(&ctx.gl);
}

#[test]
#[ignore = "requires EGL/OpenGL; compares multi-second 12 FPS sequences across the flight"]
fn mandelbrot_motion_stability() {
    let ctx = Context::new();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("shaders/mandelbrot-prism-flight.frag");
    let mut cases = Vec::new();
    if let Some(reference) = std::env::var_os("MOTION_FLICKER_REFERENCE") {
        cases.push((
            "previous-with-history",
            std::path::PathBuf::from(reference),
            true,
        ));
    }
    cases.push(("fixed-spatial-with-history", source, true));
    for (label, path, stable) in cases {
        let mut scene = gpu::Gpu::shader(&ctx.gl, &path, stable).unwrap();
        for start in [35.0_f32, 75.0, 143.0, 210.0] {
            let dt = 0.6 / 12.0;
            for i in 0..8 {
                scene
                    .draw(&ctx.gl, 640, 400, start - (8 - i) as f32 * dt, false, 1.0)
                    .unwrap();
            }
            let (mut previous, mut before) = (Vec::<f32>::new(), Vec::<f32>::new());
            let (mut acceleration, mut pulses, mut samples) = (0.0_f64, 0_usize, 0_usize);
            let (mut previous_mean, mut max_global_step) = (None::<f64>, 0.0_f64);
            for i in 0..50 {
                let pixels = frame(&mut scene, &ctx.gl, 640, 400, start + i as f32 * dt, 1.0);
                let current: Vec<f32> = pixels
                    .chunks_exact(4)
                    .map(|p| {
                        (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
                    })
                    .collect();
                let mean = current.iter().map(|&x| x as f64).sum::<f64>() / current.len() as f64;
                if let Some(prev) = previous_mean {
                    max_global_step = max_global_step.max((mean - prev).abs());
                }
                previous_mean = Some(mean);
                if !before.is_empty() {
                    for ((&a, &b), &c) in before.iter().zip(&previous).zip(&current) {
                        let left = b - a;
                        let right = c - b;
                        acceleration += (right - left).abs() as f64;
                        if left * right < 0.0 && left.abs().min(right.abs()) > 0.12 {
                            pulses += 1;
                        }
                        samples += 1;
                    }
                }
                before = previous;
                previous = current;
            }
            println!("{label} at {start}s: temporal acceleration {:.5}, one-frame pulses {:.4}% ({pulses} events), max mean step {:.5}",acceleration/samples as f64,pulses as f64*100.0/samples as f64,max_global_step);
            if label == "fixed-spatial-with-history" {
                assert!(pulses <= 12, "isolated frame pulses: {pulses}");
                assert!(
                    max_global_step < 0.003,
                    "whole-frame brightness jump: {max_global_step}"
                );
            }
        }
        scene.destroy(&ctx.gl);
    }
}

#[test]
fn mandelbrot_keeps_pixel_grid_across_power_changes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.toml");
    let mut config = motion_render::config::Config::load(&path).unwrap();
    config.active = "mandelbrot-prism-flight".into();
    assert_eq!(
        config.effective_scale(false),
        config.effective_scale(true),
        "power changes must not resize this high-detail shader"
    );
}
