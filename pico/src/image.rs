use crate::OHNO_PINK;

use bytemuck::*;

use alloc::{vec, vec::Vec};
use core::ops::{Index, IndexMut};

#[derive(Copy, Clone, PartialEq, Eq, Pod, Zeroable, TransparentWrapper)]
#[repr(transparent)]
pub struct Rgb565([u8; 2]);

impl Rgb565 {
    pub const fn new(rgb565: u16) -> Self {
        Self(rgb565.to_be_bytes())
    }

    pub const fn from_rgb888(x: u32) -> Self {
        Self(to_rgb565(x).to_be_bytes())
    }
}

const fn to_rgb565(color: u32) -> u16 {
    #![allow(clippy::identity_op)]
    const fn scale_from_8_bits(n: u8, bits: u32) -> u16 {
        ((n as f32) / 255. * ((1 << bits) - 1) as f32) as u16
    }
    let b = scale_from_8_bits(((color >> 0) & 0xff) as u8, 5);
    let g = scale_from_8_bits(((color >> 8) & 0xff) as u8, 6);
    let r = scale_from_8_bits(((color >> 16) & 0xff) as u8, 5);

    (b << 0) | (g << 5) | (r << (5 + 6))
}

// TODO: Generic over pixel type (Rgb565)?
pub struct Image {
    buf: Vec<Rgb565>,
    width: u16,
    height: u16,
}

impl Image {
    pub fn new(width: u16, height: u16) -> Self {
        let buf = vec![OHNO_PINK; (width * height) as usize];
        Self { buf, width, height }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.buf)
    }

    fn raw_idx(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.width() && y < self.height() {
            let idx = x + y * self.width();
            Some(idx as usize)
        } else {
            None
        }
    }
}

impl Index<(u16, u16)> for Image {
    type Output = Rgb565;
    fn index(&self, (x, y): (u16, u16)) -> &Self::Output {
        if let Some(idx) = self.raw_idx(x, y) {
            &self.buf[idx]
        } else {
            panic!(
                "Out of bounds read on Image: ({x}, {y}) but image is ({w}, {h})",
                w = self.width(),
                h = self.height()
            );
        }
    }
}

impl IndexMut<(u16, u16)> for Image {
    fn index_mut(&mut self, (x, y): (u16, u16)) -> &mut Self::Output {
        if let Some(idx) = self.raw_idx(x, y) {
            &mut self.buf[idx]
        } else {
            panic!(
                "Out of bounds read+write on Image: ({x}, {y}) but image is ({w}, {h})",
                w = self.width(),
                h = self.height()
            );
        }
    }
}
