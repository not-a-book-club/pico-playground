#![allow(dead_code, unused)]
//! WIP Trait to manage multiple scenes

use crate::oled::Display;

use cortex_m::delay::Delay;
use rand::rngs::SmallRng;

use core::cmp;

/// A trait that describes what actions a Scene might need to do in response to user input
///
/// - When a scene is first switched into, its `init()` method is called to setup any one-time work.
/// - After that, the `update()` method is called on repeat until the user switches out.
/// - When a scene is being switched out of, its `deinit()` method is called to perform any one-time work or cleanup.
///
/// Scene objects are not deallocated between scene switches, but can choose to reset state in `init()` and `deinit()`.
pub trait Scene {
    /// Called before a scene has started updating
    fn init(&mut self, ctx: &mut Context<'_>) {
        let _ = ctx;
    }

    /// Called in a loop with user input updates etc
    ///
    /// Returns true if it wants a screen update
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice;

    /// After a scene stops being in focus, its deinit() method is called to perform any additional one-time work or cleanup resources.
    fn deinit(&mut self, ctx: &mut Context<'_>) {
        let _ = ctx;
    }
}

/// Information passed to [`Scene::update()`] call
pub struct Context<'a> {
    /// Random Number Generator
    pub rng: &'a mut SmallRng,

    /// Whether the A button / Key1 is pressed or not
    pub btn_a: bool,

    /// Whether the B button / Key0 is pressed or not
    pub btn_b: bool,

    /// Allows Scenes to sleep in their update() calls
    pub delay: &'a mut Delay,
}

use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

pub struct ConwayScene {
    sim: simulations::Life,

    view_width: u32,
    view_height: u32,
    base_y: i32,
}

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
}

#[rustfmt::skip]
const STEP_NUMERATORS:   [i32; 22] = [ 1,  1,  1, 1, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987];
#[rustfmt::skip]
const STEP_DENOMINATORS: [i32; 22] = [30, 21, 13, 8, 5, 3, 2, 1, 1, 1, 1, 1,  1,  1,  1,  1,  1,   1,   1,   1,   1,   1];

impl BitflipperScene {
    pub fn new<Device, DataCmdPin>(display: &crate::oled::Display<Device, DataCmdPin>) -> Self
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        let view_width = display.width() as i32;
        let view_height = display.height() as i32;

        let step_index = 1;
        let t = 0;
        let x = 0;
        let y = 0;
        let dir_x = 1923;
        let dir_y = 3391;
        let bits = simulations::BitGrid::new(view_width as usize, view_height as usize);

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
        if self.step_index == 0 {
            return 0;
        }

        return 10920 * STEP_NUMERATORS[self.step_index.abs() as usize - 1]
            / STEP_DENOMINATORS[self.step_index.abs() as usize - 1]
            * self.step_index.signum();
    }

    fn advance_by(&mut self, pixel_delta: i32) {
        for _ in 0..pixel_delta.abs() {
            self.flip_and_advance(pixel_delta.signum())
        }
    }

    fn flip_and_advance(&mut self, dir: i32) {
        if (dir.signum() > 0) {
            self.flipBit()
        }

        let next_x = (((self.x + if self.dir_x * dir < 0 { -1 } else { 0 }) / self.dir_y.abs())
            + if self.dir_x * dir >= 0 { 1 } else { 0 })
            * self.dir_y.abs();
        let next_y = (((self.y + if self.dir_y * dir < 0 { -1 } else { 0 }) / self.dir_x.abs())
            + if self.dir_y * dir >= 0 { 1 } else { 0 })
            * self.dir_x.abs();

        let dist_x = next_x - self.x;
        let dist_y = next_y - self.y;

        let move_amount = cmp::min(dist_x.abs(), dist_y.abs());

        self.x += move_amount * dir * self.dir_x.signum();
        self.y += move_amount * dir * self.dir_y.signum();

        if (dir.signum() < 0) {
            self.flipBit()
        }

        if (self.x == 0 || self.x == self.view_width * self.dir_y.abs()) {
            self.dir_x *= -1;
        }

        if (self.y == 0 || self.y == self.view_height * self.dir_x.abs()) {
            self.dir_y *= -1;
        }
    }

    fn flipBit(&mut self) {
        let x_pixel = (self.x + if self.dir_x >= 0 { 0 } else { -1 }) / self.dir_y.abs();
        let y_pixel = (self.y + if self.dir_y >= 0 { 0 } else { -1 }) / self.dir_x.abs();
        self.bits.flip(x_pixel as i16, y_pixel as i16);
    }
}

impl Scene for BitflipperScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        if ctx.btn_a {
            if self.step_index > 0 || self.step_index.abs() < STEP_NUMERATORS.len() as i32 {
                self.step_index -= 1;
                ctx.delay.delay_ms(200);
            }
        }

        if ctx.btn_b {
            if self.step_index < STEP_NUMERATORS.len() as i32 {
                self.step_index += 1;
                ctx.delay.delay_ms(200);
            }
        }

        self.t += self.current_step_count();
        let pixel_delta = self.t / 10920;
        self.t -= pixel_delta * 10920;
        self.advance_by(pixel_delta);
        display.flush_with(&self.bits);

        false
    }
}

impl ConwayScene {
    pub fn new<Device, DataCmdPin>(display: &crate::oled::Display<Device, DataCmdPin>) -> Self
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
        display: &mut Display<Device, DataCmdPin>,
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
