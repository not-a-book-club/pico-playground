use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{Operation, SpiDevice};

use crate::{Rgb565, OHNO_PINK};

use proc_bitfield::{bitfield, Bitfield};

use core::ops::Range;

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;

/// Driver for the `ST7789VW` LCD Display
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
        this.init();

        if cfg!(debug_assertions) {
            this.clear_to_color(OHNO_PINK);
        }

        this
    }

    /// Scrolls the entire image with the given offset(?)
    ///
    /// Call this with an increasing offset to animate.
    ///
    /// ## Note
    /// If this offset overlaps with the fixed areas provided by [`Self::define_vertical_scroll_areas`], an "undesirable image will be displayed".
    ///
    /// ## Note
    /// There is no mechanism for scrolling horizontally.
    pub fn vertical_scroll_update(&mut self, offset: u16) {
        // VSCSAD (37h): Vertical Scroll Start Address of RAM
        self.cmd8(
            0x37,
            bytemuck::cast_slice(&[
                offset.to_be_bytes(), // Vertical Scroll Position
            ]),
        )
    }

    /// Set up vertical scrolling
    ///
    /// ## Arguments
    /// - `top_fixed` is the number of lines, from the top of the display, that should not update when scrolling.
    /// - `bot_fixed` is the number of lines, from the bottom of the display, that should not update when scrolling.
    ///
    /// ## Note
    /// Use `0` for both to scroll the entire display.
    ///
    /// ## Note
    /// There is no mechanism for scrolling horizontally.
    pub fn define_vertical_scroll_areas(&mut self, top_fixed: u16, bot_fixed: u16) {
        let tfa = top_fixed.to_be_bytes();
        let vsa = (HEIGHT - top_fixed - bot_fixed).to_be_bytes();
        // The display RAM is sized for 320 tall, but our model only uses 240 of it.
        // As such, we need to used the fixed area controls to exclude the unused memory. It's full of garbage data that isn't typically visible.
        let bfa = (bot_fixed + (320 - HEIGHT)).to_be_bytes();

        // VSCRDEF (33h): Vertical Scrolling Definition
        self.cmd8(
            0x33,
            &[
                tfa[0], tfa[1], // Top Fixed Area
                vsa[0], vsa[1], // Vertical Scroll Area
                bfa[0], bfa[1], // Bottom Fixed Area
            ],
        );
    }

    /// Updates only the AABB quad (xs, ys) from `image` to the display
    pub fn present_range(&mut self, xs: Range<u16>, ys: Range<u16>, image: &crate::Image) {
        // CASET - Column Address Set
        self.cmd8(
            0x2A,
            bytemuck::cast_slice(&[
                xs.start.to_be_bytes(),     // X Start
                (xs.end - 1).to_be_bytes(), // X End
                ys.start.to_be_bytes(),     // Y Start
                (ys.end - 1).to_be_bytes(), // Y End
            ]),
        );

        // RAMWR - Memory Write
        self.cmd8(0x2C, image.as_bytes());

        // DISPON - Display On
        self.cmd8(0x29, &[]);
    }

    pub fn present(&mut self, image: &crate::Image) {
        self.present_range(0..WIDTH, 0..HEIGHT, image);
    }

    pub fn clear_to_color(&mut self, color: Rgb565) {
        let x_s = 0_u16.to_be_bytes();
        let x_e = (WIDTH - 1).to_be_bytes();

        let y_s = 0_u16.to_be_bytes();
        let y_e = (HEIGHT - 1).to_be_bytes();

        // CASET - Column Address Set
        self.cmd8(
            0x2A,
            &[
                x_s[0], x_s[1], // X Start
                x_e[0], x_e[1], // X End
                y_s[0], y_s[1], // Y Start
                y_e[0], y_e[1], // Y End
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

bitfield! {
    /// Memory Data Access Control parameter
    ///
    /// ```txt
    ///     Bit     NAME    DESCRIPTION
    ///     D7      MY      Page Address Order
    ///     D6      MX      Column Address Order
    ///     D5      MV      Page/Column Order
    ///     D4      ML      Line Address Order
    ///     D3      RGB     RGB/BGR Order
    ///     D2      MH      Display Data Latch Order
    /// ```
    #[derive(Copy, Clone, PartialEq, Eq)]
    struct MadCtl(u8) : Debug, FromStorage, IntoStorage {
        /// Page Address Order
        pub my: u8 @ 7..=7,

        /// Column Address Order
        pub mx: u8 @ 6..=6,

        /// Page/Column Order
        pub mv: u8 @ 5..=5,

        /// Line Address Order
        pub ml: u8 @ 4..=4,

        /// RGB/BGR Order
        pub rgb: u8 @ 3..=3,

        /// Display Data Latch Order
        pub mh: u8 @ 2..=2,

        pub reserved: u8 @ 0..2,
    }
}

impl MadCtl {
    fn new() -> Self {
        Self::from_storage(0x0)
    }
}

/// Internal methods
#[allow(dead_code)]
impl<Device, DataCmdPin> LcdDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    /// RDDMADCTL (0Bh): Read Display MADCTL
    fn read_madctl(&mut self) -> MadCtl {
        let mut buf = [0; 1];

        self.dc.set_low().unwrap();
        self.dev.write(&[0x0B]).unwrap();

        self.dc.set_high().unwrap();
        self.dev
            .transaction(&mut [Operation::TransferInPlace(&mut buf)])
            .unwrap();

        MadCtl::from(u8::from_be_bytes(buf))
    }

    /// MADCTL (36h): Memory Data Access Control
    fn write_madctl(&mut self, madctl: MadCtl) {
        self.cmd8(0x36, &madctl.into_storage().to_be_bytes());
    }

    fn init(&mut self) {
        // Was 0x70/0x0, but this looks good too
        self.write_madctl(
            MadCtl::new()
                .with_mv(1) //
                .with_mx(1), //
        );

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
