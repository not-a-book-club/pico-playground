#![no_std]
#![no_main]
#![allow(clippy::identity_op)]

// Runtime things
extern crate alloc;
use panic_probe as _;

// Embedded things
use cortex_m::delay::Delay;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::pwm::SetDutyCycle;
use embedded_hal_bus::spi::ExclusiveDevice;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

use rand::{rngs::SmallRng, SeedableRng};
use simulations::Elementry;
use simulations::Life;

use pico::peripherals::*;
use pico::*;

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

    // === LCD Specific Code Begins ===========================================

    // GPIOs:
    //      ePaper Pico/Pico2  Description
    //      VCC    VSYS        Power Input
    //      GND    GND         GND
    //      DIN    GP11        MOSI pin of SPI, slave device data input
    //      CLK    GP10        SCK pin of SPI, clock pin
    //      CS     GP9         Chip selection of SPI, low active
    //      DC     GP8         Data/Command control pin (High for data; Low for command)
    //      RST    GP12        Reset pin, low active
    //      BL     GP13        Backlight control
    //      A      GP15        User button A
    //      B      GP17        User button B
    //      X      GP19        User button X
    //      Y      GP21        User buttonY
    //      UP     GP2         Joystick-up
    //      DOWM   GP18        Joystick-down
    //      LEFT   GP16        Joystick-left
    //      RIGHT  GP20        Joystick-right
    //      CTRL   GP3         Joystick-center
    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let mut rst = pins.gpio12.into_push_pull_output();
    let _bl = pins
        .gpio13
        .into_push_pull_output_in_state(hal::gpio::PinState::High);

    let mut btn_a = pins.gpio15.into_pull_up_input();
    let mut btn_b = pins.gpio17.into_pull_up_input();
    let mut _btn_x = pins.gpio19.into_pull_up_input();
    let mut btn_y = pins.gpio21.into_pull_up_input();

    // Note: Center is the middle joy thing and is easy to hit while using the directions
    let mut _joy_center = pins.gpio3.into_pull_up_input();
    let mut _joy_up = pins.gpio2.into_pull_up_input();
    let mut _joy_down = pins.gpio18.into_pull_up_input();
    let mut _joy_left = pins.gpio16.into_pull_up_input();
    let mut _joy_right = pins.gpio20.into_pull_up_input();

    // LED on the board - we use this mostly for proof-of-life
    let mut led = pins.led.into_push_pull_output();

    let sclk = pins.gpio10.into_function::<hal::gpio::FunctionSpi>();
    let mosi = pins.gpio11.into_function::<hal::gpio::FunctionSpi>();
    let layout = (mosi, sclk);
    let spi_bus = hal::Spi::<_, _, _, 8>::new(pac.SPI1, layout);
    let spi_bus = spi_bus.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_u32.MHz(),
        embedded_hal::spi::MODE_0,
    );
    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let spi_dev = ExclusiveDevice::new(spi_bus, cs, timer).unwrap();

    // TODO: This PWN work came from the sample code but I do not understand it.
    // Set PWM
    let pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    {
        // slicenum from bl pin
        let mut pwm = pwm_slices.pwm6;
        pwm.set_ph_correct();

        // chan B to 90
        pwm.channel_b.set_duty_cycle_fraction(9, 10).unwrap();
        pwm.enable();
    }

    let mut display = st7789::ST7789Display::new(spi_dev, dc, &mut rst, &mut delay);

    // Generate more of these at: https://coolors.co/313715-d16014
    // Pick two and hit Space to generate random pairs until you like what you see
    let palettes = [
        // [Background, Foreground]
        [AOC_BLUE, AOC_GOLD],
        [Rgb565::from_rgb888(0x1B081D), Rgb565::from_rgb888(0x830C8F)],
        [Rgb565::from_rgb888(0xFFFBFE), Rgb565::from_rgb888(0x7A7D7D)],
        [Rgb565::from_rgb888(0xD16014), Rgb565::from_rgb888(0x313715)],
    ];
    let mut palette = 0;

    let mut image = Image::new(st7789::WIDTH, st7789::HEIGHT);
    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));

    // Hold down Y to go into the elementary sim
    let do_life = btn_y.is_high().unwrap();
    if do_life {
        let mut sim = Life::new(st7789::WIDTH as usize / 4, st7789::HEIGHT as usize / 4);
        sim.clear_random(&mut rng);

        loop {
            led.set_high().unwrap();

            {
                let a = btn_a.is_low().unwrap();
                let b = btn_b.is_low().unwrap();
                match (a, b) {
                    // Reset back to BOOTSEL so that the next cargo-run updates our code
                    (true, true) => hal::rom_data::reset_to_usb_boot(0, 0),

                    // Press A to clear to random
                    (true, _) => sim.clear_random(&mut rng),

                    // Press B to cycle palettes
                    (_, true) => {
                        palette += 1;
                        palette %= palettes.len();
                    }

                    _ => {}
                }
            }

            let n_updated = sim.step();
            if n_updated != 0 {
                for y in 0..sim.height() {
                    for x in 0..sim.width() {
                        let is_alive = sim.get(x, y);

                        // Write a 4x4 big pixel
                        for dy in 0..4 {
                            let yy = 4 * (y as u16) + dy;
                            for dx in 0..4 {
                                let xx = 4 * (x as u16) + dx;
                                image[(xx, yy)] = palettes[palette][is_alive as usize];
                            }
                        }
                    }
                }
            }

            display.present(&image);

            led.set_low().unwrap();
            delay.delay_ms(100);
        }
    } else {
        // let rule = 30;
        // let rule = 45;
        // let rule = 89;
        let rule = 90;
        // let rule = 110;
        // let rule = 184;

        let scale = 3;
        let mut sim = Elementry::new(rule, (st7789::HEIGHT / scale) as usize);
        sim.set(sim.width() / 2, true);
        image.fill(palettes[palette][0]);

        display.define_vertical_scroll_areas(0, 0);

        // Run our simulation sideways so we can use "vertical" scrolling to move it smoothly.
        loop {
            // Each column is a snapshot of the simulation
            for x in (0..st7789::WIDTH).step_by(scale as usize).rev() {
                {
                    let a = btn_a.is_low().unwrap();
                    let b = btn_b.is_low().unwrap();
                    match (a, b) {
                        // Reset back to BOOTSEL so that the next cargo-run updates our code
                        (true, true) => hal::rom_data::reset_to_usb_boot(0, 0),

                        // Press A to clear to random
                        (true, _) => sim.clear_random(&mut rng),

                        // Press B to cycle palettes
                        (_, true) => {
                            palette += 1;
                            palette %= palettes.len();
                        }

                        _ => {}
                    }
                }

                display.vertical_scroll_update(x);
                // Update the sim
                sim.step();

                // Write the updated state into our buffer
                for y in 0..sim.width() {
                    let is_alive = sim.get(y);
                    let y = y as u16;
                    // Write a scale by scale big pixel
                    for dx in 0..scale {
                        for dy in 0..scale {
                            image[((x + dx), scale * y + dy)] =
                                palettes[palette][is_alive as usize];
                        }
                    }
                }

                // Scroll and update the display with our new image
                display.present(&image);
                delay.delay_ms(10);
            }
        }
    }
}
