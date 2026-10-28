//! Event-driven raster UI: no animation timer, external assets or background polling.
use crate::{
    buffer::Buffer,
    color::Color,
    draw::{Font, DEJAVUSANS_MONO},
};
use std::io;

pub struct Access<'a> {
    pub question: &'a str,
    pub answer: &'a str,
    pub secret: bool,
    pub error: &'a str,
    pub preview: bool,
    pub waiting: bool,
    pub fingerprint: bool,
    pub cancelling: bool,
}
pub struct Interface {
    title: Font,
    title_text: String,
    body: Font,
    small: Font,
    label: Font,
}
impl Interface {
    pub fn new() -> Self {
        Self {
            title: Font::new(&DEJAVUSANS_MONO, 52.0),
            title_text: String::new(),
            body: Font::new(&DEJAVUSANS_MONO, 28.0),
            small: Font::new(&DEJAVUSANS_MONO, 16.0),
            label: Font::new(&DEJAVUSANS_MONO, 20.0),
        }
    }
    pub fn draw(
        &mut self,
        buf: &mut Buffer,
        config: &crate::config::Config,
        access: Access<'_>,
    ) -> io::Result<()> {
        if self.title_text != config.title {
            self.title = Font::new(&DEJAVUSANS_MONO, 52.0);
            self.title.fit_width(&config.title, 676);
            self.title_text = config.title.clone();
        }
        let panel = &config.background;
        let yellow = &config.headline;
        let cyan = &config.prompt;
        let red = &config.prompt_err;
        let Access {
            question,
            answer,
            secret,
            error,
            preview,
            waiting,
            fingerprint,
            cancelling,
        } = access;
        let ink = Color::new(0.025, 0.035, 0.045, 1.0);
        let field = Color::new(0.055, 0.085, 0.10, 1.0);
        let muted = Color::new(0.46, 0.57, 0.60, 1.0);
        let white = Color::new(0.88, 0.94, 0.92, 1.0);
        buf.memset(&Color::new(0.0, 0.0, 0.0, 0.0));
        // Deliberately asymmetric silhouette and hard circuit-like cuts.
        polygon(
            buf,
            &[(20, 28), (738, 28), (780, 70), (780, 151), (20, 151)],
            yellow,
        )?;
        polygon(
            buf,
            &[(20, 165), (780, 165), (780, 480), (748, 512), (20, 512)],
            cyan,
        )?;
        polygon(
            buf,
            &[(22, 167), (778, 167), (778, 479), (747, 510), (22, 510)],
            panel,
        )?;
        rect(buf, 20, 165, 6, 230, yellow)?;
        rect(buf, 20, 416, 6, 63, red)?;
        text(
            &mut self.small,
            buf,
            (46, 43, 650, 25),
            yellow,
            &ink,
            "NIGHT CITY   /   TERMINAL 01",
        )?;
        text(
            &mut self.title,
            buf,
            (44, 71, 676, 65),
            yellow,
            &ink,
            &config.title,
        )?;
        // Registration marks, serial rail and simple data blocks.
        rect(buf, 734, 102, 25, 4, &ink)?;
        rect(buf, 745, 91, 4, 25, &ink)?;
        text(
            &mut self.small,
            buf,
            (44, 183, 510, 24),
            panel,
            cyan,
            "NEURAL LINK  /  LOCAL ACCESS",
        )?;
        text(
            &mut self.small,
            buf,
            (624, 183, 130, 24),
            panel,
            yellow,
            "NIRI // 01",
        )?;
        rect(buf, 44, 219, 710, 1, &muted)?;
        let label = if fingerprint {
            "02 / FINGERPRINT"
        } else if waiting {
            "02 / SIGN IN"
        } else if secret {
            "02 / AUTHENTICATION"
        } else {
            "01 / USER ID"
        };
        text(
            &mut self.label,
            buf,
            (44, 239, 710, 28),
            panel,
            yellow,
            label,
        )?;
        // Actual PAM challenge remains visible, including multifactor prompts.
        text(
            &mut self.small,
            buf,
            (44, 276, 710, 24),
            panel,
            &muted,
            &limit(question, 66),
        )?;
        polygon(
            buf,
            &[(44, 309), (754, 309), (754, 356), (741, 369), (44, 369)],
            &muted,
        )?;
        polygon(
            buf,
            &[(46, 311), (752, 311), (752, 355), (740, 367), (46, 367)],
            &field,
        )?;
        rect(buf, 46, 311, 3, 56, cyan)?;
        text(&mut self.body, buf, (63, 327, 34, 34), &field, cyan, ">")?;
        let value = if cancelling {
            "CANCELLING ...".into()
        } else if waiting {
            if fingerprint {
                "SCAN YOUR FINGER".into()
            } else {
                "PLEASE WAIT ...".into()
            }
        } else if answer.is_empty() {
            if secret {
                "ENTER PASSWORD".into()
            } else {
                "YOUR USERNAME".into()
            }
        } else {
            let value = if secret {
                "*".repeat(answer.chars().count())
            } else {
                answer.to_owned()
            };
            let end: String = value
                .chars()
                .rev()
                .take(35)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!("{end}_")
        };
        text(
            &mut self.body,
            buf,
            (94, 327, 636, 34),
            &field,
            if answer.is_empty() { &muted } else { &white },
            &value,
        )?;
        polygon(
            buf,
            &[(44, 393), (379, 393), (395, 409), (395, 443), (44, 443)],
            yellow,
        )?;
        text(
            &mut self.label,
            buf,
            (63, 409, 314, 28),
            yellow,
            &ink,
            if waiting {
                "CANCEL          X"
            } else {
                "SIGN IN         >>"
            },
        )?;
        text(
            &mut self.small,
            buf,
            (423, 401, 324, 23),
            panel,
            cyan,
            if waiting {
                "[ ESC ]    CANCEL"
            } else {
                "[ ENTER ]  CONFIRM"
            },
        )?;
        text(
            &mut self.small,
            buf,
            (423, 425, 324, 23),
            panel,
            &muted,
            "[ CTRL+U ] RESET",
        )?;
        if error.is_empty() {
            text(
                &mut self.small,
                buf,
                (44, 470, 655, 23),
                panel,
                &muted,
                if preview {
                    "PREVIEW // SIGN IN DISABLED"
                } else if fingerprint {
                    "PASSWORD PROMPT FOLLOWS ON TIMEOUT"
                } else if waiting {
                    "CHECKING AUTHENTICATION"
                } else {
                    "READY // ENTER YOUR USERNAME"
                },
            )?;
        } else {
            text(
                &mut self.small,
                buf,
                (44, 470, 655, 23),
                panel,
                red,
                &limit(error, 66),
            )?;
        }
        // Circuit tails and deterministic barcode, static to avoid UI flicker.
        rect(buf, 44, 527, 470, 2, cyan)?;
        for i in 0..31 {
            let width = 1 + (i * 7 % 3);
            rect(
                buf,
                550 + i * 6,
                525,
                width,
                15,
                if i % 5 == 0 { red } else { cyan },
            )?;
        }
        text(
            &mut self.small,
            buf,
            (44, 542, 490, 23),
            &Color::new(0.0, 0.0, 0.0, 0.0),
            cyan,
            "MOTION / ACCESS SYSTEM              2077",
        )?;
        Ok(())
    }
}
fn limit(s: &str, max: usize) -> String {
    let clean: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if clean.chars().count() <= max {
        clean
    } else {
        format!("{}...", clean.chars().take(max - 3).collect::<String>())
    }
}
fn text(
    font: &mut Font,
    buf: &mut Buffer,
    bounds: (u32, u32, u32, u32),
    bg: &Color,
    color: &Color,
    value: &str,
) -> io::Result<()> {
    font.auto_draw_text(&mut buf.subdimensions(bounds)?, bg, color, value)?;
    Ok(())
}
fn rect(buf: &mut Buffer, x: u32, y: u32, w: u32, h: u32, color: &Color) -> io::Result<()> {
    buf.subdimensions((x, y, w, h))?.memset(color);
    Ok(())
}
fn polygon(buf: &mut Buffer, vertices: &[(u32, u32)], color: &Color) -> io::Result<()> {
    let min = vertices.iter().map(|v| v.1).min().unwrap_or(0);
    let max = vertices.iter().map(|v| v.1).max().unwrap_or(0);
    for y in min..max {
        let scan = y as f32 + 0.5;
        let mut intersections = Vec::with_capacity(vertices.len());
        for i in 0..vertices.len() {
            let a = vertices[i];
            let b = vertices[(i + 1) % vertices.len()];
            if (a.1 as f32 <= scan && b.1 as f32 > scan)
                || (b.1 as f32 <= scan && a.1 as f32 > scan)
            {
                intersections.push(
                    a.0 as f32
                        + (scan - a.1 as f32) * (b.0 as f32 - a.0 as f32)
                            / (b.1 as f32 - a.1 as f32),
                );
            }
        }
        intersections.sort_by(f32::total_cmp);
        for pair in intersections.as_chunks::<2>().0 {
            let x = pair[0].ceil() as u32;
            let end = pair[1].ceil() as u32;
            if end > x {
                rect(buf, x, y, end - x, 1, color)?;
            }
        }
    }
    Ok(())
}
