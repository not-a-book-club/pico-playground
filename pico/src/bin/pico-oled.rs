#![no_std]
#![no_main]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if
)]

// Runtime things
extern crate alloc;
use defmt_rtt as _;
// use panic_probe as _;
use alloc::format;

// Embedded things
use cortex_m::delay::Delay;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::{renderer::TextRenderer, Text};
use embedded_hal::digital::InputPin;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::oled::{Display, SH1107Driver};

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
            fplib_end.offset_from(fplib_start),
            fplib_start,
            fplib_end,
        );

        info!("bootrom git rev: {}", rom::git_revision());
    }

    // === OLED Specific Code Begins ==========================================
    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();

    // Note: key0 on the board
    let mut btn_a = pins.gpio15.into_pull_up_input();
    // Note: key1 on the board
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

    let driver = SH1107Driver::new(spi_dev, dc, &mut rst, &mut delay);
    let mut display = Display::new(driver);

    let view_width = display.width() as u32;
    let view_height = display.height() as u32;
    let style_white_border = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        // .fill_color(BinaryColor::Off)
        .build();
    let _style_fill_black = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .fill_color(BinaryColor::On)
        .build();
    let _style_fill_white = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .fill_color(BinaryColor::On)
        .build();

    // Draw a title screen of sorts
    {
        let width = display.width() as i32;
        let height = display.height() as i32;

        // Fullscreen white-border
        let r = 4;
        let screen_border = RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32)),
            Size::new(r, r),
        );
        let _ = screen_border.draw_styled(&style_white_border, &mut display);

        // Draw "bitflipper", stylized
        {
            let mut bit_style = MonoTextStyle::new(&ascii::FONT_6X13_BOLD, BinaryColor::Off);
            bit_style.set_background_color(Some(BinaryColor::On));
            let bit = Text::new("BIT", Point::new(38, 32), bit_style);
            let _ = bit.draw(&mut display);

            let flipper_style = MonoTextStyle::new(&ascii::FONT_6X13_ITALIC, BinaryColor::On);
            let flipper = Text::new("flipper", Point::new(58, 35), flipper_style);
            let _ = flipper.draw(&mut display);
        }

        // Draw some lines below everything
        for i in 0..3 {
            let xs = width * 1 / 8 + 3 * (3 - i);
            let xe = width * 7 / 8 - 3 * (3 - i);
            let y = 3 * height / 4 + (i - 1) * 5;
            let line0 = Line::new(Point::new(xs, y), Point::new(xe, y));
            let _ = line0.draw_styled(&style_white_border, &mut display);
        }

        // Animate a load bar (this goes too far but it's hilarious so leave it alone please)
        let time = 16; // units of 100ms
        let xs = width * 1 / 16;
        let xe = width * 1 / 16;
        let y = height / 5;
        for i in 1..=time {
            let _ = Line::new(Point::new(xs, y), Point::new(xe + i * width / 16, y))
                .draw_styled(&style_white_border, &mut display);

            display.flush();

            delay.delay_ms(100);
        }

        display.flush();
        display.clear_unset();
    }

    let style_text = MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On);
    let style_text_tiny = MonoTextStyle::new(&ascii::FONT_4X6, BinaryColor::On);
    let line_height = style_text.line_height() as i32;
    let _line_margin = line_height / 3;

    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));

    // loop {
    //     led.set_high().unwrap();
    //     delay.delay_ms(100);
    //     led.set_low().unwrap();
    //     delay.delay_ms(500);
    // }

    let mut state = 0;
    let mut conway_scene = pico::scene::ConwayScene::new(&display);
    let mut bitflipper_scene = pico::scene::BitflipperScene::new(&display);

    'screens: loop {
        // Delay when changing
        delay.delay_ms(500);

        match state {
            0 => {
                use pico::scene::*;

                display.clear_unset();
                display.flush();

                let mut ctx = Context {
                    rng: &mut rng,
                    btn_a: false,
                    btn_b: false,
                    delay: &mut delay,
                };
                bitflipper_scene.init(&mut ctx);

                loop {
                    ctx.btn_a = btn_a.is_low().unwrap();
                    ctx.btn_b = btn_b.is_low().unwrap();

                    // Reset back to BOOTSEL so that the next cargo-run updates our code
                    if ctx.btn_a && ctx.btn_b {
                        hal::rom_data::reset_to_usb_boot(0, 0);
                        unreachable!();
                    }

                    // Press A to go to the next screen
                    if ctx.btn_a && !ctx.btn_b {
                        state += 1;
                        bitflipper_scene.deinit(&mut ctx);
                        continue 'screens;
                    } else {
                        let needs_flush = bitflipper_scene.update(&mut ctx, &mut display);
                        if needs_flush {
                            display.flush();
                        }
                    }
                }
            }

            1 => {
                use pico::scene::*;

                display.clear_unset();
                display.flush();

                let mut ctx = Context {
                    rng: &mut rng,
                    btn_a: false,
                    btn_b: false,
                    delay: &mut delay,
                };
                conway_scene.init(&mut ctx);

                loop {
                    ctx.btn_a = btn_a.is_low().unwrap();
                    ctx.btn_b = btn_b.is_low().unwrap();

                    // Reset back to BOOTSEL so that the next cargo-run updates our code
                    if ctx.btn_a && ctx.btn_b {
                        hal::rom_data::reset_to_usb_boot(0, 0);
                        unreachable!();
                    }

                    // Press A to go to the next screen
                    if ctx.btn_a && !ctx.btn_b {
                        state += 1;
                        conway_scene.deinit(&mut ctx);
                        continue 'screens;
                    } else {
                        let needs_flush = conway_scene.update(&mut ctx, &mut display);
                        if needs_flush {
                            display.flush();
                        }
                    }
                }
            }

            // Copy and Paste this block when adding a new "screen"
            _ => {
                display.clear_unset();
                let text = format!("End of the line.\nNo more scenes.\nstate={state:?}");
                let text = Text::new(&text, Point::new(8, 16), style_text);
                let _ = text.draw(&mut display);
                display.flush();
                delay.delay_ms(1000);

                loop {
                    let a = btn_a.is_low().unwrap();
                    let b = btn_b.is_low().unwrap();

                    match (a, b) {
                        // Reset back to BOOTSEL so that the next cargo-run updates our code
                        (true, true) => hal::rom_data::reset_to_usb_boot(0, 0),

                        // Press A to go to the next screen
                        (true, _) => {
                            // Do this:
                            // state += 1;
                            // continue 'screens;

                            // Not this:
                            state = 0;
                            continue 'screens;
                        }

                        (_, true) => {
                            //
                        }

                        _ => {}
                    }

                    delay.delay_ms(100);
                }
            }
        }
    }
}
