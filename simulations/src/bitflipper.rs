use crate::BitGrid;

pub struct BitFlipper {
    pub x: i32,
    pub y: i32,
    dir_x: i32,
    dir_y: i32,
    pub bits: BitGrid,
}

impl BitFlipper {
    pub fn new(view_width: i32, view_height: i32, dir_x: i32, dir_y: i32) -> Self {
        let bits = BitGrid::new(view_width as usize, view_height as usize);

        Self {
            bits,
            x: 0,
            y: 0,
            dir_x: dir_x,
            dir_y: dir_y,
        }
    }

    pub fn flip_and_advance(&mut self, dir: i32) {
        if self.x <= 0 {
            self.dir_x = self.dir_x.abs() * dir;
        }

        if self.x >= self.bits.width() as i32 * self.dir_y.abs() {
            self.dir_x = -self.dir_x.abs() * dir;
        }

        if self.y <= 0 {
            self.dir_y = self.dir_y.abs() * dir;
        }

        if self.y >= self.bits.height() as i32 * self.dir_x.abs() {
            self.dir_y = -self.dir_y.abs() * dir;
        }

        self.flip_bit(dir);

        let next_x = Self::next_multiple_of_n_in_direction(self.x, self.dir_y, self.dir_x * dir);
        let next_y = Self::next_multiple_of_n_in_direction(self.y, self.dir_x, self.dir_y * dir);

        let dist_x = next_x - self.x;
        let dist_y = next_y - self.y;

        let move_amount = i32::min(dist_x.abs(), dist_y.abs());

        self.x += move_amount * dir * self.dir_x.signum();
        self.y += move_amount * dir * self.dir_y.signum();
    }

    fn next_multiple_of_n_in_direction(i: i32, n: i32, dir: i32) -> i32 {
        if dir < 0 {
            return -Self::next_multiple_of_n_in_direction(-i, -n, -dir);
        }

        i + n.abs() - Self::positive_modulo(i, n)
    }

    fn positive_modulo(i: i32, n: i32) -> i32 {
        (n.abs() + (i % n.abs())) % n.abs()
    }

    fn flip_bit(&mut self, dir: i32) {
        let x_pixel = (self.x + if self.dir_x * dir >= 0 { 0 } else { -1 }) / self.dir_y.abs();
        let y_pixel = (self.y + if self.dir_y * dir >= 0 { 0 } else { -1 }) / self.dir_x.abs();
        self.bits.flip(x_pixel as i16, y_pixel as i16);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case::simple_1_to_2(1, 1, 1, 2)]
    #[case::simple_2_to_4(2, 2, 2, 4)]
    #[case::simple_3_to_4(3, 2, 2, 4)]
    #[case::simple_3_to_7(3, 7, 2, 7)]
    #[case::simple_9_to_12(9, 3, 3, 12)]
    #[case::simple_9_to_5(9, 5, -1, 5)]
    #[case::simple_negative_9_to_negative_5(-9, -5, 1, -5)]
    #[case::simple_negative_9_to_negative_10(-9, -10, -1, -10)]
    fn test_next_multiple_of_n_in_direction(
        #[case] i: i32,
        #[case] n: i32,
        #[case] dir: i32,
        #[case] expected: i32,
    ) {
        assert_eq!(
            expected,
            BitFlipper::next_multiple_of_n_in_direction(i, n, dir)
        );
    }

    #[test]
    fn test_1_by_1_enabled() {
        let mut expected = BitGrid::new(1, 1);
        expected.flip(1, 1);

        let mut bit_flipper = BitFlipper::new(expected.width() as _, expected.height() as _, 1, 1);
        bit_flipper.flip_and_advance(1);

        let actual = bit_flipper.bits;
        save_test_image("simple_1_by_1", "expected", &expected);
        save_test_image("simple_1_by_1", "actual", &actual);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_32_by_32_simple_diagonal() {
        let mut expected = BitGrid::new(32, 32);

        let mut bit_flipper = BitFlipper::new(expected.width() as _, expected.height() as _, 1, 1);
        for _i in 0..64 {
            bit_flipper.flip_and_advance(1);
        }

        let actual = bit_flipper.bits;
        save_test_image("simple_32_by_32", "expected", &expected);
        save_test_image("simple_32_by_32", "actual", &actual);

        assert_eq!(expected, actual);
    }

    fn save_test_image(scope: &str, label: &str, frame: &BitGrid) {
        use image::{imageops, Luma};
        eprintln!("+ Saving {scope}_{label}:");

        // TODO: Should probably sanitize scope incase it contains "::" or something that makes for bad filenames.

        // Usually the folder with the Cargo.toml
        // let _ = dbg!(std::env::current_dir());

        let out_dir = "./target/test-images";
        std::fs::create_dir_all(out_dir).unwrap();
        let out_path = format!("{out_dir}/{scope}_{label}.png");
        eprintln!(
            "+ Saving to {out_path} ({}x{})",
            frame.width(),
            frame.height()
        );

        let mut img =
            image::GrayImage::from_fn(frame.width() as u32, frame.height() as u32, |x, y| {
                if frame.get(x as _, y as _) {
                    Luma([0xFF])
                } else {
                    Luma([0x00])
                }
            });

        let max_dim = i16::max(frame.width(), frame.height()) as f32;

        if max_dim < 500. {
            let nw = (img.width() as f32 * (500. / max_dim)) as u32;
            let nh = (img.height() as f32 * (500. / max_dim)) as u32;
            img = imageops::resize(&img, nw, nh, imageops::FilterType::Nearest);
        }

        img.save(out_path).unwrap();
    }
}
