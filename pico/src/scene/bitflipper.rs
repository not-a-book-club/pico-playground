use alloc::{vec, vec::Vec};

use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use super::{Context, Scene};
use crate::oled::SH1107Display;

pub struct BitflipperScene {
    view_width: i32,
    view_height: i32,
    step_index: i32,
    t: i32,
    x: i32,
    y: i32,
    dir_x: i32,
    dir_y: i32,
    bits: simulations::BitGrid,
    cycle_count: i32,
    frames_since_input: i32,
    slopes: Vec<i32>,
}

#[rustfmt::skip]
const STEP_NUMERATORS:   [i32; 21] = [1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181];
#[rustfmt::skip]
const STEP_DENOMINATORS: [i32; 21] = [5, 3, 2, 1, 1, 1, 1, 1,  1,  1,  1,  1,  1,   1,   1,   1,   1,   1,    1,    1,    1];
const MIN_CYCLE: i32 = -(1 << 11);
const MAX_CYCLE: i32 = (1 << 11) - 1;

impl BitflipperScene {
    pub fn new<Device, DataCmdPin>(display: &crate::oled::SH1107Display<Device, DataCmdPin>) -> Self
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let view_width = display.width() as i32;
        let view_height = display.height() as i32;
        let bits = simulations::BitGrid::new(view_width as usize, view_height as usize);

        Self {
            view_height,
            view_width,
            bits,

            step_index: 6, // vroom vroom
            t: 0,
            x: 0,
            y: 0,
            // dir_x: 183,
            // dir_y: 203,
            // Only one frame?
            dir_x: 374,
            dir_y: 3895,
            cycle_count: 0,
            frames_since_input: 0,
            slopes: vec![],
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

    fn flip_and_advance(&mut self, dir: i32) {
        if dir.signum() > 0 {
            self.flip_bit()
        }

        let next_x = (((self.x + if self.dir_x * dir < 0 { -1 } else { 0 }) / self.dir_y.abs())
            + if self.dir_x * dir >= 0 { 1 } else { 0 })
            * self.dir_y.abs();
        let next_y = (((self.y + if self.dir_y * dir < 0 { -1 } else { 0 }) / self.dir_x.abs())
            + if self.dir_y * dir >= 0 { 1 } else { 0 })
            * self.dir_x.abs();

        let dist_x = next_x - self.x;
        let dist_y = next_y - self.y;

        let move_amount = i32::min(dist_x.abs(), dist_y.abs());

        self.x += move_amount * dir * self.dir_x.signum();
        self.y += move_amount * dir * self.dir_y.signum();

        if dir.signum() < 0 {
            self.flip_bit()
        }

        if self.x == 0 || self.x == self.view_width * self.dir_y.abs() {
            self.dir_x *= -1;
        }

        if self.y == 0 || self.y == self.view_height * self.dir_x.abs() {
            self.dir_y *= -1;
        }
    }

    fn flip_bit(&mut self) {
        let x_pixel = (self.x + if self.dir_x >= 0 { 0 } else { -1 }) / self.dir_y.abs();
        let y_pixel = (self.y + if self.dir_y >= 0 { 0 } else { -1 }) / self.dir_x.abs();
        self.bits.flip(x_pixel as i16, y_pixel as i16);
    }

    fn set_slope_for_cycle_count(&mut self, ctx: &mut Context<'_>) {
        let dir_x_idx: usize = if self.cycle_count >= 0 {
            (self.cycle_count * 4) as usize
        } else {
            (self.cycle_count * -4 - 2) as usize
        };

        self.fill_slope_vec_until(dir_x_idx + 1, ctx);
        self.dir_x = self.slopes[dir_x_idx];
        self.dir_y = self.slopes[dir_x_idx + 1];
    }

    fn fill_slope_vec_until(&mut self, index: usize, ctx: &mut Context<'_>) {
        use rand::Rng;
        while self.slopes.len() <= index {
            self.slopes.push(1 + (ctx.rng.gen::<i32>() % 4096));
        }
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

        if btn_a && btn_b {
            self.frames_since_input = -1;

            self.cycle_count = 0;
            self.slopes.clear();
            self.set_slope_for_cycle_count(ctx);

            self.bits.clear();
        } else if btn_a {
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
            if self.x == 0 && self.y == 0 {
                self.cycle_count += self.step_index.signum();

                if self.step_index > 0 && self.cycle_count == MAX_CYCLE {
                    self.cycle_count = MIN_CYCLE + 1
                }
                if self.step_index < 0 && self.cycle_count == MIN_CYCLE {
                    self.cycle_count = MAX_CYCLE - 1
                }

                self.set_slope_for_cycle_count(ctx);
            }

            self.flip_and_advance(pixel_delta.signum());
        }

        display.copy_image(&self.bits);

        // Draw some nums on the bottom bar
        {
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
                        self.view_width as u32,
                        self.view_height as u32 - base_y as u32,
                    ),
                ),
                Size::new(5, 5),
            )
            .draw_styled(&style_white_border, display);

            let line = alloc::format!(
                "({dx},{dy}), {slopes}",
                dx = self.dir_x.abs(),
                dy = self.dir_y.abs(),
                slopes = self.slopes.len()
            );
            let text = Text::new(
                &line,
                Point::new(4, base_y + 9),
                MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
            );
            let _ = text.draw(display);
        }

        true
    }
}
