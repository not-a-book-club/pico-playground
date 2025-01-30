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

// Embedded things
use cortex_m::delay::Delay;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::Text;
use embedded_hal::digital::InputPin;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

// Import all 4 even if we aren't using them at this moment
#[allow(unused_imports)]
use defmt::{debug, error, info, warn};

use rand::{rngs::SmallRng, SeedableRng};

use pico::oled::{Display, SH1107Driver};
use pico::scene::*;

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
    // === Setup embedded things ==========================================

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

    // === OLED Specific setup ==========================================
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
    display.clear_unset();
    display.flush();

    // === Let's go BitFlipper! ==========================================
    do_titlescreen(&mut display, &mut btn_a, &mut btn_b, &mut delay);

    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));
    let mut bitflipper_scene = pico::scene::BitflipperScene::new(&display);
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

        let needs_flush = bitflipper_scene.update(&mut ctx, &mut display);
        if needs_flush {
            display.flush();
        }
    }
}

fn do_titlescreen<Device, DataCmdPin>(
    display: &mut Display<Device, DataCmdPin>,
    btn_a: &mut impl InputPin,
    btn_b: &mut impl InputPin,
    delay: &mut Delay,
) where
    Device: embedded_hal::spi::SpiDevice,
    DataCmdPin: embedded_hal::digital::OutputPin,
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
    let _ = screen_border.draw_styled(&style_white_border, display);

    // Draw "bitflipper", stylized
    {
        let mut bit_style = MonoTextStyle::new(&ascii::FONT_6X13_BOLD, BinaryColor::Off);
        bit_style.set_background_color(Some(BinaryColor::On));
        let bit = Text::new("BIT", Point::new(38, 19), bit_style);
        let _ = bit.draw(display);

        let flipper_style = MonoTextStyle::new(&ascii::FONT_6X13_ITALIC, BinaryColor::On);
        let flipper = Text::new("flipper", Point::new(58, 22), flipper_style);
        let _ = flipper.draw(display);
    }

    // Draw some lines below everything
    for i in 0..3 {
        let xs = width * 1 / 8 + 3 * (3 - i);
        let xe = width * 7 / 8 - 3 * (3 - i);
        let y = 3 * height / 4 + (i - 1) * 5 - 16;
        let line0 = Line::new(Point::new(xs, y), Point::new(xe, y));
        let _ = line0.draw_styled(&style_white_border, display);
    }

    // Instruct the obediant
    let anykey = Text::new(
        "PRESS ANY KEY",
        Point::new(32, 3 * height / 4 + 4),
        MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
    );
    let _ = anykey.draw(display);

    display.flush();

    // Wait until a button press
    loop {
        let a = btn_a.is_low().unwrap();
        let b = btn_b.is_low().unwrap();
        if a || b {
            break;
        }

        delay.delay_ms(100);
    }

    display.clear_unset();
}
