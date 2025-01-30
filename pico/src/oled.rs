use simulations::BitGrid;

use cortex_m::delay::Delay;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;

// TODO: This is our specific model and not general.
//       We should consider taking these as either const generics on `Display`, or runtime values
//       The physical dimensions of the screen change across specific parts, but we always have 128x128 bits of memory. I think.
const WIDTH: u16 = 128;
const HEIGHT: u16 = 64;

/// A `Display` represents the interface to the Pico-OLED-1.3 `SH1107` Display
///
/// For lower level control of the display, see [`SH1107Driver`]
///
/// ## The "Framebuffer"
/// The display object owns its own cache of image data, the framebuffer, that may be out of
/// sync with the contents of the display's RAM. Always call [`Display::flush()`] when you're
/// done drawing to ensure the display is up to date.
///
/// The display RAM is populated from the framebuffer, but the framebuffer is never
/// updated by reading back the display RAM.
pub struct Display<Device, DataCmdPin> {
    driver: SH1107Driver<Device, DataCmdPin>,
    framebuffer: BitGrid,
}

impl<Device, DataCmdPin> Display<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    /// Constructs a new Display interface from the hal driver object
    ///
    /// The framebuffer is initialized to all `false` values.
    /// See [`Display::clear_set`] and [`Display::clear_unset`] for quick ways to clear the display.
    pub fn new(driver: SH1107Driver<Device, DataCmdPin>) -> Self {
        Self {
            driver,
            framebuffer: BitGrid::new(WIDTH as usize, HEIGHT as usize),
        }
    }

    /// The width in pixels of the display
    pub const fn width(&self) -> u16 {
        WIDTH
    }

    /// The height in pixels of the display
    pub const fn height(&self) -> u16 {
        HEIGHT
    }

    /// Returns whether the pixel at the given coordinate is set or unset.
    ///
    /// The Display can adjust the set/unset color mapping.
    /// See [`SH1107Driver::inverse_on`] and [`SH1107Driver::inverse_off`] for more details.
    pub fn get(&self, x: i16, y: i16) -> bool {
        self.framebuffer.get(x, y)
    }

    /// Sets the pixel at the given coordinate.
    ///
    /// The Display can adjust the set/unset color mapping.
    /// See [`SH1107Driver::inverse_on`] and [`SH1107Driver::inverse_off`] for more details.
    ///
    /// # Return Value
    /// Returns the previous state of the pixel
    pub fn set(&mut self, x: i16, y: i16, c: bool) -> bool {
        self.framebuffer.set(x, y, c)
    }

    /// Atomically flips the pixel at the given coordiante.
    ///
    /// This is logically equivilent to:
    /// ```rust,no_run
    /// let is_set = display.get(x, y);
    /// display.set(x, y, !is_set);
    /// ```
    ///
    /// The Display can adjust the set/unset color mapping.
    /// See [`SH1107Driver::inverse_on`] and [`SH1107Driver::inverse_off`] for more details.
    ///
    /// # Return Value
    /// Returns the previous state of the pixel
    pub fn flip(&mut self, x: i16, y: i16) -> bool {
        self.framebuffer.flip(x, y)
    }

    /// Sets all "pixels" to unset.
    ///
    /// This behaves as if `self.set(x, y, false)` was called for every pixel.
    ///
    /// Whether this is black or white depends on the display's inversion mode. See [`SH1107Driver::inverse_on`].
    pub fn clear_unset(&mut self) {
        let _ = self.clear(BinaryColor::Off);
    }

    /// Sets all "pixels" to set.
    ///
    /// This behaves as if `self.set(x, y, true)` was called for every pixel.
    ///
    /// Whether this is black or white depends on the display's inversion mode. See [`SH1107Driver::inverse_on`].
    pub fn clear_set(&mut self) {
        let _ = self.clear(BinaryColor::On);
    }

    /// Writes the full state of the framebuffer to the display
    ///
    /// This writes the full state of the framebuffer to the display. After this method returns,
    /// the display should mimic the contents framebuffer.
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

    /// Writes the full state of the given framebuffer to the display
    ///
    /// This acts like [`Display::flush()`] but with the provided `image` instead of the stored framebuffer.
    ///
    /// The contents of the Display framebuffer cache are not changed after this call.
    pub fn copy_image(&mut self, image: &BitGrid) {
        assert_eq!(image.width(), self.framebuffer.width());
        assert_eq!(image.height(), self.framebuffer.height());

        self.framebuffer
            .as_mut_bytes()
            .copy_from_slice(image.as_bytes());
    }

    /// Consume the Display object and recover its hal objects.
    pub fn free(self) -> (Device, DataCmdPin) {
        self.driver.free()
    }
}

// This trait is exposed from `embedded_graphics`
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

// This trait is exposed from `embedded_graphics`
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
                self.set(x as i16, y as i16, c.is_on());
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
pub struct SH1107Driver<Device, DataCmdPin> {
    /// SPI Device for reading+writing
    dev: Device,

    /// Data/Command Pin to control whether we're writing a command or its data
    dc: DataCmdPin,
}

/// Higher level usage of the OLED Display
impl<Device, DataCmdPin> SH1107Driver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    /// Construct a new driver object from its required SPI device and pins.
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

    /// Directly clears the display
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

    /// Consume the Driver object and recover its hal objects.
    pub fn free(self) -> (Device, DataCmdPin) {
        let Self { dev, dc } = self;
        (dev, dc)
    }
}

