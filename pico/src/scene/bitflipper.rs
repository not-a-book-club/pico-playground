use alloc::{vec, vec::Vec};

use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

pub struct BitflipperScene {
    bit_flipper: simulations::BitFlipper,
    step_index: i32,
    t: i32,
    cycle_count: i32,
    frames_since_input: i32,
    slopes: Vec<i32>,
    last_cycle_change_time_usec: u64,
}

#[rustfmt::skip]
const STEP_NUMERATORS:   [i32; 24] = [1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181, 4181 << 1, 4181 << 2, 4181 << 3];
#[rustfmt::skip]
const STEP_DENOMINATORS: [i32; 24] = [5, 3, 2, 1, 1, 1, 1, 1,  1,  1,  1,  1,  1,   1,   1,   1,   1,   1,    1,    1,    1,         1,         1,         1];
const CYCLE_SIZE: i32 = 1 << 11;

impl BitflipperScene {
    pub fn new<Device, DataCmdPin>(display: &SH1107Display<Device, DataCmdPin>) -> Self
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let view_width = display.width() as i32;
        let view_height = display.height() as i32;
        let bit_flipper = simulations::BitFlipper::new(view_width, view_height, 0, 0); // will be thrown away immediately

        Self {
            bit_flipper: bit_flipper,
            step_index: 6, // vroom vroom
            t: 0,
            cycle_count: 0,
            frames_since_input: 0,
            slopes: vec![],
            last_cycle_change_time_usec: 0,
        }
    }

    fn current_step_count(&mut self) -> i32 {
        if self.step_index == 0 {
            return 0;
        }

        10920 * STEP_NUMERATORS[self.step_index.unsigned_abs() as usize - 1]
            / STEP_DENOMINATORS[self.step_index.unsigned_abs() as usize - 1]
            * self.step_index.signum()
    }

    fn slope_for_cycle_count(&mut self, ctx: &mut Context<'_>) -> (i32, i32) {
        let dir_x_idx: usize = if self.cycle_count < CYCLE_SIZE / 2 {
            (self.cycle_count * 4) as usize
        } else {
            ((CYCLE_SIZE - self.cycle_count) * 4 - 2) as usize
        };

        self.fill_slope_vec_until(dir_x_idx + 1, ctx);
        (self.slopes[dir_x_idx], self.slopes[dir_x_idx + 1])
    }

    fn fill_slope_vec_until(&mut self, index: usize, ctx: &mut Context<'_>) {
        use rand::Rng;
        while self.slopes.len() <= index {
            self.slopes.push(ctx.rng.random_range(1..2048_i32));
        }
    }

    fn positive_modulo(i: i32, n: i32) -> i32 {
        (n.abs() + (i % n.abs())) % n.abs()
    }
}

impl Scene for BitflipperScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let btn_a = (self.frames_since_input > 20) && ctx.btn_a;
        let btn_b = (self.frames_since_input > 20) && ctx.btn_b;

        if btn_a || btn_b {
            // When we tap a button, show the slopes dialog again. briefly.
            self.last_cycle_change_time_usec = ctx.time;
        }

        if btn_a {
            self.frames_since_input = -1;
            if self.step_index > 0 || self.step_index.abs() < STEP_NUMERATORS.len() as i32 {
                self.step_index -= 1;
            }
        } else if btn_b {
            self.frames_since_input = -1;
            if self.step_index < STEP_NUMERATORS.len() as i32 {
                self.step_index += 1;
            }
        }

        self.frames_since_input = self.frames_since_input.saturating_add(1);

        self.t += self.current_step_count();
        let pixel_delta = self.t / 10920;
        self.t -= pixel_delta * 10920;

        for _ in 0..pixel_delta.abs() {
            if self.bit_flipper.x == 0 && self.bit_flipper.y == 0 {
                self.last_cycle_change_time_usec = ctx.time;
                self.cycle_count =
                    Self::positive_modulo(self.cycle_count + self.step_index.signum(), CYCLE_SIZE);

                let slope = self.slope_for_cycle_count(ctx);
                self.bit_flipper = simulations::BitFlipper::new(
                    self.bit_flipper.bits.width() as _,
                    self.bit_flipper.bits.height() as _,
                    slope.0,
                    slope.1,
                )
            }

            self.bit_flipper.flip_and_advance(pixel_delta.signum());
        }

        display.copy_image(&self.bit_flipper.bits);

        // Draw some nums on the bottom bar
        if self.last_cycle_change_time_usec + 2_000_000 >= ctx.time {
            let dx = self.slope_for_cycle_count(ctx).0;
            let dy = self.slope_for_cycle_count(ctx).1;
            let line = alloc::format!("({dx}, {dy})");

            let base_y = 48;
            let style_white_border = PrimitiveStyleBuilder::new()
                .stroke_width(1)
                .stroke_color(BinaryColor::On)
                .fill_color(BinaryColor::Off)
                .build();

            let _ = RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(0, base_y),
                    Size::new(
                        5 + 5 * line.len() as u32,
                        display.height() as u32 - base_y as u32,
                    ),
                ),
                Size::new(5, 5),
            )
            .draw_styled(&style_white_border, display);

            // Make sure to draw the text ONTOP of the rectangle
            let style = MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On);
            let text = Text::new(&line, Point::new(4, base_y + 9), style);
            let _ = text.draw(display);
        }

        true
    }
}
