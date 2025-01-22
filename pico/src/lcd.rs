use crate::{Image, Rgb565, OHNO_PINK};

use cortex_m::delay::Delay;
use defmt::Format;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{Operation, SpiDevice};
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

/// Display identification information
#[derive(Copy, Clone, Format, PartialEq, Eq, Default)]
pub struct DisplayId {
    pub manufacturer_id: u8,
    pub version_id: u8,
    pub module_id: u8,
}

#[allow(dead_code)]
/// High level usage of the LCD Display
impl<Device, DataCmdPin> LcdDriver<Device, DataCmdPin>
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
        this.init();

        if cfg!(debug_assertions) {
            this.clear_to_color(OHNO_PINK);
        }

        this
    }

    /// Updates the entire display using `image`
    pub fn present(&mut self, image: &Image<Rgb565>) {
        self.present_range(0..WIDTH, 0..HEIGHT, image);
    }

    /// Updates the region of the display specified by the AABB quad (xs, ys) using the same region from `image`
    // TODO: Is "region matching" like this useful? Maybe should be a dedicated image sent whole-sale
    pub fn present_range(&mut self, xs: Range<u16>, ys: Range<u16>, image: &Image<Rgb565>) {
        self.set_window(xs.start, xs.end - 1, ys.start, ys.end - 1);

        // RAMWR - Memory Write
        self.cmd8(0x2C, image.as_bytes());
    }

    // TODO: Fails dunno why
    // pub fn set_pixel_2x2(&mut self, x: u16, y: u16, color: Rgb565) {
    //     self.set_window(x, x + 1, y, y + 1);

    //     // RAMWR - Memory Write
    //     self.cmd8(0x2C, bytemuck::cast_slice(&[color, color, color, color]));
    // }

    pub fn clear_to_color(&mut self, color: Rgb565) {
        self.set_window(0, WIDTH - 1, 0, HEIGHT - 1);

        // RAMWR - Memory Write
        {
            // Write the clear color one row at a time
            self.cmd8(0x2C, &[]);

            let buf = [color; WIDTH as usize];
            let bytes: &[u8] = bytemuck::cast_slice(&buf);
            for _ in 0..HEIGHT {
                self.dev.write(bytes).unwrap();
            }
        }
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
    ///
    // This is high-level because there are weird uses of this that we do not expose
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
    #[derive(Copy, Clone, Format, PartialEq, Eq)]
    struct MadCtl(u8) : FromStorage, IntoStorage {
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

/// Lower level usage of the display that maps directly to a HW command
#[allow(dead_code)]
impl<Device, DataCmdPin> LcdDriver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    /// Returns 24-bit display identification information
    /// RDDID (04h): Read Display ID
    pub fn id(&mut self) -> DisplayId {
        let mut buf = [0_u8; 4];

        self.dc.set_low().unwrap();
        self.dev.write(&[0x04]).unwrap();

        self.dc.set_high().unwrap();
        self.dev
            .transaction(&mut [Operation::TransferInPlace(&mut buf)])
            .unwrap();

        let [_, manufacturer_id, version_id, module_id] = buf;
        DisplayId {
            manufacturer_id,
            version_id,
            module_id,
        }
    }

    /// RDDMADCTL (0Bh): Read Display MADCTL
    fn read_madctl(&mut self) -> MadCtl {
        let mut buf = [0; 1];

        // TODO: Refactor into a helper function like cmd8
        self.dc.set_low().unwrap();
        self.dev.write(&[0x0B]).unwrap();

        self.dc.set_high().unwrap();
        self.dev
            .transaction(&mut [Operation::TransferInPlace(&mut buf)])
            .unwrap();

        MadCtl::from(u8::from_be_bytes(buf))
    }

    /// INVOFF (20h): Display Inversion Off
    pub fn inversion_off(&mut self) {
        self.cmd8(0x20, &[]);
    }

    /// INVON (21h): Display Inversion On
    pub fn inversion_on(&mut self) {
        self.cmd8(0x21, &[]);
    }

    /// DISPOFF (28h): Display Off
    pub fn display_off(&mut self) {
        self.cmd8(0x28, &[]);
    }

    /// DISPON (29h): Display On
    pub fn display_on(&mut self) {
        self.cmd8(0x29, &[]);
    }

    /// Sets the window for the following (usually) command
    ///
    /// Note that `x_hi` and `y_hi` are **inclusive**.
    ///
    /// CASET - Column Address Set
    fn set_window(&mut self, x_lo: u16, x_hi: u16, y_lo: u16, y_hi: u16) {
        self.cmd8(
            0x2A,
            bytemuck::cast_slice(&[
                x_lo.to_be_bytes(), // X Start
                x_hi.to_be_bytes(), // X End
                y_lo.to_be_bytes(), // Y Start
                y_hi.to_be_bytes(), // Y End
            ]),
        );
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
    ///
    /// VSCSAD (37h): Vertical Scroll Start Address of RAM
    pub fn vertical_scroll_update(&mut self, offset: u16) {
        self.cmd8(
            0x37,
            bytemuck::cast_slice(&[
                offset.to_be_bytes(), // Vertical Scroll Position
            ]),
        )
    }

    /// Disable the LCD Idle Mode
    ///
    /// Idle Mode defaults to off.
    ///
    /// IDMOFF (38h): Idle Mode Off
    pub fn idle_mode_off(&mut self) {
        self.cmd8(0x38, &[]);
    }

    /// Enable the LCD Idle Mode
    ///
    /// When Idle Mode is on, color bit depth is significant reduced.
    /// - The MSB of each R, G, and B channel is used to select colors
    /// - 8-Color mode frame frequency is applied.
    ///
    /// Idle Mode defaults to off.
    ///
    /// IDMON (39h): Idle mode on
    pub fn idle_mode_on(&mut self) {
        self.cmd8(0x39, &[]);
    }

    /// MADCTL (36h): Memory Data Access Control
    fn write_madctl(&mut self, madctl: MadCtl) {
        self.cmd8(0x36, &[madctl.into_storage()]);
    }

    /// WRDISBV (51h): Write Display Brightness
    pub fn write_brightness(&mut self, brightness: u8) {
        self.cmd8(0x51, &[brightness]);
    }

    /// RDDISBV (52h): Read Display Brightness Value
    pub fn read_brightness(&mut self, brightness: u8) -> u8 {
        let mut buf = [0, brightness];

        self.dc.set_low().unwrap();
        self.dev.write(&[0x52]).unwrap();

        self.dc.set_high().unwrap();
        self.dev
            .transaction(&mut [Operation::TransferInPlace(&mut buf)])
            .unwrap();

        buf[1]
    }

    fn init(&mut self) {
        self.write_madctl(
            MadCtl::new()
                .with_mv(1) //
                .with_mx(1), //
        );

        // COLMOD (3Ah): Interface Pixel Format
        self.cmd8(0x3A, &[0x05]);

        // PORCTRL (B2h): Porch Setting
        self.cmd8(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33]);

        // GCTRL (B7h): Gate Control
        self.cmd8(0xB7, &[0x35]);

        // VCOMS (BBh): VCOM
        self.cmd8(0xBB, &[0x19]);

        // LCMCTRL (C0h): LCM Control
        self.cmd8(0xC0, &[0x2C]);

        // VDVVRHEN (C2h): VDV and VRH Command Enable
        self.cmd8(0xC2, &[0x01]);

        // VRHS (C3h): VRH Set
        self.cmd8(0xC3, &[0x12]);

        // VDVS (C4h): VDV Set
        self.cmd8(0xC4, &[0x20]);

        // FRCTRL2 (C6h): Frame Rate Control in Normal Mode
        self.cmd8(0xC6, &[0x0F]);

        // PWCTRL1 (D0h): Power Control 1 .
        self.cmd8(0xD0, &[0xA4, 0xA1]);

        // PVGAMCTRL (E0h): Positive Voltage Gamma Control
        self.cmd8(
            0xE0,
            &[
                0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F, 0x54, 0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23,
            ],
        );

        // NVGAMCTRL (E1h): Negative Voltage Gamma Control
        self.cmd8(
            0xE1,
            &[
                0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F, 0x44, 0x51, 0x2F, 0x1F, 0x1F, 0x20, 0x23,
            ],
        );

        // Display Inversion On
        // We need this ON to have colors behave normally. Seems backwards but it works.
        self.inversion_on();

        // SLPOUT (11h): Sleep Out
        // TODO:
        //      - It will be necessary to wait 5msec before sending any new commands to a display module
        //          following this command to allow time for the supply voltages and clock circuits to stabilize.
        //      - It will be necessary to wait 120msec after sending sleep out command
        //          (when in sleep in mode) before sending an sleep in command.
        self.cmd8(0x11, &[]);

        // Display On
        self.display_on();
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
}
