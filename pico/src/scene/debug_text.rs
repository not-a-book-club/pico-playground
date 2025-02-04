use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

use alloc::string::String;

#[derive(Clone, Default)]
pub struct DebugTextScene {
    pub text: String,
    frames_since_input: u32,
}

impl DebugTextScene {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Scene for DebugTextScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let _btn_a = (self.frames_since_input > 20) && ctx.btn_a;
        let _btn_b = (self.frames_since_input > 20) && ctx.btn_b;

        let style_white_border = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .fill_color(BinaryColor::Off)
            .build();

        let _ = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(0, 0),
                Size::new(display.width() as u32, display.height() as u32),
            ),
            Size::new(5, 5),
        )
        .draw_styled(&style_white_border, display);

        let mut y = 6;
        crate::chunk_lines(&self.text, 24, |line| {
            let text = Text::new(
                line,
                Point::new(4, y),
                MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
            );
            let _ = text.draw(display);
            y += 8
        });

        true
    }
}
