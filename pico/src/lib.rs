#![no_std]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if
)]

extern crate alloc;

pub mod image;
pub use image::{Image, Rgb565};

pub mod lcd;
pub use lcd::LcdDriver;

pub mod oled;

pub mod scene;

pub const AOC_BLUE: Rgb565 = Rgb565::from_rgb888(0x0f_0f_23);
pub const AOC_GOLD: Rgb565 = Rgb565::from_rgb888(0xff_ff_66);
pub const OHNO_PINK: Rgb565 = Rgb565::new(0xF8_1F);
