#![cfg_attr(not(test), no_std)]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if
)]

extern crate alloc;

pub mod image;
pub use image::{Image, Rgb565};

pub mod peripherals;
pub mod scene;

pub const AOC_BLUE: Rgb565 = Rgb565::from_rgb888(0x0f_0f_23);
pub const AOC_GOLD: Rgb565 = Rgb565::from_rgb888(0xff_ff_66);
pub const OHNO_PINK: Rgb565 = Rgb565::new(0xF8_1F);

/// Chunk lines for drawing on a small display
//  TODO: Make it handle non-alpha characters too (and simpler)
pub fn chunk_lines<'a>(text: &'a str, chars_per_line: usize, mut callback: impl FnMut(&'a str)) {
    let mut bytes: &[u8] = text.as_bytes();

    while bytes.len() > chars_per_line {
        // println!("===");
        // println!("  + bytes.len()={}", bytes.len());

        let next_len = bytes.len().min(chars_per_line);
        // println!("  + Searching {} bytes", next_len);
        // println!("    + {:?}", unsafe {
        //     core::str::from_utf8_unchecked(&bytes[..next_len])
        // });

        // Within the next line's worth of bytes, find somewhere to break.
        let break_idx: usize = if let Some(idx) = bytes[..next_len].iter().position(|&b| b == b'\n')
        {
            // println!("  + Found first newline, using idx={idx}");
            idx + 1
        } else if let Some(idx) = bytes[..next_len]
            .iter()
            .rposition(|b| b.is_ascii_whitespace())
        {
            // println!("  + Found last whitespace, using idx={idx}");
            idx + 1
        } else if let Some(idx) = bytes[..next_len]
            .iter()
            .rposition(|b| !b.is_ascii_alphanumeric())
        {
            // println!(
            //     "  + Found NO whitespace, using ugly letter ({:?}) at idx={idx}",
            //     bytes[idx] as char
            // );
            idx + 1
        } else {
            // println!("  + Found nothing worth breaking at, forcing break at {next_len}");
            next_len
        };
        // println!(
        //     "  + Breaking on {:?}",
        //     bytes.get(break_idx).map(|&b| b as char)
        // );

        let line = unsafe { core::str::from_utf8_unchecked(&bytes[..break_idx]) };
        let line = line.trim_end();

        bytes = &bytes[break_idx..];

        if line.trim().is_empty() {
            // empty line? SKIP
            continue;
        }

        for part in line.split("\n") {
            callback(part);
        }
    }

    let line = unsafe { core::str::from_utf8_unchecked(bytes) };
    let line = line.trim_end();
    if !line.trim().is_empty() {
        for part in line.split("\n") {
            callback(part);
        }
    }

    // println!("  + bytes.len()={}", bytes.len());
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use rstest::*;

    use core::time::Duration;

    #[rstest]
    #[case::empty("", 10, [])]
    #[case::short_line("abcd", 10, ["abcd"])]
    #[case::perfect_fit_x2("abcd ABCD", 4, ["abcd", "ABCD"])]
    #[case::perfect_fit_x4("abcd ABCD 1234 6789", 4, ["abcd", "ABCD", "1234", "6789"])]
    #[case::split_on_space("aaaaaa bbbbbb aaaaaa", 10, [
        "aaaaaa",
        "bbbbbb",
        "aaaaaa",
    ])]
    #[case::split_on_too_many_spaces("abcd                  ABCD", 4, ["abcd", "ABCD"])]
    #[case::split_in_word("aaaaaaaaaaaaaaaaaaaa", 7, [
        "aaaaaaa",
        "aaaaaaa",
        "aaaaaa",
    ])]
    #[case::psuedo_debug_fmt(make_sys_info(), 24, [
        "System Info",
        "  chmnfct  = 0xffff",
        "  chpart   = 0xffff",
        "  chrev    = 0xff",
        "  fpga     = true",
        "  asic     = false",
        "  hwgitref = 0xffffffff",
        "........................",
    ])]
    // Check some cases where we SHOULD NOT line break
    #[case::short_alpha("abcd", 100, ["abcd"])]
    #[case::short_alpha_white("abcd defg", 100, ["abcd defg"])]
    #[case::short_alpha_ugly("abcd-defg", 100, ["abcd-defg"])]
    #[timeout(Duration::from_millis(10))]
    fn check_chunk_lines(
        #[case] text: impl AsRef<str>,
        #[case] len: usize,
        #[case] expected: impl IntoIterator<Item = &'static str> + 'static,
    ) {
        let text = text.as_ref();

        println!("Trunc to len={len}");
        println!("text     len={}", text.len());
        println!();
        println!("{text:}");
        println!();
        let expected: Vec<&str> = expected.into_iter().collect();
        for e in &expected {
            assert!(e.len() <= len, "Requesting truncating to {len} chars, but \"expected\" value has line {e:?}, which is {} long!", e.len());
        }
        let mut actual = vec![];

        chunk_lines(text, len, |line| {
            actual.push(line);
        });

        assert_eq!(expected, actual);
    }

    fn make_sys_info() -> String {
        let chip_id_manufacturer = u16::MAX;
        let chip_id_part = u16::MAX;
        let chip_id_revision = u8::MAX;

        let platform_fpga: bool = true;
        let platform_asic: bool = false;

        let gitref_rp2040_spec = u32::MAX;

        format!(
            r#"System Info
  chmnfct  = 0x{chip_id_manufacturer:x}
  chpart   = 0x{chip_id_part:x}
  chrev    = 0x{chip_id_revision:x}
  fpga     = {platform_fpga}
  asic     = {platform_asic}
  hwgitref = 0x{gitref_rp2040_spec:x}
........................
    "#
        )
    }
}
