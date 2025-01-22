#![no_std]
#![no_main]

// Runtime things
extern crate alloc;
use defmt_rtt as _;
use panic_probe as _;

// Embedded things
use cortex_m::delay::Delay;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_bus::spi::ExclusiveDevice;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::*;

#[rp_pico::entry]
fn main() -> ! {
    // Init Heap
    {
        #![allow(static_mut_refs)]
        use core::mem::MaybeUninit;
        use embedded_alloc::LlffHeap as Heap;

        #[global_allocator]
        static HEAP: Heap = Heap::empty();

        // NOTE: The rp2040 has 264 kB of on-chip SRAM
        const HEAP_SIZE: usize = 132 * 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    }

    // Singleton objects
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();

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

    // The delay object lets us wait for specified amounts of time
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Log some interesting data from ROM
    unsafe {
        use hal::rom_data as rom;

        info!("\"{}\"", rom::copyright_string());
        info!("rom_version_number: {}", rom::rom_version_number());

        let fplib_start = rom::fplib_start();
        let fplib_end = rom::fplib_end();
        info!(
            "fplib: {} bytes [0x{:08x}, 0x{:08x}]",
            fplib_start.offset_from(fplib_end),
            fplib_start,
            fplib_end,
        );

        info!("bootrom git rev: {}", rom::git_revision());
    }

    // === OLED Specific Code Begins ==========================================

    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();

    let mut btn_a = pins.gpio15.into_pull_up_input();
    let mut btn_b = pins.gpio17.into_pull_up_input();

    // LED on the board - we use this mostly for proof-of-life
    let mut led = pins.led.into_push_pull_output();

    let sclk = pins.gpio10.into_function::<hal::gpio::FunctionSpi>();
    let mosi = pins.gpio11.into_function::<hal::gpio::FunctionSpi>();
    let spi_bus = hal::Spi::<_, _, _, 8>::new(pac.SPI1, (mosi, sclk));
    let spi_bus = spi_bus.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_u32.MHz(),
        embedded_hal::spi::MODE_0,
    );
    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let spi_dev = ExclusiveDevice::new(spi_bus, cs, timer).unwrap();

    // Log some interesting data from ROM
    unsafe {
        use hal::rom_data as rom;

        info!("\"{}\"", rom::copyright_string());
        info!("rom_version_number: {}", rom::rom_version_number());

        let fplib_start = rom::fplib_start();
        let fplib_end = rom::fplib_end();
        info!(
            "fplib: {} bytes [0x{:08x}, 0x{:08x}]",
            fplib_start.offset_from(fplib_end),
            fplib_start,
            fplib_end,
        );

        info!("bootrom git rev: {}", rom::git_revision());
    }

    let mut display = OledDriver::new(spi_dev, dc, &mut rst, &mut delay);

    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));
    let mut sim = simulations::Life::new(oled::WIDTH as usize, oled::HEIGHT as usize);
    sim.clear_random(&mut rng);

    let mut contrast = 0;
    loop {
        // Functionally, this is brightness
        display.set_contrast(contrast);
        contrast = contrast.wrapping_add(8);

        led.set_high().unwrap();
        display.inverse_on();
        delay.delay_ms(400);

        let a = btn_a.is_low().unwrap();
        let b = btn_b.is_low().unwrap();

        match (a, b) {
            // Reset back to BOOTSEL so that the next cargo-run updates our code
            (true, true) => hal::rom_data::reset_to_usb_boot(0, 0),

            // Press A to clear to random
            (true, _) => sim.clear_random(&mut rng),

            _ => {}
        }

        let n_updated = sim.step();
        if n_updated != 0 {
            for y in 0..sim.height() {
                for x in 0..sim.width() {
                    if sim.get(x, y) {
                        // display.set(x, y);
                    }
                }
            }
        }

        led.set_low().unwrap();
        display.inverse_off();
        delay.delay_ms(400);
    }
}
