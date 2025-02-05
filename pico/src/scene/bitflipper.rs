use alloc::{vec, vec::Vec};

use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

pub struct BitflipperScene {
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
        let bits = simulations::BitGrid::new(view_width as usize, view_height as usize);

        Self {
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

    fn flip_and_advance(&mut self, dir: i32) {
        if self.x <= 0 {
            self.dir_x = self.dir_x.abs() * dir;
        }

        if self.x >= self.bits.width() as i32 * self.dir_y.abs() {
            self.dir_x = -self.dir_x.abs() * dir;
        }

        if self.y <= 0 {
            self.dir_y = self.dir_y.abs() * dir;
        }

        if self.y >= self.bits.height() as i32 * self.dir_x.abs() {
            self.dir_y = -self.dir_y.abs() * dir;
        }

        self.flip_bit(dir);

        let next_x = Self::next_multiple_of_n_in_direction(self.x, self.dir_y, self.dir_x * dir);
        let next_y = Self::next_multiple_of_n_in_direction(self.y, self.dir_x, self.dir_y * dir);

        let dist_x = next_x - self.x;
        let dist_y = next_y - self.y;

        let move_amount = i32::min(dist_x.abs(), dist_y.abs());

        self.x += move_amount * dir * self.dir_x.signum();
        self.y += move_amount * dir * self.dir_y.signum();
    }

    fn next_multiple_of_n_in_direction(i: i32, n: i32, dir: i32) -> i32 {
        if dir < 0 {
            return -Self::next_multiple_of_n_in_direction(-i, -n, -dir);
        }

        i + n.abs() - Self::positive_modulo(i, n)
    }

    fn positive_modulo(i: i32, n: i32) -> i32 {
        (n.abs() + (i % n.abs())) % n.abs()
    }

    fn flip_bit(&mut self, dir: i32) {
        let x_pixel = (self.x + if self.dir_x * dir >= 0 { 0 } else { -1 }) / self.dir_y.abs();
        let y_pixel = (self.y + if self.dir_y * dir >= 0 { 0 } else { -1 }) / self.dir_x.abs();
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
            self.slopes.push(ctx.rng.gen_range(1..2048_i32));
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
                self.last_cycle_change_time_usec = ctx.time;
                self.cycle_count =
                    Self::positive_modulo(self.cycle_count + self.step_index.signum(), CYCLE_SIZE);

                self.set_slope_for_cycle_count(ctx);
            }

            self.flip_and_advance(pixel_delta.signum());
        }

        display.copy_image(&self.bits);

        // Draw some nums on the bottom bar
        if self.last_cycle_change_time_usec + 2_000_000 >= ctx.time {
            let dx = self.dir_x.abs();
            let dy = self.dir_y.abs();
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
                        display.width() as u32,
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

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case::simple_1_to_2(1, 1, 1, 2)]
    #[case::simple_2_to_4(2, 2, 2, 4)]
    #[case::simple_3_to_4(3, 2, 2, 4)]
    #[case::simple_3_to_7(3, 7, 2, 7)]
    #[case::simple_9_to_12(9, 3, 3, 12)]
    #[case::simple_9_to_5(9, 5, -1, 5)]
    #[case::simple_negative_9_to_negative_5(-9, -5, 1, -5)]
    #[case::simple_negative_9_to_negative_10(-9, -10, -1, -10)]
    fn test_next_multiple_of_n_in_direction(
        #[case] i: i32,
        #[case] n: i32,
        #[case] dir: i32,
        #[case] expected: i32,
    ) {
        assert_eq!(
            expected,
            BitflipperScene::next_multiple_of_n_in_direction(i, n, dir)
        );
    }
}
