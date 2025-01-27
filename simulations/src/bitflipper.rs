#![allow(clippy::identity_op, clippy::collapsible_else_if)]

use crate::BitGrid;

pub struct BitFlipper {
    pub view_width: i32,
    pub view_height: i32,
    pub step_index: isize,
    pub t: i32,
    pub x: i32,
    pub y: i32,
    pub dir_x: i32,
    pub dir_y: i32,
    pub bits: BitGrid,
}

#[rustfmt::skip]
const STEP_NUMERATORS:   [i32; 18] = [ 1,  1,  1, 1, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];
#[rustfmt::skip]
const STEP_DENOMINATORS: [i32; 18] = [30, 21, 13, 8, 5, 3, 2, 1, 1, 1, 1, 1,  1,  1,  1,  1,  1,   1];

impl BitFlipper {
    pub fn new(width: i32, height: i32) -> Self {
        let view_width = width;
        let view_height = height;

        let step_index = 1;
        let t = 0;
        let x = 0;
        let y = 0;
        let dir_x = 3;
        let dir_y = 5;
        let bits = BitGrid::new(view_width as usize, view_height as usize);

        Self {
            view_height,
            view_width,
            step_index,
            t,
            x,
            y,
            dir_x,
            dir_y,
            bits,
        }
    }

    fn current_step_count(&mut self) -> i32 {
        10920 * STEP_NUMERATORS[self.step_index.unsigned_abs() - 1]
            / STEP_DENOMINATORS[self.step_index.unsigned_abs() - 1]
            * self.step_index.signum() as i32
    }

    pub fn advance_by(&mut self, pixel_delta: i32) {
        for _ in 0..pixel_delta.abs() {
            self.flip_and_advance(pixel_delta.signum())
        }
    }

    fn flip_and_advance(&mut self, _dir: i32) {
        let flipped_x_pixel = self.current_x_pixel();
        let flipped_y_pixel = self.current_y_pixel();
        self.flip(flipped_x_pixel, flipped_y_pixel);

        loop {
            let next_x = ((self.x / self.dir_y.abs()) + self.dir_x.signum()) * self.dir_y.abs();
            let next_y = ((self.y / self.dir_x.abs()) + self.dir_y.signum()) * self.dir_x.abs();

            let dist_x = next_x - self.x;
            let dist_y = next_y - self.y;

            if (dist_x * self.dir_x).abs() < (dist_y * self.dir_y).abs() {
                // next x boundary is closer
                self.x = next_x;
                self.y += dist_x * self.dir_x / self.dir_y;
            } else {
                // next y boundary is closer
                self.y = next_y;
                self.x += dist_y * self.dir_y / self.dir_x;
            }

            if self.x == 0 || self.x == self.view_width * self.dir_y.abs() {
                self.dir_x *= -1;
            }

            if self.y == 0 || self.y == self.view_width * self.dir_x.abs() {
                self.dir_y *= -1;
            }

            if self.current_x_pixel() != flipped_x_pixel
                || self.current_y_pixel() != flipped_y_pixel
            {
                break;
            }
        }
    }

    fn current_x_pixel(&mut self) -> i32 {
        self.x / self.dir_y.abs() + if self.dir_x > 0 { 0 } else { -1 }
    }

    fn current_y_pixel(&mut self) -> i32 {
        self.y / self.dir_x.abs() + if self.dir_y > 0 { 0 } else { -1 }
    }

    fn flip(&mut self, x_pixel: i32, y_pixel: i32) {
        self.bits.flip(x_pixel as i16, y_pixel as i16);
    }

    pub fn step_index_forward(&mut self) {
        if self.step_index < STEP_NUMERATORS.len() as isize {
            self.step_index += 1;
        }
    }

    pub fn step_index_bakward(&mut self) {
        if self.step_index.abs() < STEP_NUMERATORS.len() as isize {
            self.step_index -= 1;
        }
    }

    pub fn step(&mut self) {
        self.t += self.current_step_count();
        let pixel_delta = self.t / 10920;
        self.t -= pixel_delta * 10920;
        self.advance_by(pixel_delta);

    }
}
