//! WIP Trait to manage multiple scenes

use rand::rngs::SmallRng;

use crate::peripherals::SH1107Display;

/// A test scene that runs Conways Game of Life
mod conway;
pub use conway::*;

/// The main attraction: a scene that displays a Bitflipper simulation
mod bitflipper;
pub use bitflipper::*;

/// Debugging scene that displays whatever text we care about
mod debug_text;
pub use debug_text::*;

/// About scene that displays information about who made this and the build that's running
mod credits;
pub use credits::*;

/// Information passed to scens with [`Scene::update()`]
pub struct Context<'a> {
    /// Random Number Generator
    pub rng: &'a mut SmallRng,

    /// Whether or not the A button / Key1 is pressed
    pub btn_a: bool,

    /// Whether or not the B button / Key0 is pressed
    pub btn_b: bool,

    /// Time in microseconds since boot, so that scenes can wait
    pub time: u64,
}

/// A trait that describes what actions a Scene might need to do in response to user input
pub trait Scene {
    /// Called in a loop with user input updates etc
    ///
    /// Returns true when the display should update. Implementors that do not change what they present can return false
    /// to help the system do fewer display updates.
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice;
}
