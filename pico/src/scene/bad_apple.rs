#![allow(unused)]

// use embedded_graphics::mono_font::{ascii, MonoTextStyle};
// use embedded_graphics::pixelcolor::BinaryColor;
// use embedded_graphics::prelude::*;
// use embedded_graphics::primitives::*;
// use embedded_graphics::text::Text;
use simulations::BitGrid;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

// use crate::alloc::string::*;

use alloc::vec::Vec;

const fn packed_buffer() -> &'static [u8] {
    include_bytes!("../../out.bin").as_slice()
}

#[derive(Clone)]
pub struct BadAppleScene {
    frame_data: &'static [u8],
    start: u64,
    buf: BitGrid,
}

impl BadAppleScene {
    pub fn new(start: u64) -> Self {
        let frame_data = packed_buffer();
        let buf = BitGrid::new(128, 64);

        Self {
            frame_data,
            start,
            buf,
        }
    }

    pub fn next_frame(&mut self) -> Option<BitGrid> {
        if self.frame_data.len() < 4 {
            self.frame_data = packed_buffer();
            assert!(
                self.frame_data.len() > 4,
                "self.frame_data.len() = {}",
                self.frame_data.len()
            );
        }

        let size = *bytemuck::from_bytes::<u32>(&self.frame_data[..4]) as usize;
        self.frame_data = &self.frame_data[4..];

        if size == 0 {
            return None;
        }

        let bitmap = &self.frame_data[..size];
        self.frame_data = &self.frame_data[size..];

        let mut frame = BitGrid::new(85, 64);
        frame.as_mut_bytes().copy_from_slice(bitmap);

        Some(frame)
    }
}

impl Scene for BadAppleScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        if let Some(frame) = self.next_frame() {
            self.buf.clear();
            for y in 0..frame.height() {
                for x in 0..frame.width() {
                    let x_offset = (128 - 85) / 2;
                    self.buf.set(x + x_offset, y, frame.get(x, y));
                }
            }
            display.copy_image(&self.buf);
        }

        true
    }
}
