use image_tools::VideoDecoder;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

const fn packed_buffer() -> &'static [u8] {
    include_bytes!("../../bad-apple.video").as_slice()
}

pub struct BadAppleScene {
    decoder: VideoDecoder<'static>,
}

impl BadAppleScene {
    pub fn new(_start: u64) -> Self {
        // TODO: Could drive playback with `start` time but incrementing per frame works so far
        let decoder = VideoDecoder::new(packed_buffer());

        Self { decoder }
    }
}

impl Scene for BadAppleScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        _ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        if self.decoder.is_finished() {
            self.decoder.reset();
        }

        if let Some(frame) = self.decoder.next_frame() {
            if frame.background_set {
                display.clear_set();
            } else {
                display.clear_unset();
            }

            let x_offset = (128 - frame.bitmap.width()) / 2;

            for y in 0..frame.bitmap.height() {
                for x in 0..frame.bitmap.width() {
                    display.set(x + x_offset, y, frame.bitmap.get(x, y));
                }
            }
        }

        true
    }
}
