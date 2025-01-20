#![no_std]
#![no_main]

// Runtime things
extern crate alloc;
use defmt_rtt as _;
use panic_probe as _;

// Embedded things
use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;
use embedded_hal_bus::spi::ExclusiveDevice;
use hal::fugit::*;
use hal::prelude::*;
use rp_pico::hal;

use rand::{rngs::SmallRng, SeedableRng};
use simulations::Elementry;
use simulations::Life;

mod image;
use image::{Image, Rgb565};

mod lcd;
use lcd::LcdDriver;

pub const AOC_BLUE: Rgb565 = Rgb565::from_rgb888(0x0f_0f_23);
pub const AOC_GOLD: Rgb565 = Rgb565::from_rgb888(0xff_ff_66);
pub const OHNO_PINK: Rgb565 = Rgb565::new(0xF8_1F);

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
    let mut rst = pins.gpio12.into_push_pull_output();
    let dc = pins.gpio8.into_push_pull_output();
    let cs = pins.gpio9.into_push_pull_output();
    let _bl = pins
        .gpio13
        .into_push_pull_output_in_state(hal::gpio::PinState::High);

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

    // Need to reset the display before initializing it
    // TODO: Explain/cite the magic here
    {
        // Set PWM
        {
            let pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);

            // slicenum from bl pin
            let mut pwm = pwm_slices.pwm6;
            pwm.set_ph_correct();

            // chan B to 90
            pwm.channel_b.set_duty_cycle_fraction(9, 10).unwrap();
            pwm.enable();
        }
        // Reset
        {
            rst.set_high().unwrap();
            delay.delay_ms(100);

            rst.set_low().unwrap();
            delay.delay_ms(100);

            rst.set_high().unwrap();
            delay.delay_ms(100);
        }
    }

    // TODO: Read frame data before we init, it's a source of RNG!

    let mut display = LcdDriver::new(spi_dev, dc);

    // Generate more of these at: https://coolors.co/313715-d16014
    // Pick two and hit Space to generate random pairs until you like what you see
    let palettes = [
        // [Background, Foreground]
        [AOC_BLUE, AOC_GOLD],
        // TODO: These pallets suck. Way too low contrast.
        [Rgb565::from_rgb888(0x1B081D), Rgb565::from_rgb888(0x830C8F)],
        [Rgb565::from_rgb888(0xFFFBFE), Rgb565::from_rgb888(0x7A7D7D)],
        [Rgb565::from_rgb888(0xD16014), Rgb565::from_rgb888(0x313715)],
    ];
    let mut palette = 0;

    let mut framebuffer = Image::new(lcd::WIDTH, lcd::HEIGHT);
    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 17));

    let do_life = false;
    if do_life {
        let mut life = Life::new(lcd::WIDTH as usize / 4, lcd::HEIGHT as usize / 4);
        life.clear_random(&mut rng);

        // Age of the current simulation in steps
        let mut sim_age = 0;
        // MS delay after each step to make sure we have a good framerate
        let per_step_delay_ms = 100;

        loop {
            if sim_age > 0 {
                if sim_age % 20 == 0 {
                    display.idle_mode_on();
                } else if sim_age % 20 == 10 {
                    display.idle_mode_off();
                }
            }

            led.set_high().unwrap();

            // if /* button A is pressed */ {
            //    life.clear_random(&mut rng);
            // }

            // Simulations usually end in a looping and boring state, so periodically clear to random
            if sim_age > {
                (1000 / per_step_delay_ms) /* steps per second */ * 10 /* seconds */
            } {
                life.clear_random(&mut rng);

                sim_age = 0;
                palette += 1;
                palette %= palettes.len();
            }

            let n_updated = life.step();

            if n_updated != 0 {
                for y in 0..life.height() {
                    for x in 0..life.width() {
                        let is_alive = life.get(x, y);

                        // Write a 4x4 big pixel
                        for dy in 0..4 {
                            let yy = 4 * (y as u16) + dy;
                            for dx in 0..4 {
                                let xx = 4 * (x as u16) + dx;
                                framebuffer[(xx, yy)] = palettes[palette][is_alive as usize];
                            }
                        }
                    }
                }
            }

            display.present(&framebuffer);
            sim_age += 1;

            led.set_low().unwrap();
            delay.delay_ms(per_step_delay_ms);
        }
    } else {
        // let rule = 30;
        // let rule = 45;
        // let rule = 89;
        let rule = 90;
        // let rule = 110;
        // let rule = 184;

        let scale = 3;
        let mut sim = Elementry::new(rule, (lcd::HEIGHT / scale) as usize);
        sim.clear_random(&mut rng);
        framebuffer.fill(palettes[palette][0]);

        display.define_vertical_scroll_areas(0, 0);

        // Run our simulation sideways so we can use "vertical" scrolling to move it smoothly.
        loop {
            // Each column is a snapshot of the simulation
            for x in (0..lcd::WIDTH).step_by(scale as usize).rev() {
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
                            framebuffer[((x + dx), scale * y + dy)] =
                                palettes[palette][is_alive as usize];
                        }
                    }
                }

                // Scroll and update the display with our new image
                display.present(&framebuffer);
                delay.delay_ms(10);
            }
        }
    }
}