/// Lower level usage of the display that maps directly to one or more HW command(s)
///
/// These methods are ordered by their command value. For example, [`SH1107Driver::set_column_addr`] comes first
/// because its command code is any of `0x00` through `0x17`.
impl<Device, DataCmdPin> SH1107Driver<Device, DataCmdPin>
where
    Device: SpiDevice,
    DataCmdPin: OutputPin,
{
    // We should keep these doc comments up to date with the datasheet command(s) they use and reference.
    #![deny(missing_docs)]

    /// Sets the column address
    ///
    /// # *1. Set Lower Column Address: (00H - 0FH)*
    /// # *2. Set Higher Column Address: (10H - 17H)*
    /// > Specify column address of display RAM. Divide the column address into
    /// > 4 higher bits and 4 lower bits. Set each of them into successions.
    /// > When the microprocessor repeats to access to the display RAM, the
    /// > column address counter is incremented during each access until
    /// > address 127 is accessed (In page addressing mode).
    /// > The page address is not changed during this time.
    pub fn set_column_addr(&mut self, col: u8) {
        self.reg(0x00 + (col & 0x0f));
        self.reg(0x10 + (col >> 4));
    }

    /// Sets the contrast value
    ///
    /// Bigger is brighter. The default contrast is `128`.
    ///
    /// # *4. Set Contrast Control Register: (Double Bytes Command)*
    /// > The chip has 256 contrast steps from `0x00` to `0xFF`. The segment output
    /// > current increases as the contrast step value increases.
    /// > Segment output current setting:
    /// >
    /// > `ISEG = α/256 * IREF * scale_factor`
    /// >
    /// > Where:
    /// > - `α` is contrast step
    /// > - `IREF` is reference current equals 15.625μA
    /// > - `scale_factor` == 32
    pub fn set_contrast(&mut self, contrast: u8) {
        self.reg(0x81);
        self.reg(contrast);
    }

    /// Disables inverse mode
    ///
    /// When inverse mode is disabled, the image swaps black and white pixels without
    /// affecting display RAM.
    ///
    /// When inverse mode is **OFF**:
    /// - a `true` pixel is white
    /// - a `false` pixel is black
    ///
    /// See: [`SH1107Driver::inverse_on`]
    pub fn inverse_off(&mut self) {
        self.reg(0xA6);
    }

    /// Enables inverse mode
    ///
    /// When inverse mode is enabled, the image swaps black and white pixels without
    /// affecting display RAM.
    ///
    /// When inverse mode is **ON**:
    /// - a `true` pixel is black
    /// - a `false` pixel is white
    ///
    /// See: [`SH1107Driver::inverse_off`]
    pub fn inverse_on(&mut self) {
        self.reg(0xA7);
    }

    /// Turns the display off while keeping other functions active
    ///
    /// # *11. Display OFF/ON: (AEH - AFH)*
    /// > When the display OFF command is executed, power saver mode will be entered.
    /// >
    /// > Sleep mode:
    /// > This mode stops every operation of the OLED display system, and can reduce current consumption nearly to a static current
    /// > value if no access is made from the microprocessor.
    /// >
    /// > The internal status in the sleep mode is as follows:
    /// > 1. Stops the oscillator circuit and DC-DC circuit.
    /// > 2. Stops the OLED drive and outputs Hz as the segment/common driver output.
    /// > 3. Holds the display data and operation mode provided before the start of the sleep mode.
    /// > 4. The MPU can access to the built-in display RAM.
    pub fn display_off(&mut self) {
        self.reg(0xAE);
    }

    /// Turns the display on and resumes normal activity
    ///
    /// See: [`SH1107Driver::display_off`].
    pub fn display_on(&mut self) {
        self.reg(0xAF);
    }

    /// Sets the page address
    ///
    /// # *12. Set Page Address: (B0H - BFH)*
    /// Specify page address to load display RAM data to page address register.
    /// Any RAM data bit can be accessed when its page address and column address
    ///  are specified. The display remains unchanged even when the page address is changed.
    fn set_page_addr(&mut self, page: u8) {
        debug_assert!(page <= 0b1111);
        self.reg(0xB0 + (page >> 4));
    }

    /// No operation
    ///
    /// This does one thing: it sends the "do nothing" command to the device.
    pub fn nop(&mut self) {
        self.reg(0xE3);
    }

    /// Writes a byte over the interface with DC set low
    ///
    /// Leaves DC set high after returning.
    fn reg(&mut self, reg: u8) {
        self.dc.set_low().unwrap();
        let _ = self.dev.write(&[reg]);

        self.dc.set_high().unwrap();
    }

    /// Writes a byte over the interface with DC set high
    ///
    /// Leaves DC set high after returning.
    fn data(&mut self, byte: u8) {
        self.dc.set_high().unwrap();
        let _ = self.dev.write(&[byte]);
    }

    /// Resets the display and leaves it ready for commands
    ///
    /// We must call this before any useful interactions can happen.
    ///
    /// There are ~400ms of delays in this function.
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

    /// First-time initialization with reasonable defaults
    fn init(&mut self, delay: &mut Delay) {
        // TODO: All these reg() calls should be named commands on the driver
        // TODO: We should better cite/document where this order comes from and what's necessary
        //       It came from the sample code which is low-key sus fr fr.

        self.display_off();

        self.set_column_addr(0);

        self.set_page_addr(0);

        // set display start line
        self.reg(0xDC);
        self.reg(0x00);

        self.set_contrast(128);

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
