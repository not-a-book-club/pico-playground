//! WIP Trait to manage multiple scenes

use cortex_m::delay::Delay;
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

/// A trait that describes what actions a Scene might need to do in response to user input
///
/// - When a scene is first switched into, its `init()` method is called to setup any one-time work.
/// - After that, the `update()` method is called on repeat until the user switches out.
/// - When a scene is being switched out of, its `deinit()` method is called to perform any one-time work or cleanup.
///
/// Scene objects are not deallocated between scene switches, but can choose to reset state in `init()` and `deinit()`.
pub trait Scene {
    /// Called before a scene has started updating
    // TODO: Should probably call this new() and give it a Display+RNG instead
    fn init(&mut self, ctx: &mut Context<'_>) {
        let _ = ctx;
    }

    /// Called in a loop with user input updates etc
    ///
    /// Returns true if it wants a screen update
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
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
