use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone)]
pub struct BitGrid {
    buf: Vec<u8>,
    width: i16,
    height: i16,
}

impl BitGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let buf = vec![0; ((width + 7) / 8) * height];

        Self {
            buf,
            width: width as i16,
            height: height as i16,
        }
    }

    pub fn width(&self) -> i16 {
        self.width
    }

    pub fn height(&self) -> i16 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.buf.iter().all(|&byte| byte == 0)
    }

    #[track_caller]
    pub fn get(&self, x: i16, y: i16) -> bool {
        let (idx, bit) = self.idx(x, y);
        let mask = 1 << bit;

        (self.buf[idx] & mask) != 0
    }

    #[track_caller]
    pub fn set(&mut self, x: i16, y: i16, elem: bool) -> bool {
        let (idx, bit) = self.idx(x, y);
        let mask = 1 << bit;

        let old = (self.buf[idx] & mask) != 0;

        self.buf[idx] &= !mask;
        self.buf[idx] |= (elem as u8) << bit;

        old
    }

    #[track_caller]
    pub fn flip(&mut self, x: i16, y: i16) -> bool {
        let (idx, bit) = self.idx(x, y);
        let mask = 1 << bit;

        let old = (self.buf[idx] & mask) != 0;

        self.buf[idx] ^= 1 << bit;

        old
    }

    pub fn clear(&mut self) {
        self.as_mut_bytes().fill(0b0000_0000_u8);
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    pub fn idx(&self, mut x: i16, mut y: i16) -> (usize, u8) {
        // Wrap x and y along their axis
        x = (x + self.width()) % self.width();
        y = (y + self.height()) % self.height();

        let idx = (x / 8) + y * ((self.width() + 7) / 8);
        let bit = x % 8;

        (idx as usize, bit as u8)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, clippy::bool_assert_comparison)]
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case::x_is_00(0, (0, 0))]
    #[case::x_is_01(1, (0, 1))]
    #[case::x_is_04(4, (0, 4))]
    #[case::x_is_08(8, (1, 0))]
    #[case::x_is_12(12, (1, 4))]
    #[case::x_is_16(16, (2, 0))]
    #[case::x_is_17(17, (2, 1))]
    // Check wrapping behavior too
    #[case::x_is_00_wrap(0+32, (0, 0))]
    #[case::x_is_01_wrap(1+32, (0, 1))]
    #[case::x_is_04_wrap(4+32, (0, 4))]
    #[case::x_is_08_wrap(8+32, (1, 0))]
    #[case::x_is_12_wrap(12+32, (1, 4))]
    #[case::x_is_16_wrap(16+32, (2, 0))]
    #[case::x_is_17_wrap(17+32, (2, 1))]
    fn check_32x1_idx(#[case] x: i16, #[case] (idx, bit): (usize, u8)) {
        let grid = BitGrid::new(32, 1);
        let y = 0;

        println!("Checking index of x={x}, y={y}");
        let ans = grid.idx(x, y);
        let expected = (idx, bit);
        assert_eq!(
            ans, expected,
            "Flat index of ({x}, {y}) was {ans:?} but should have been {expected:?}"
        );

        // Make sure this doesn't panic
        let _ = grid.get(x, y);
    }

    #[rstest]
    #[case::y_is_00(0, (0, 0))]
    #[case::y_is_01(1, (1, 0))]
    #[case::y_is_04(4, (4, 0))]
    #[case::y_is_08(8, (8, 0))]
    #[case::y_is_12(12, (12, 0))]
    #[case::y_is_16(16, (16, 0))]
    #[case::y_is_17(17, (17, 0))]
    // Check wrapping behavior too
    #[case::y_is_00_wrap(0+32, (0, 0))]
    #[case::y_is_01_wrap(1+32, (1, 0))]
    #[case::y_is_04_wrap(4+32, (4, 0))]
    #[case::y_is_08_wrap(8+32, (8, 0))]
    #[case::y_is_12_wrap(12+32, (12, 0))]
    #[case::y_is_16_wrap(16+32, (16, 0))]
    #[case::y_is_17_wrap(17+32, (17, 0))]
    fn check_1x32_idx(#[case] y: i16, #[case] (idx, bit): (usize, u8)) {
        let grid = BitGrid::new(1, 32);
        let x = 0;

        println!("Checking index of x={x}, y={y}");
        let ans = grid.idx(x, y);
        let expected = (idx, bit);
        assert_eq!(
            ans, expected,
            "Flat index of ({x}, {y}) was {ans:?} but should have been {expected:?}"
        );

        // Make sure this doesn't panic
        let _ = grid.get(x, y);
    }

    #[test]
    fn check_get_set() {
        let mut grid = BitGrid::new(16, 16);
        assert!(grid.is_empty());

        for y in 0..grid.height() {
            for x in 0..grid.width() {
                assert!(grid.is_empty());
                assert_eq!(grid.get(x, y), false);

                grid.set(x, y, true);
                assert!(!grid.is_empty());
                assert_eq!(grid.get(x, y), true);

                grid.set(x, y, false);
                assert_eq!(grid.get(x, y), false);
            }
        }
    }

    #[test]
    fn check_flip() {
        let mut grid = BitGrid::new(16, 16);
        assert!(grid.is_empty());

        for y in 0..grid.height() {
            for x in 0..grid.width() {
                grid.flip(x, y);
            }
        }

        assert_eq!(grid.is_empty(), false);
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                assert_eq!(grid.get(x, y), true);
            }
        }
    }

    #[test]
    fn check_byte_layout() {
        let mut grid = BitGrid::new(16, 16);

        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let (idx, bit) = grid.idx(x, y);
                println!("Checking setting bit at ({x}, {y}) ~= idx={idx}, bit={bit}");
                assert_eq!(grid.get(x, y), false, "Failed to get bit at ({x}, {y})");
                grid.set(x, y, true);
            }
        }

        let byte_len = (grid.width() * grid.height() / 8) as usize;
        assert_eq!(grid.as_bytes().len(), byte_len);
        assert_eq!(grid.as_bytes(), vec![0b1111_1111; byte_len]);
    }
}
