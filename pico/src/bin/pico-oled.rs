#![no_std]
#![no_main]
#![allow(clippy::identity_op)]

// Runtime things
extern crate alloc;
use defmt_rtt as _;
// use panic_probe as _;

// Embedded things
use cortex_m::delay::Delay;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::{renderer::TextRenderer, Text};
use embedded_hal::digital::InputPin;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::*;

// Reboot to BOOTSEL on panic
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::sync::atomic::*;

    static PANICKED: AtomicBool = AtomicBool::new(false);

    cortex_m::interrupt::disable();

    // Guard against infinite recursion, just in case.
    if !PANICKED.load(Ordering::Relaxed) {
        PANICKED.store(true, Ordering::Relaxed);
        error!("[PANIC]: {:?}", info);
    }

    hal::rom_data::reset_to_usb_boot(0, 0);
    cortex_m::asm::udf();
}

#[cortex_m_rt::entry]
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
    let mut _led = pins.led.into_push_pull_output();

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
    let spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new(spi_bus, cs, timer).unwrap();

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

    let driver = OledDriver::new(spi_dev, dc, &mut rst, &mut delay);
    let mut display = oled::Display::new(driver);

    let view_width = oled::WIDTH as u32;
    let view_height = oled::HEIGHT as u32 - 15;
    let style_white_border = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(oled::BinaryColor::On)
        // .fill_color(oled::BinaryColor::Off)
        .build();
    let _style_fill_black = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(oled::BinaryColor::On)
        .fill_color(oled::BinaryColor::On)
        .build();
    let _style_fill_white = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(oled::BinaryColor::On)
        .fill_color(oled::BinaryColor::On)
        .build();

    let style_text = MonoTextStyle::new(&ascii::FONT_5X8, oled::BinaryColor::On);
    let line_height = style_text.line_height() as i32;
    let line_margin = line_height / 3;

    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));
    let mut sim = simulations::Life::new(oled::WIDTH as usize, oled::HEIGHT as usize);
    sim.clear_random(&mut rng);

    let mut needs_refresh = true;

    loop {
        // led.set_high().unwrap();
        // delay.delay_ms(100);

        let a = btn_a.is_low().unwrap();
        let b = btn_b.is_low().unwrap();

        match (a, b) {
            // Reset back to BOOTSEL so that the next cargo-run updates our code
            (true, true) => hal::rom_data::reset_to_usb_boot(0, 0),

            // Press A to clear to random
            (true, _) => sim.clear_random(&mut rng),

            // Press B to spawn random circles
            (_, true) => {
                use rand::Rng;
                let n = 10;
                let xx: i16 = rng.gen_range(2 * n..sim.width()) - n;
                let yy: i16 = rng.gen_range(2 * n..sim.height()) - n;
                for y in (yy - n)..(yy + n) {
                    for x in (xx - n)..(xx + n) {
                        let dist = (x - xx).abs() + (y - yy).abs();
                        if dist <= n && dist % 3 == 0 {
                            sim.set(x, y, true);
                        }
                    }
                }
            }

            _ => {}
        }

        // let n_updated = 0;
        let n_updated = sim.step();
        if n_updated != 0 {
            needs_refresh = true;
        }

        // Draw!
        if needs_refresh {
            let base_y = (oled::HEIGHT as u32 - view_height) as i32;

            // Draw a nice title
            let text = Text::new(
                "Conway's Game of Life",
                Point::new(3, base_y - 3),
                style_text,
            );
            let _ = text.draw(&mut display);

            // Draw our sim "to" the view
            for y in (base_y as i16 + 3)..(sim.height() - 3) {
                for x in 3..(sim.width() - 3) {
                    let is_alive = sim.get(x, y);
                    display.set(x, y, is_alive);
                }
            }

            // Draw border around our view
            let _ = RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(0, base_y), Size::new(view_width, view_height)),
                Size::new(5, 5),
            )
            .draw_styled(&style_white_border, &mut display);

            display.flush();
            needs_refresh = false;
        }

        // led.set_low().unwrap();
        // delay.delay_ms(500);
    }
}
