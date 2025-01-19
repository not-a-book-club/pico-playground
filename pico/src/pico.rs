#![no_std]
#![no_main]
#![allow(clippy::type_complexity)]

extern crate alloc;
use defmt_rtt as _;
use panic_probe as _;

use cortex_m::delay::Delay;
use embedded_alloc::LlffHeap as Heap;
use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle, spi::SpiBus};
use rp_pico::hal;
use rp_pico::hal::{fugit::*, gpio, pac, prelude::*, Spi};
use rp_pico::Pins;

use rand::{rngs::SmallRng, RngCore, SeedableRng};
use simulations::Life;

mod image;
use image::*;

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub const AOC_BLUE: Rgb565 = Rgb565::from_rgb888(0x0f_0f_23);
pub const AOC_GOLD: Rgb565 = Rgb565::from_rgb888(0xff_ff_66);
pub const OHNO_PINK: Rgb565 = Rgb565::new(0xF8_1F);

const WIDTH: u16 = 240;
const HEIGHT: u16 = 240;

#[rp_pico::entry]
fn main() -> ! {
    // Init Heap
    {
        #![allow(static_mut_refs)]
        use core::mem::MaybeUninit;

        // NOTE: The rp2040 has 264 kB of on-chip SRAM
        const HEAP_SIZE: usize = 132 * 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    }

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
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins up according to their function on this particular board
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut life = Life::new(WIDTH as usize / 4, HEIGHT as usize / 4);
    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 7));

    for y in 0..life.height() {
        for x in 0..life.width() {
            life.set(x, y, rng.next_u32() % 2 == 0);
        }
    }

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
    //
    let mut led = pins.led.into_push_pull_output();

    let mut rst = pins.gpio12.into_push_pull_output();
    let mut dc = pins.gpio8.into_push_pull_output();
    let mut cs = pins.gpio9.into_push_pull_output();

    let sclk = pins.gpio10.into_function::<gpio::FunctionSpi>();
    let mosi = pins.gpio11.into_function::<gpio::FunctionSpi>();
    let layout = (mosi, sclk);
    let mut spi = Spi::<_, _, _, 8>::new(pac.SPI1, layout).init(
        &mut pac.RESETS,
        125_000_000u32.Hz(),
        16_000_000u32.Hz(),
        embedded_hal::spi::MODE_0,
    );

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

    init_display(&mut dc, &mut cs, &mut spi);

    let mut framebuffer = Image::new(WIDTH, HEIGHT);
    present(&mut dc, &mut cs, &mut spi, &framebuffer);

    let palette = [AOC_BLUE, AOC_GOLD];

    loop {
        led.set_high().unwrap();

        // if /* button A is pressed */ {
        //     for y in 0..life.height() {
        //         for x in 0..life.width() {
        //             life.set(x, y, rng.next_u32() % 2 == 0);
        //         }
        //     }
        // }

        let n_updated = life.step();
        if n_updated != 0 {
            for y in 0..life.height() {
                for x in 0..life.width() {
                    let is_alive = life.get(x, y);

                    // Write a 4x4 big pixel
                    for dy in 0..4 {
                        let y = 4 * (y as u16) + dy;
                        for dx in 0..4 {
                            let x = 4 * (x as u16) + dx;
                            framebuffer[(x, y)] = palette[is_alive as usize];
                        }
                    }
                }
            }
        }

        // clear_to(&mut dc, &mut cs, &mut spi, 0xF81F_u16);
        present(&mut dc, &mut cs, &mut spi, &framebuffer);

        led.set_low().unwrap();
        delay.delay_ms(10);
    }
}

