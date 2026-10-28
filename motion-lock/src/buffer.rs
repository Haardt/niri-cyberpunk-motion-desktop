use crate::color::Color;

pub struct Buffer<'a> {
    buf: &'a mut [u8],
    dimensions: (u32, u32),
    subdimensions: Option<(u32, u32, u32, u32)>,
}

impl<'a> Buffer<'a> {
    pub fn new(buf: &'a mut [u8], dimensions: (u32, u32)) -> Buffer<'a> {
        Buffer {
            buf,
            dimensions,
            subdimensions: None,
        }
    }

    pub fn get_bounds(&self) -> (u32, u32, u32, u32) {
        if let Some(subdim) = self.subdimensions {
            subdim
        } else {
            (0, 0, self.dimensions.0, self.dimensions.1)
        }
    }

    pub fn get_signed_bounds(&self) -> (i32, i32, i32, i32) {
        if let Some(subdim) = self.subdimensions {
            (
                subdim.0 as i32,
                subdim.1 as i32,
                subdim.2 as i32,
                subdim.3 as i32,
            )
        } else {
            (0, 0, self.dimensions.0 as i32, self.dimensions.1 as i32)
        }
    }

    pub fn subdimensions(
        &mut self,
        subdimensions: (u32, u32, u32, u32),
    ) -> Result<Buffer<'_>, ::std::io::Error> {
        let bounds = self.get_bounds();
        if subdimensions.0 + subdimensions.2 > bounds.2
            || subdimensions.1 + subdimensions.3 > bounds.3
        {
            return Err(::std::io::Error::other(format!(
                "cannot create subdimensions larger than buffer: {:?} > {:?}",
                subdimensions, bounds
            )));
        }

        Ok(Buffer {
            buf: self.buf,
            dimensions: self.dimensions,
            subdimensions: Some((
                subdimensions.0 + bounds.0,
                subdimensions.1 + bounds.1,
                subdimensions.2,
                subdimensions.3,
            )),
        })
    }

    pub fn memset(&mut self, color: &Color) {
        let (x, y, w, h) = self.get_bounds();
        let bytes = color.as_argb8888().to_ne_bytes();
        for row in y..y + h {
            let start = ((row * self.dimensions.0 + x) * 4) as usize;
            for pixel in self.buf[start..start + (w * 4) as usize]
                .as_chunks_mut::<4>()
                .0
            {
                pixel.copy_from_slice(&bytes);
            }
        }
    }

    pub fn put(&mut self, pos: (u32, u32), c: &Color) -> Result<(), ::std::io::Error> {
        let true_pos = if let Some(subdim) = self.subdimensions {
            if pos.0 >= subdim.2 || pos.1 >= subdim.3 {
                return Err(::std::io::Error::other(format!(
                    "put({:?}) is not within subdimensions of buffer ({:?})",
                    pos, subdim
                )));
            }
            (pos.0 + subdim.0, pos.1 + subdim.1)
        } else {
            if pos.0 >= self.dimensions.0 || pos.1 >= self.dimensions.1 {
                return Err(::std::io::Error::other(format!(
                    "put({:?}) is not within dimensions of buffer ({:?})",
                    pos, self.dimensions
                )));
            }
            pos
        };

        let offset = 4 * (true_pos.0 + true_pos.1 * self.dimensions.0) as usize;
        self.buf[offset..offset + 4].copy_from_slice(&c.as_argb8888().to_ne_bytes());

        Ok(())
    }
}
