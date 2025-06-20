use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::Text;

use super::{Context, Scene};
use crate::peripherals::SH1107Display;

use crate::alloc::string::*;

pub struct CreditsScene {
    text: String,
    frames_since_input: u32,
    base_y: i32,
    count: u32,
}

impl CreditsScene {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for CreditsScene {
    fn default() -> Self {
        use alloc::format;
        use indoc::indoc;

        let git_ref_full = &env!("VERGEN_GIT_SHA");
        let git_ref = &git_ref_full[..git_ref_full.len().min(8)];
        let git_dirty = env!("VERGEN_GIT_DIRTY");
        let opt_level = env!("VERGEN_CARGO_OPT_LEVEL");
        let build_date = env!("VERGEN_BUILD_DATE");

        let text = format!(
            indoc!(
                /*
                Note: Too many new lines break the thing so use . for empty lines
                Max width is 24 (used in `chunk_lines` below).
                That's this long:
                |----------------------| */
                r#"
                ~~Credits~~~~
                                     <3
                Nerd:
                  Bug Fixing:  C&M
                  Bug Writing: C&M
                  Emotional
                      Support:
                                     <3
                Software:
                  Ref:   {git_ref}
                  Dirty? {git_dirty}
                  OptLv: {opt_level}
                  Built: {build_date}
                                     <3
                "#
            ),
            git_ref = git_ref,
            git_dirty = git_dirty,
            build_date = build_date,
            opt_level = opt_level,
        );
        Self {
            text,
            frames_since_input: 0,
            base_y: 7,
            count: 0,
        }
    }
}

impl Scene for CreditsScene {
    fn update<Device, DataCmdPin>(
        &mut self,
        ctx: &mut Context<'_>,
        display: &mut SH1107Display<Device, DataCmdPin>,
    ) -> bool
    where
        DataCmdPin: embedded_hal::digital::OutputPin,
        Device: embedded_hal::spi::SpiDevice,
    {
        self.count = self.count.saturating_add(1);
        self.frames_since_input = self.frames_since_input.saturating_add(1);

        let btn_a = (self.frames_since_input > 20) && ctx.btn_a;
        let btn_b = (self.frames_since_input > 20) && ctx.btn_b;

        let needs_refresh = true;

        if btn_a || btn_b {
            if btn_a {
                self.base_y -= 2;
            } else if btn_b {
                self.base_y += 2;
            }

            self.base_y = self.base_y.min(32);
        }

        {
            let style_white_border = PrimitiveStyleBuilder::new()
                .stroke_width(1)
                .stroke_color(BinaryColor::On)
                .fill_color(BinaryColor::Off)
                .build();

            let _ = RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(0, 0),
                    Size::new(display.width() as u32, display.height() as u32),
                ),
                Size::new(5, 5),
            )
            .draw_styled(&style_white_border, display);

            let mut y = self.base_y;
            crate::chunk_lines(&self.text, 24, |line| {
                let text = Text::new(
                    line,
                    Point::new(4, y),
                    MonoTextStyle::new(&ascii::FONT_5X8, BinaryColor::On),
                );
                let _ = text.draw(display);
                y += 8
            });

            let lines = y - self.base_y;
            self.base_y = self.base_y.max(-lines + 16);
        }

        // needs_refresh
        needs_refresh
    }
}
