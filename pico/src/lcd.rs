use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;

use crate::{Rgb565, OHNO_PINK};

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;

/// Driver for ST7789VW LCD display
pub struct LcdDriver<Device, DataCmdPin> {
    /// SPI Device for reading+writing
    dev: Device,

    /// Data/Command Pin to control whether we're writing a command or its data
    dc: DataCmdPin,
}

/// Basic usage of the LCD Display
impl<Device, DataCmdPin> LcdDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    pub fn new(dev: Device, dc: DataCmdPin) -> Self {
        let mut this = Self { dev, dc };

        this.set_direction_horizontal(true);
        this.init();

        if cfg!(debug_assertions) {
            this.clear_to_color(OHNO_PINK);
        }

        this
    }

    pub fn present(&mut self, image: &crate::Image) {
        // CASET - Column Address Set
        self.cmd8(
            0x2A,
            &[
                // X coordinates
                0x00,
                0x00, // ??
                0x00,
                WIDTH as u8 - 1,
                //
                // Y coordinates
                0x00,
                0x00, // ??
                0x00,
                HEIGHT as u8 - 1,
            ],
        );

        // RAMWR - Memory Write
        self.cmd8(0x2C, image.as_bytes());

        // DISPON - Display On
        self.cmd8(0x29, &[]);
    }

    pub fn clear_to_color(&mut self, color: Rgb565) {
        // CASET - Column Address Set
        self.cmd8(
            0x2A,
            &[
                // X coordinates
                0x00,
                0x00, // ??
                0x00,
                WIDTH as u8 - 1,
                //
                // Y coordinates
                0x00,
                0x00, // ??
                0x00,
                HEIGHT as u8 - 1,
            ],
        );

        // RAMWR - Memory Write
        {
            self.cmd8(0x2C, &[]);

            // Write the clear color one row at a time
            let buf = [color; WIDTH as usize];
            let bytes: &[u8] = bytemuck::cast_slice(&buf);
            for _ in 0..HEIGHT {
                self.dev.write(bytes).unwrap();
            }
        }

        // DISPON - Display On
        self.cmd8(0x29, &[]);
    }

    /// Enable the LCD Idle Mode
    ///
    /// When Idle Mode is on, color bit depth is significant reduced.
    /// - The MSB of each R, G, and B channel is used to select colors
    /// - 8-Color mode frame frequency is applied.
    ///
    /// Idle Mode defaults to off.
    pub fn idle_mode_on(&mut self) {
        // IDMON (39h): Idle mode on
        self.cmd8(0x39, &[]);
    }

    /// Disable the LCD Idle Mode
    ///
    /// Idle Mode defaults to off.
    pub fn idle_mode_off(&mut self) {
        // IDMOFF (38h): Idle Mode Off
        self.cmd8(0x38, &[]);
    }
}

/// Internal methods
impl<Device, DataCmdPin> LcdDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    fn set_direction_horizontal(&mut self, is_horizontal: bool) {
        let memory_access = if is_horizontal { 0x70 } else { 0x0 };

        // MADCTL (36h): Memory Data Access Control
        self.cmd8(0x36, &[memory_access]);
    }

    fn init(&mut self) {
        // COLMOD (3Ah): Interface Pixel Format
        self.cmd8(0x3A, &[0x05]);

        // magic i guess
        self.cmd8(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33]);

        // Gate Control
        self.cmd8(0xB7, &[0x35]);

        // VCOM Setting
        self.cmd8(0xBB, &[0x19]);

        // LCM Control
        self.cmd8(0xC0, &[0x2C]);

        // VDV and VRH Command Enable
        self.cmd8(0xC2, &[0x01]);

        // VRH Set
        self.cmd8(0xC3, &[0x12]);

        // VDV Set
        self.cmd8(0xC4, &[0x20]);

        // Frame Rate Control in Normal Mode
        self.cmd8(0xC6, &[0x0F]);

        // 8Power Control 1
        self.cmd8(0xD0, &[0xA4, 0xA1]);

        // Positive Voltage Gamma Control
        self.cmd8(
            0xE0,
            &[
                0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F, 0x54, 0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23,
            ],
        );

        // Negative Voltage Gamma Control
        self.cmd8(
            0xE1,
            &[
                0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F, 0x44, 0x51, 0x2F, 0x1F, 0x1F, 0x20, 0x23,
            ],
        );

        // Display Inversion On
        self.cmd8(0x21, &[]);

        // Sleep Out
        self.cmd8(0x11, &[]);
        // "-It will be necessary to wait 5msec before sending any new commands"
        // todo

        // Display On
        self.cmd8(0x29, &[]);
    }

    /// Sends `reg` with DC (DataCmdPin) set low, then sets DC high.
    /// Only writes bytes if there are bytes to write, but always sets DC back to low.
    ///
    /// DC is always left high when this function exits!
    // (TODO: Do interrupts here matter?)
    fn cmd8(&mut self, reg: u8, data: &[u8]) {
        self.dc.set_low().unwrap();
        self.dev.write(&[reg]).unwrap();

        self.dc.set_high().unwrap();
        if !data.is_empty() {
            self.dev.write(data).unwrap();
        }
    }
}
