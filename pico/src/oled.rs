#![allow(non_upper_case_globals)]

use simulations::BitGrid;

use cortex_m::delay::Delay;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;

pub const WIDTH: u16 = 128;
pub const HEIGHT: u16 = 64;

pub use embedded_graphics::pixelcolor::BinaryColor;

/// A `Display` represents the interface to the Pico-OLED-1.3 SH1107 Display
///
/// The display object owns its own framebuffer of data that may be modified before it is sent
/// to the display driver. Always call [`Display::flush()`] when you're done modifying it to ensure that
/// the physical display has been updated.
pub struct Display<Device, DataCmdPin> {
    driver: OledDriver<Device, DataCmdPin>,
    framebuffer: BitGrid,
}

impl<Device, DataCmdPin> Display<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    pub fn new(driver: OledDriver<Device, DataCmdPin>) -> Self {
        Self {
            driver,
            framebuffer: BitGrid::new(WIDTH as usize, HEIGHT as usize),
        }
    }

    pub fn get(&self, x: i16, y: i16) -> bool {
        self.framebuffer.get(x, y)
    }

    pub fn set(&mut self, x: i16, y: i16, c: bool) {
        self.framebuffer.set(x, y, c);
    }

    pub fn flip(&mut self, x: i16, y: i16) {
        self.framebuffer.flip(x, y);
    }

    pub fn get_color(&self, x: i16, y: i16) -> BinaryColor {
        self.get(x, y).into()
    }

    pub fn set_color(&mut self, x: i16, y: i16, c: BinaryColor) {
        self.set(x, y, c.is_on());
    }

    /// Sets all "pixels" to unset.
    ///
    /// This behaves as if `self.set(x, y, false)` was called for every pixel.
    ///
    /// Whether this is black or white depends on the display's inversion mode. See [`OledDriver::inverse_on`].
    pub fn clear_unset(&mut self) {
        let _ = self.clear(BinaryColor::Off);
    }

    /// Sets all "pixels" to set.
    ///
    /// This behaves as if `self.set(x, y, true)` was called for every pixel.
    ///
    /// Whether this is black or white depends on the display's inversion mode. See [`OledDriver::inverse_on`].
    pub fn clear_set(&mut self) {
        let _ = self.clear(BinaryColor::On);
    }

    pub fn flush(&mut self) {
        let width = WIDTH as i16;
        let height = HEIGHT as i16;

        let bytes = self.framebuffer.as_bytes();
        self.driver.set_page_addr(0); // ???
        for y in 0..height {
            self.driver.set_column_addr(y as u8);
            for x in (0..width).step_by(8).rev() {
                let (idx, _) = self.framebuffer.idx(x, y);
                self.driver.data(bytes[idx].reverse_bits());
            }
        }
    }

    pub fn free(self) -> (Device, DataCmdPin) {
        self.driver.free()
    }
}
impl<Device, DataCmdPin> Dimensions for Display<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    fn bounding_box(&self) -> Rectangle {
        Rectangle {
            top_left: Point::new(0, 0),
            size: Size {
                width: WIDTH as u32,
                height: HEIGHT as u32,
            },
        }
    }
}

impl<Device, DataCmdPin> DrawTarget for Display<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    type Color = BinaryColor;
    type Error = ();

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, c) in pixels.into_iter() {
            if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
                self.set_color(x as i16, y as i16, c);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        if color.is_off() {
            self.framebuffer.as_mut_bytes().fill(0b0000_0000);
        } else {
            self.framebuffer.as_mut_bytes().fill(0b1111_1111);
        }

        Ok(())
    }
}

/// Driver for the `SH1107` OLED Display
pub struct OledDriver<Device, DataCmdPin> {
    /// SPI Device for reading+writing
    dev: Device,

    /// Data/Command Pin to control whether we're writing a command or its data
    dc: DataCmdPin,
}

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
        for y in 0..height {
            self.set_column_addr(y);
            for _y in 0..(width / 8) {
                let row = 0b0000_0000;
                self.data(row);
            }
        }
    }

    pub fn free(self) -> (Device, DataCmdPin) {
        let Self { dev, dc } = self;
        (dev, dc)
    }
}

/// Lower level usage of the display that maps directly to a HW command
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

    fn _nop(&mut self) {
        self.reg(0xE3);
    }

    fn reg(&mut self, reg: u8) {
        self.dc.set_low().unwrap();
        let _ = self.dev.write(&[reg]);

        self.dc.set_high().unwrap();
    }

    fn data(&mut self, byte: u8) {
        self.dc.set_high().unwrap();
        let _ = self.dev.write(&[byte]);
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
