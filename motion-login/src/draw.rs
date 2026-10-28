use crate::buffer::Buffer;
use crate::color::Color;

use std::collections::HashMap;

use rusttype::{point, Font as RustFont, Scale};
use std::sync::LazyLock;

pub static DEJAVUSANS_MONO_FONT_DATA: &[u8] = include_bytes!("../fonts/dejavu/DejaVuSansMono.ttf");
#[allow(dead_code)]
pub static ROBOTO_REGULAR_FONT_DATA: &[u8] = include_bytes!("../fonts/Roboto-Regular.ttf");

pub static DEJAVUSANS_MONO: LazyLock<RustFont<'static>> =
    LazyLock::new(|| RustFont::try_from_bytes(DEJAVUSANS_MONO_FONT_DATA).expect("embedded font"));

struct CachedGlyph {
    dimensions: (u32, u32),
    origin: (i32, i32),
    render: Vec<f32>,
}

impl CachedGlyph {
    fn new(font: &RustFont, size: f32, ch: char) -> CachedGlyph {
        let scale = Scale::uniform(size);
        let v_metrics = font.v_metrics(scale);
        let glyph = font
            .glyph(ch)
            .scaled(scale)
            .positioned(point(0.0, v_metrics.ascent));

        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            let origin = (bounding_box.min.x, bounding_box.min.y);

            let dimensions = (
                (bounding_box.max.x - bounding_box.min.x) as u32,
                (bounding_box.max.y - bounding_box.min.y) as u32,
            );
            let mut render = vec![0.0; (dimensions.0 * dimensions.1) as usize];
            glyph.draw(|x, y, o| {
                let pos = x + (y * dimensions.0);
                render[pos as usize] = o;
            });
            CachedGlyph {
                origin,
                dimensions,
                render,
            }
        } else {
            CachedGlyph {
                origin: (0, 0),
                dimensions: ((size / 4.0) as u32, 0),
                render: Vec::new(),
            }
        }
    }

    fn draw(&self, buf: &mut Buffer, pos: (i32, i32), bg: &Color, c: &Color) {
        let mut x = 0;
        let mut y = 0;
        for v in &self.render {
            let _ = buf.put(
                (
                    (x + pos.0 + self.origin.0) as u32,
                    (y + pos.1 + self.origin.1) as u32,
                ),
                &bg.blend(c, *v),
            );

            if x == self.dimensions.0 as i32 - 1 {
                y += 1;
                x = 0;
            } else {
                x += 1;
            }
        }
    }
}

pub struct Font {
    glyphs: HashMap<char, CachedGlyph>,
    font: &'static RustFont<'static>,
    size: f32,
}

impl Font {
    pub fn new(font: &'static RustFont, size: f32) -> Font {
        Font {
            glyphs: HashMap::new(),
            font,
            size,
        }
    }

    pub fn fit_width(&mut self, text: &str, max_width: u32) {
        loop {
            self.add_str_to_cache(text);
            let width: i32 = text
                .chars()
                .map(|ch| {
                    let glyph = &self.glyphs[&ch];
                    glyph.dimensions.0 as i32 + glyph.origin.0
                })
                .sum();
            if width <= max_width as i32 {
                break;
            }
            // Measure again after resizing: rasterized glyph edges round to pixels.
            self.size *= (max_width as f32 / width as f32).min(0.98);
            self.glyphs.clear();
        }
    }

    pub fn add_str_to_cache(&mut self, s: &str) {
        for ch in s.chars() {
            if !self.glyphs.contains_key(&ch) {
                let glyph = CachedGlyph::new(self.font, self.size, ch);
                self.glyphs.insert(ch, glyph);
            }
        }
    }

    pub fn draw_text(
        &self,
        buf: &mut Buffer,
        bg: &Color,
        c: &Color,
        s: &str,
    ) -> Result<(u32, u32), ::std::io::Error> {
        let mut x_off = 0;
        let mut off = 0;
        let mut glyphs = Vec::with_capacity(s.len());
        for ch in s.chars() {
            let glyph = match self.glyphs.get(&ch) {
                Some(glyph) => glyph,
                None => {
                    return Err(::std::io::Error::other(format!(
                        "glyph for {:} not in cache",
                        ch
                    )))
                }
            };
            glyphs.push(glyph);
            if glyph.origin.1 < off {
                off = glyph.origin.1
            }
        }
        for glyph in glyphs {
            glyph.draw(buf, (x_off, -off), bg, c);
            x_off += glyph.dimensions.0 as i32 + glyph.origin.0;
        }

        Ok((x_off as u32, self.size as u32))
    }

    pub fn auto_draw_text(
        &mut self,
        buf: &mut Buffer,
        bg: &Color,
        c: &Color,
        s: &str,
    ) -> Result<(u32, u32), ::std::io::Error> {
        self.add_str_to_cache(s);
        self.draw_text(buf, bg, c, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_title_fits_header_and_keeps_its_text() {
        let text = "WELCOME TO YOUR VERY OWN CYBERPUNK CITY";
        let mut font = Font::new(&DEJAVUSANS_MONO, 52.0);
        font.fit_width(text, 676);
        let mut bytes = vec![0; 676 * 65 * 4];
        let mut buf = Buffer::new(&mut bytes, (676, 65));
        let (width, _) = font
            .auto_draw_text(
                &mut buf,
                &Color::default(),
                &Color::new(1., 1., 1., 1.),
                text,
            )
            .unwrap();
        assert!(width <= 676, "title width {width}");
        assert!(font.size < 52.0);
    }
}
