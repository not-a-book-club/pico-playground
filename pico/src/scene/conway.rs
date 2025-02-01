use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use crate::oled::SH1107Display;

use super::{Context, Scene};

pub struct ConwayScene {
    sim: simulations::Life,

    view_width: u32,
    view_height: u32,
    base_y: i32,
}

impl ConwayScene {
    pub fn new<Device, DataCmdPin>(display: &crate::oled::SH1107Display<Device, DataCmdPin>) -> Self
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let sim = simulations::Life::new(display.width() as usize, display.height() as usize);
        let view_width = display.width() as u32;
        let view_height = display.height() as u32;
        let base_y = (display.height() as u32 - view_height) as i32;

        Self {
            sim,
            view_height,
            view_width,
            base_y,
        }
    }
}

impl Scene for ConwayScene {
    fn init(&mut self, ctx: &mut Context<'_>) {
        self.sim.clear_random(&mut ctx.rng);
    }

    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let mut needs_refresh = false;
        let style_text = MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On);
        let style_white_border = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            // .fill_color(BinaryColor::Off)
            .build();

        // Press B to spawn random circles and keep the sim interesting
        if ctx.btn_b {
            use rand::Rng;
            let n = 10;
            let xx: i16 = ctx.rng.gen_range(2 * n..self.sim.width()) - n;
            let yy: i16 = ctx.rng.gen_range(2 * n..self.sim.height()) - n;
            for y in (yy - n)..(yy + n) {
                for x in (xx - n)..(xx + n) {
                    let dist = (x - xx).abs() + (y - yy).abs();
                    if dist <= n && dist % 3 == 0 {
                        self.sim.set(x, y, true);
                    }
                }
            }
        }

        // let n_updated = 0;
        let n_updated = self.sim.step();
        if n_updated != 0 {
            needs_refresh = true;
        }

        // Draw!
        if needs_refresh {
            // Draw a nice title
            let text = Text::new(
                "Conway's Game of Life",
                Point::new(3, self.base_y - 3),
                style_text,
            );
            let _ = text.draw(display);

            // Draw our sim "to" the view
            for y in (self.base_y as i16 + 3)..(self.sim.height() - 3) {
                for x in 3..(self.sim.width() - 3) {
                    let is_alive = self.sim.get(x, y);
                    display.set(x, y, is_alive);
                }
            }

            // Draw border around our view
            let _ = RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(0, self.base_y),
                    Size::new(self.view_width, self.view_height),
                ),
                Size::new(5, 5),
            )
            .draw_styled(&style_white_border, display);
        }

        needs_refresh
    }
}
