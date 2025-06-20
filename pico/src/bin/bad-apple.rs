#![no_std]
#![no_main]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    unused
)]

// Runtime things
extern crate alloc;

// Embedded things
use cortex_m::delay::Delay;
use embedded_alloc::LlffHeap as Heap;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::Text;
use embedded_hal::digital::{InputPin, OutputPin};
use fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::peripherals::*;
use pico::scene::*;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cortex_m_rt::entry]
fn entry() -> ! {
    main();
    unreachable!();
}

fn main() {
    // === Setup embedded things ==========================================
    // Init Heap
    {
        #![allow(static_mut_refs)]
        use core::mem::MaybeUninit;

        // NOTE: The rp2040 has 264 kB of on-chip SRAM
        const HEAP_SIZE: usize = 192 * 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

        unsafe {
            HEAP.init(HEAP_MEM.as_mut_ptr() as usize, HEAP_SIZE);
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

    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();

    // Note: key1 on the board
    let mut btn_a = pins.gpio15.into_pull_up_input();
    // Note: key0 on the board
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
    let spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new(spi_bus, cs, timer).unwrap();

    let driver = SH1107Driver::new(spi_dev, dc, &mut rst, &mut delay);
    let mut display = SH1107Display::new(driver);

    // See if we left any interesting panic info in RAM
    if let Some(mut msg) = panic_persist::get_panic_message_utf8() {
        display.clear_unset();
        display.driver().inverse_on();

        if msg.starts_with("panicked at src/") {
            msg = &msg[16..];
        }

        if let Some(short_msg) = msg.strip_prefix("panicked at ") {
            if let Some(short_msg) = msg.strip_prefix(env!("CARGO_MANIFEST_DIR")) {
                msg = short_msg;
            }
        }

        // Panic info
        {
            // Chunk the text to fit our display.
            let mut y = 6;
            pico::chunk_lines(msg, 24, |line: &str| {
                let text = Text::new(
                    line,
                    Point::new(5, y),
                    MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
                );
                let _ = text.draw(&mut display);
                y += 8;
            });
        }

        display.flush();

        // Wait for input and then reset to USB mode
        {
            for i in 0.. {
                let a = btn_a.is_low().unwrap_or(true);
                let b = btn_b.is_low().unwrap_or(true);
                if a || b {
                    break;
                }

                if i % 2 == 0 {
                    let _ = led.set_high();
                } else {
                    let _ = led.set_low();
                }
                delay.delay_ms(100);
            }

            hal::rom_data::reset_to_usb_boot(0, 0);
        }
    }

    // Show a pretty title screen, and wait on it until user input
    {
        let width = display.width() as i32;
        let height = display.height() as i32;

        // Fullscreen white-border
        let style_white_border = PrimitiveStyleBuilder::new()
            .stroke_width(1)
            .stroke_color(BinaryColor::On)
            .build();
        let r = 4;
        let screen_border = RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32)),
            Size::new(r, r),
        );
        let _ = screen_border.draw_styled(&style_white_border, &mut display);

        // Classic text
        let anykey = Text::new(
            "PRESS ANY KEY",
            Point::new(32, 3 * height / 4),
            MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
        );
        let _ = anykey.draw(&mut display);

        display.flush();

        // Wait until a button press
        for i in 0.. {
            let a: bool = btn_a.is_low().unwrap();
            let b: bool = btn_b.is_low().unwrap();

            // If EITHER A or B are pressed, move on to the next screen
            if a || b {
                break;
            }

            // Toggle the LED every ~500ms while waiting for input
            if i % 10 == 0 {
                let _ = led.set_high();
            } else if i % 10 == 5 {
                let _ = led.set_low();
            }

            delay.delay_ms(100);
        }

        display.clear_unset();
    }

    let seed_bytes = timer.get_counter_low().to_le_bytes();
    let seed = core::array::from_fn(|i| seed_bytes[i % 4]);
    let mut rng = SmallRng::from_seed(seed);

    let mut ctx = Context {
        rng: &mut rng,
        btn_a: false,
        btn_b: false,
        time: timer.get_counter().ticks(),
    };

    let mut scene = pico::scene::BadAppleScene::new(ctx.time);

    loop {
        ctx.btn_a = btn_a.is_low().unwrap();
        ctx.btn_b = btn_b.is_low().unwrap();
        ctx.time = timer.get_counter().ticks();

        if ctx.btn_a && ctx.btn_b {
            panic!("Ha-ah! Panic handling works! {}:{}", file!(), line!());
        }

        if scene.update(&mut ctx, &mut display) {
            display.flush();
        }

        delay.delay_us(1_000);
    }
}
