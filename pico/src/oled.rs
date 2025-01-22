#![allow(unused)]
#![allow(non_upper_case_globals)]

use bytemuck::*;
use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{Operation, SpiDevice};

use core::ops::Range;

pub const WIDTH: u16 = 128;
pub const HEIGHT: u16 = 64;

/// Driver for the `SH1107` OLED Display
pub struct OledDriver<Device, DataCmdPin> {
    /// SPI Device for reading+writing
    dev: Device,

    /// Data/Command Pin to control whether we're writing a command or its data
    dc: DataCmdPin,
}

#[allow(dead_code)]
/// High level usage of the OLED Display
impl<Device, DataCmdPin> OledDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    pub fn new<Pin>(dev: Device, dc: DataCmdPin, rst: &mut Pin, delay: &mut Delay) -> Self
    where
        Pin: embedded_hal::digital::OutputPin,
    {
        let mut this = Self { dev, dc };

        this.reset(rst, delay);
        this.init(delay);
        this.clear();

        this
    }

    pub fn clear(&mut self) {
        let width = WIDTH as u8;
        let height = HEIGHT as u8;
        self.set_page_addr(0); // ???
        for y in (0..height).rev() {
            self.set_column_addr(y);
            for i in 0..(width / 8) {
                let row = 0b0000_0000;
                self.data(row);
            }
        }
    }
}

/// Lower level usage of the display that maps directly to a HW command
#[allow(dead_code)]
impl<Device, DataCmdPin> OledDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    pub fn set_column_addr(&mut self, col: u8) {
        self.reg(0x00 + (col & 0x0f));
        self.reg(0x10 + (col >> 4));
    }

    pub fn set_contrast(&mut self, contrast: u8) {
        self.reg(0x81);
        self.reg(contrast);
    }

    pub fn inverse_off(&mut self) {
        self.reg(0xA6);
    }

    pub fn inverse_on(&mut self) {
        self.reg(0xA7);
    }

    pub fn display_off(&mut self) {
        self.reg(0xAE);
    }

    pub fn display_on(&mut self) {
        self.reg(0xAF);
    }

    fn set_page_addr(&mut self, page: u8) {
        debug_assert!(page <= 0b1111);
        self.reg(0xB0 + (page >> 4));
    }

    fn nop(&mut self) {
        self.reg(0xE3);
    }

    fn reg(&mut self, reg: u8) {
        self.dc.set_low().unwrap();
        self.dev.write(&[reg]);

        self.dc.set_high().unwrap();
    }

    fn data(&mut self, byte: u8) {
        self.dc.set_high().unwrap();
        self.dev.write(&[byte]);
    }

    fn reset<Pin>(&mut self, rst: &mut Pin, delay: &mut Delay)
    where
        Pin: embedded_hal::digital::OutputPin,
    {
        delay.delay_ms(100);

        rst.set_high().unwrap();
        delay.delay_ms(100);

        rst.set_low().unwrap();
        delay.delay_ms(100);

        rst.set_high().unwrap();
        delay.delay_ms(100);
    }

    fn init(&mut self, delay: &mut Delay) {
        self.display_off();

        // set lower column address
        self.reg(0x00);
        // set higher column address
        self.reg(0x10);

        // set page address
        self.reg(0xB0);

        // set display start line
        self.reg(0xDC);
        self.reg(0x00);

        // contract control
        self.reg(0x81);
        // 128
        self.reg(0x6F);
        //  Set Memory addressing mode (0x20/0x21)
        self.reg(0x21);

        // set segment remap
        self.reg(0xA0);
        // Com scan direction
        self.reg(0xC0);
        // Disable Entire Display On (0xA4/0xA5)
        self.reg(0xA4);

        self.inverse_off();
        // multiplex ratio
        self.reg(0xA8);
        // duty = 1/64
        self.reg(0x3F);

        // set display offset
        self.reg(0xD3);
        self.reg(0x60);

        // set osc division
        self.reg(0xD5);
        self.reg(0x41);

        // set pre-charge period
        self.reg(0xD9);
        self.reg(0x22);

        // set vcomh
        self.reg(0xDB);
        self.reg(0x35);

        // set charge pump enable
        self.reg(0xAD);
        // Set DC-DC enable (a=0:disable; a=1:enable)
        self.reg(0x8A);

        delay.delay_ms(200);
        self.display_on();
    }
}