fn init_display(
    dc: &mut gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    cs: &mut gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    spi: &mut hal::Spi<
        hal::spi::Enabled,
        pac::SPI1,
        (
            hal::gpio::Pin<hal::gpio::bank0::Gpio11, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio10, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
        ),
    >,
) {
    // Set resolution & scanning method of the screen
    // const HORIZONTAL: u8 = 0;
    {
        let memory_access_reg = 0x70;
        // Set the read / write scan direction of the frame memory
        send_cmd(dc, cs, spi, 0x36); //MX, MY, RGB mode
        send_u8(dc, cs, spi, memory_access_reg); //0x08 set RGB
    }

    // Init Reg
    {
        send_cmd(dc, cs, spi, 0x3A);
        send_u8(dc, cs, spi, 0x05);

        send_cmd(dc, cs, spi, 0xB2);
        send_u8(dc, cs, spi, 0x0C);
        send_u8(dc, cs, spi, 0x0C);
        send_u8(dc, cs, spi, 0x00);
        send_u8(dc, cs, spi, 0x33);
        send_u8(dc, cs, spi, 0x33);

        // Gate Control
        send_cmd(dc, cs, spi, 0xB7);
        send_u8(dc, cs, spi, 0x35);

        // VCOM Setting
        send_cmd(dc, cs, spi, 0xBB);
        send_u8(dc, cs, spi, 0x19);

        // LCM Control
        send_cmd(dc, cs, spi, 0xC0);
        send_u8(dc, cs, spi, 0x2C);

        // VDV and VRH Command Enable
        send_cmd(dc, cs, spi, 0xC2);
        send_u8(dc, cs, spi, 0x01);

        // VRH Set
        send_cmd(dc, cs, spi, 0xC3);
        send_u8(dc, cs, spi, 0x12);

        // VDV Set
        send_cmd(dc, cs, spi, 0xC4);
        send_u8(dc, cs, spi, 0x20);

        // Frame Rate Control in Normal Mode
        send_cmd(dc, cs, spi, 0xC6);
        send_u8(dc, cs, spi, 0x0F);

        // 8Power Control 1
        send_cmd(dc, cs, spi, 0xD0);
        send_u8(dc, cs, spi, 0xA4);
        send_u8(dc, cs, spi, 0xA1);

        // Positive Voltage Gamma Control
        send_cmd(dc, cs, spi, 0xE0);
        send_u8(dc, cs, spi, 0xD0);
        send_u8(dc, cs, spi, 0x04);
        send_u8(dc, cs, spi, 0x0D);
        send_u8(dc, cs, spi, 0x11);
        send_u8(dc, cs, spi, 0x13);
        send_u8(dc, cs, spi, 0x2B);
        send_u8(dc, cs, spi, 0x3F);
        send_u8(dc, cs, spi, 0x54);
        send_u8(dc, cs, spi, 0x4C);
        send_u8(dc, cs, spi, 0x18);
        send_u8(dc, cs, spi, 0x0D);
        send_u8(dc, cs, spi, 0x0B);
        send_u8(dc, cs, spi, 0x1F);
        send_u8(dc, cs, spi, 0x23);

        // Negative Voltage Gamma Control
        send_cmd(dc, cs, spi, 0xE1);
        send_u8(dc, cs, spi, 0xD0);
        send_u8(dc, cs, spi, 0x04);
        send_u8(dc, cs, spi, 0x0C);
        send_u8(dc, cs, spi, 0x11);
        send_u8(dc, cs, spi, 0x13);
        send_u8(dc, cs, spi, 0x2C);
        send_u8(dc, cs, spi, 0x3F);
        send_u8(dc, cs, spi, 0x44);
        send_u8(dc, cs, spi, 0x51);
        send_u8(dc, cs, spi, 0x2F);
        send_u8(dc, cs, spi, 0x1F);
        send_u8(dc, cs, spi, 0x1F);
        send_u8(dc, cs, spi, 0x20);
        send_u8(dc, cs, spi, 0x23);

        // Display Inversion On
        send_cmd(dc, cs, spi, 0x21);

        // Sleep Out
        send_cmd(dc, cs, spi, 0x11);

        // Display On
        send_cmd(dc, cs, spi, 0x29);
    }
}

fn send_cmd(
    dc: &mut gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    cs: &mut gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    spi: &mut hal::Spi<
        hal::spi::Enabled,
        pac::SPI1,
        (
            hal::gpio::Pin<hal::gpio::bank0::Gpio11, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio10, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
        ),
    >,
    reg: u8,
) {
    dc.set_low().unwrap();
    cs.set_low().unwrap();
    spi.write(&[reg]).unwrap();
    cs.set_high().unwrap();
}

fn send_u8(
    dc: &mut gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    cs: &mut gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    spi: &mut hal::Spi<
        hal::spi::Enabled,
        pac::SPI1,
        (
            hal::gpio::Pin<hal::gpio::bank0::Gpio11, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio10, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
        ),
    >,
    value: u8,
) {
    dc.set_high().unwrap();
    cs.set_low().unwrap();
    spi.write(&value.to_ne_bytes()).unwrap();
    cs.set_high().unwrap();
}

#[allow(dead_code)]
fn clear_to(
    dc: &mut gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    cs: &mut gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    spi: &mut hal::Spi<
        hal::spi::Enabled,
        pac::SPI1,
        (
            hal::gpio::Pin<hal::gpio::bank0::Gpio11, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio10, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
        ),
    >,
    color: Rgb565,
) {
    {
        // LCD_1IN3_SetWindows(0, 0, LCD_1IN3.WIDTH, LCD_1IN3.HEIGHT);
        // Set Windows
        {
            send_cmd(dc, cs, spi, 0x2A);

            // Set x coordinates
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, 0);
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, WIDTH as u8 - 1);

            // Set x coordinates
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, 0);
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, HEIGHT as u8 - 1);

            send_cmd(dc, cs, spi, 0x2C);
        }

        // Clear the display
        {
            dc.set_high().unwrap();
            cs.set_low().unwrap();

            // NOTE: We need each 16-bit word to be written Big Endian!
            let buf = [color; WIDTH as usize];

            for _ in 0..HEIGHT {
                spi.write(bytemuck::bytes_of(&buf)).unwrap();
            }

            cs.set_high().unwrap();
        }

        // And Display the buffer?
        send_cmd(dc, cs, spi, 0x29);
    }
}

fn present(
    dc: &mut gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    cs: &mut gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionSio<gpio::SioOutput>, gpio::PullDown>,
    spi: &mut hal::Spi<
        hal::spi::Enabled,
        pac::SPI1,
        (
            hal::gpio::Pin<hal::gpio::bank0::Gpio11, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
            hal::gpio::Pin<hal::gpio::bank0::Gpio10, hal::gpio::FunctionSpi, hal::gpio::PullDown>,
        ),
    >,
    buf: &Image,
) {
    {
        // LCD_1IN3_SetWindows(0, 0, LCD_1IN3.WIDTH, LCD_1IN3.HEIGHT);
        // Set Windows
        {
            send_cmd(dc, cs, spi, 0x2A);

            // Set x coordinates
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, 0);
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, WIDTH as u8 - 1);

            // Set x coordinates
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, 0);
            send_u8(dc, cs, spi, 0x00);
            send_u8(dc, cs, spi, HEIGHT as u8 - 1);

            send_cmd(dc, cs, spi, 0x2C);
        }

        // Clear the display
        {
            dc.set_high().unwrap();
            cs.set_low().unwrap();

            spi.write(buf.as_bytes()).unwrap();

            cs.set_high().unwrap();
        }

        // And Display the buffer?
        send_cmd(dc, cs, spi, 0x29);
    }
}
