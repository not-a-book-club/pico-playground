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

pub struct Image<Pixel = Rgb565> {
    buf: Vec<Pixel>,
    width: u16,
    height: u16,
}

#[allow(dead_code)]
impl<Pixel> Image<Pixel>
where
    Pixel: Pod,
{
    pub fn new(width: u16, height: u16) -> Self {
        let buf = vec![Pixel::zeroed(); (width * height) as usize];
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

    pub fn fill(&mut self, color: Pixel) {
        self.buf.fill(color);
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

impl<Pixel> Index<(u16, u16)> for Image<Pixel>
where
    Pixel: Pod,
{
    type Output = Pixel;
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

impl<Pixel> IndexMut<(u16, u16)> for Image<Pixel>
where
    Pixel: Pod,
{
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
