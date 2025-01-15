#![no_std]
#![no_main]

use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use rp_pico::hal;
use rp_pico::hal::pac;
use rp_pico::hal::prelude::*;

use life::Life;
use rand::{rngs::SmallRng, RngCore, SeedableRng};

#[rp_pico::entry]
fn main() -> ! {
    // Singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Watchdog timer - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    // The default is a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // The delay object lets us wait for specified amounts of time (in milliseconds)
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Set the LED to be an output
    let mut led_pin = pins.led.into_push_pull_output();

    let mut life = Life::new(64, 64);
    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 7));

    for y in 0..life.height() {
        for x in 0..life.width() {
            life.set(x, y, rng.next_u32() % 2 == 0);
        }
    }

    // Blink the LED!
    loop {
        led_pin.set_high().unwrap();
        delay.delay_ms(750);

        let n_updated = life.step();
        defmt::println!("Updated {} cells", n_updated);

        // Print it somewhere or something

        led_pin.set_low().unwrap();
        delay.delay_ms(250);
    }
}
