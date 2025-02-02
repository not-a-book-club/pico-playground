#![no_std]
#![no_main]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if
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
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::peripherals::{SH1107Display, SH1107Driver};
use pico::scene::*;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cortex_m_rt::entry]
fn entry() -> ! {
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

    // === OLED Specific setup ==========================================
    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();

    // Note: key0 on the board
    let mut btn_a = pins.gpio15.into_pull_up_input();
    // Note: key1 on the board
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
    // TODO: Maybe we could dim the display slowly so the user doesn't notice and save battery?
    // display.driver().set_contrast(0);
    // See if we left any interesting panic info in RAM
    if let Some(mut msg) = panic_persist::get_panic_message_utf8() {
        display.clear_unset();
        display.driver().inverse_on();

        if msg.starts_with("panicked at src/") {
            msg = &msg[16..];
        }

        // Panic info
        {
            // Chunk the text to fit our display.
            let mut y = 6;
            pico::chunk_lines(msg, 30, |line: &str| {
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

        // Draw "bitflipper", stylized
        {
            let mut bit_style = MonoTextStyle::new(&ascii::FONT_6X13_BOLD, BinaryColor::Off);
            bit_style.set_background_color(Some(BinaryColor::On));
            let bit = Text::new("BIT", Point::new(38, 19), bit_style);
            let _ = bit.draw(&mut display);

            let flipper_style = MonoTextStyle::new(&ascii::FONT_6X13_ITALIC, BinaryColor::On);
            let flipper = Text::new("flipper", Point::new(58, 22), flipper_style);
            let _ = flipper.draw(&mut display);
        }

        // Draw some lines below everything
        for i in 0..3 {
            let xs = width * 1 / 8 + 3 * (3 - i);
            let xe = width * 7 / 8 - 3 * (3 - i);
            let y = 3 * height / 4 + (i - 1) * 5 - 16;
            let line0 = Line::new(Point::new(xs, y), Point::new(xe, y));
            let _ = line0.draw_styled(&style_white_border, &mut display);
        }

        // Instruct the obediant
        let anykey = Text::new(
            "PRESS ANY KEY",
            Point::new(32, 3 * height / 4 + 4),
            MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
        );
        let _ = anykey.draw(&mut display);

        display.flush();

        // Wait until a button press
        for i in 0.. {
            let a = btn_a.is_low().unwrap();
            let b = btn_b.is_low().unwrap();
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

    // Use the lower 32-bits of the timer to seed our RNG.
    // These count in usec, so ignoring the higher bits causes us to
    // "repeat" seeds after someone waits 71 minutes on the title screen.
    // Seems fine.
    let seed_bytes = timer.get_counter_low().to_le_bytes();
    let seed = core::array::from_fn(|i| seed_bytes[i % 4]);
    let mut rng = SmallRng::from_seed(seed);

    let mut ctx = Context {
        rng: &mut rng,
        btn_a: false,
        btn_b: false,
        delay: &mut delay,
    };

    let mut scene = pico::scene::BitflipperScene::new(&display);

    // Use this to debug some text
    if false {
        let mut scene = pico::scene::DebugTextScene::new(&display);
        scene.init(&mut ctx);

        let chip_id = pac.SYSINFO.chip_id().read();
        let chip_id_manufacturer: u16 = chip_id.manufacturer().bits();
        let chip_id_part: u16 = chip_id.part().bits();
        let chip_id_revision: u8 = chip_id.revision().bits();

        let gitref_rp2040_spec: u32 = pac.SYSINFO.gitref_rp2040().read().bits();

        let nmi_p0 = pac.SYSCFG.proc0_nmi_mask().read().bits();
        let nmi_p1 = pac.SYSCFG.proc1_nmi_mask().read().bits();

        scene.text = alloc::format!(
            r#"System Info
  chmnfct  = 0x{chip_id_manufacturer:x}
  chpart   = 0x{chip_id_part:x}
  chrev    = 0x{chip_id_revision:x}
  hwgitref = 0x{gitref_rp2040_spec:x}
SYSCFG
  nmi_p0   = 0b{nmi_p0:b}
  nmi_p1   = 0b{nmi_p1:b}
"#
        );
    }

    loop {
        ctx.btn_a = btn_a.is_low().unwrap();
        ctx.btn_b = btn_b.is_low().unwrap();

        if ctx.btn_a && ctx.btn_b {
            panic!("Ha-ah! Panic handling works! {}:{}", file!(), line!());
        }

        let needs_flush = scene.update(&mut ctx, &mut display);
        if needs_flush {
            display.flush();
        }
    }
}
