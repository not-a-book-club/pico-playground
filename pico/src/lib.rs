#![cfg_attr(not(test), no_std)]
#![allow(
    clippy::identity_op,
    clippy::collapsible_if,
    clippy::collapsible_else_if
)]

extern crate alloc;

pub mod image;
pub use image::{Image, Rgb565};

pub mod lcd;
pub use lcd::LcdDriver;

pub mod oled;

pub mod scene;

pub const AOC_BLUE: Rgb565 = Rgb565::from_rgb888(0x0f_0f_23);
pub const AOC_GOLD: Rgb565 = Rgb565::from_rgb888(0xff_ff_66);
pub const OHNO_PINK: Rgb565 = Rgb565::new(0xF8_1F);

/// Chunk lines for drawing on a small display
//  TODO: Make it handle non-alpha characters too (and simpler)
pub fn chunk_lines<'a>(text: &'a str, chars_per_line: usize, mut callback: impl FnMut(&'a str)) {
    let mut last_space = None;
    let mut bytes: &[u8] = text.as_bytes();

    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b' ' {
            last_space = Some(i);

            // Skip the rest of the spaces here
            while bytes.get(i) == Some(&b' ') {
                i += 1;
            }
        }
        debug_assert!(bytes.get(i) != Some(&b' '));

        if i < chars_per_line {
            // Go find more
            i += 1;
            continue;
        }

        let line: &[u8];
        if let Some(ls) = last_space.take() {
            line = &bytes[..ls];

            // Note: We expclude the last space (b' ') with this +1:
            bytes = &bytes[(ls + 1)..];
            i -= ls;
        } else {
            // We've found a full line but haven't found a space?
            // Hard chop to make it fit. Sorry.
            line = &bytes[..chars_per_line];

            // Note: We DON'T want to exclude the character where we are, so no +1 like above.
            bytes = &bytes[chars_per_line..];
            i -= chars_per_line;
        }

        i += 1;

        let line = unsafe { core::str::from_utf8_unchecked(line) };
        // Don't waste anyone's time
        if line.is_empty() || line.trim().is_empty() {
            continue;
        }

        callback(line);
    }

    i = 0;
    // Skip the rest of the spaces here
    while bytes.get(i) == Some(&b' ') {
        i += 1;
    }

    let line = unsafe { core::str::from_utf8_unchecked(&bytes[i..]) };
    // Don't waste anyone's time
    if line.is_empty() || line.trim().is_empty() {
        return;
    }

    callback(line);
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
    #[timeout(Duration::from_millis(750))]
    fn check_chunk_lines(
        #[case] text: &str,
        #[case] len: usize,
        #[case] expected: impl IntoIterator<Item = &'static str> + 'static,
    ) {
        println!("Truncating to len={len}");
        let expected: Vec<&str> = expected.into_iter().collect();
        let mut actual = vec![];

        chunk_lines(text, len, |line| {
            actual.push(line);
        });

        assert_eq!(expected, actual);
    }
}
