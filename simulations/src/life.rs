use crate::BitGrid;

#[derive(Clone)]
pub struct Life {
    /// Current state of the simulation
    cells: BitGrid,

    /// Shadow copy of cells used when stepping the simulation
    // TODO: I think it's faster to store this in the object rather than each call into step, but need to benchmark
    shadow: BitGrid,

    width: i16,
    height: i16,
}

/// Basic Usage
impl Life {
    /// Creates a new `Life` simulation with the given dimensions where all cells are initially **dead**.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: BitGrid::new(width, height),
            shadow: BitGrid::new(width, height),
            width: width as i16,
            height: height as i16,
        }
    }

    /// The width of the simulation
    pub fn width(&self) -> i16 {
        self.width
    }

    /// The height of the simulation
    pub fn height(&self) -> i16 {
        self.height
    }

    /// Checks whether the cell at `(x, y)` is **alive** or **dead**.
    ///
    /// Out of bounds access wrap around.
    #[track_caller]
    pub fn get(&self, x: i16, y: i16) -> bool {
        self.cells.get(x, y)
    }

    /// Sets the cell at `(x, y)` to either **alive** or **dead**.
    ///
    /// Out of bounds access wrap around.
    ///
    /// # Return value
    /// The previous state at this cell is returned.
    ///
    /// # Example
    /// ```rust
    /// # use simulations::Life;
    /// # fn main() {
    /// let mut life = Life::new(5, 5);
    ///
    /// // All cells start out as dead.
    /// assert_eq!(life.set(0, 0, true), false);
    ///
    /// // The above set this cell to alive, so the next call to Life::set() returns the previous state.
    /// assert_eq!(life.set(0, 0, true), true);   // Write true again, get true back
    /// assert_eq!(life.set(0, 0, false), true);  // Write false once, get true back
    /// assert_eq!(life.set(0, 0, false), false); // Write anything, get false back
    /// # }
    /// ```
    #[track_caller]
    pub fn set(&mut self, x: i16, y: i16, is_alive: bool) -> bool {
        self.cells.set(x, y, is_alive)
    }

    /// Steps the simulation once, returning the number of cells updated
    ///
    /// Note: If this ever returns `0`, the simulation will henceforth never change, because nothing is changing anymore.
    pub fn step(&mut self) -> u32 {
        let mut count = 0;

        for y in 0..self.height() {
            for x in 0..self.width() {
                let mut live_count = 0;

                live_count += self.get(x - 1, y - 1) as u8;
                live_count += self.get(x - 1, y + 0) as u8;
                live_count += self.get(x - 1, y + 1) as u8;

                live_count += self.get(x + 0, y - 1) as u8;
                // Don't count itself
                live_count += self.get(x + 0, y + 1) as u8;

                live_count += self.get(x + 1, y - 1) as u8;
                live_count += self.get(x + 1, y + 0) as u8;
                live_count += self.get(x + 1, y + 1) as u8;

                let is_alive = if self.get(x, y) {
                    // Continues to live
                    (live_count == 2) || (live_count == 3)
                } else {
                    // lives, as if by reproduction
                    live_count == 3
                };

                self.shadow.set(x, y, is_alive);

                if self.get(x, y) != is_alive {
                    count += 1;
                }
            }
        }

        core::mem::swap(&mut self.cells, &mut self.shadow);

        count
    }

    /// Marks all cells as **dead**
    pub fn clear(&mut self) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                self.set(x, y, false);
            }
        }
    }

    /// Set all cells to **alive** or **dead** using the provided rng.
    pub fn clear_random(&mut self, rng: &mut impl rand::Rng) {
        let bytes: &mut [u8] = self.cells.as_mut_bytes();
        for chunk in bytes.chunks_mut(4) {
            let rand_bytes = rng.next_u32().to_le_bytes();
            chunk.copy_from_slice(&rand_bytes[..chunk.len()]);
        }
    }
}

/// Patterns
impl Life {
    /// Writes right-facing glider with its corner at `(x, y)`
    ///
    /// # Cell info
    /// A right-facing glider looks like this:
    /// ```txt
    /// .O.
    /// ..O
    /// OOO
    /// ```
    ///
    /// Where the top left is `(x, y)`.
    #[track_caller]
    pub fn write_right_glider(&mut self, x: i16, y: i16) {
        self.set(x + 0, y + 0, false);
        self.set(x + 1, y + 0, true);
        self.set(x + 2, y + 0, false);

        self.set(x + 0, y + 1, false);
        self.set(x + 1, y + 1, false);
        self.set(x + 2, y + 1, true);

        self.set(x + 0, y + 2, true);
        self.set(x + 1, y + 2, true);
        self.set(x + 2, y + 2, true);
    }

    /// Writes left-facing glider with its corner at `(x, y)`
    ///
    /// # Cell info
    /// A left-facing glider looks like this:
    /// ```txt
    /// .O.
    /// O.
    /// OOO
    /// ```
    ///
    /// Where the top left is `(x, y)`.
    #[track_caller]
    pub fn write_left_glider(&mut self, x: i16, y: i16) {
        self.set(x + 0, y + 0, false);
        self.set(x + 1, y + 0, true);
        self.set(x + 2, y + 0, false);

        self.set(x + 0, y + 1, true);
        self.set(x + 1, y + 1, false);
        self.set(x + 2, y + 1, false);

        self.set(x + 0, y + 2, true);
        self.set(x + 1, y + 2, true);
        self.set(x + 2, y + 2, true);
    }
}

/// `std`-only functions
#[cfg(feature = "std")]
impl Life {
    /// Prints the state of the board to `stdout`
    pub fn print_ascii(&self) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                if self.get(x, y) {
                    print!("O");
                } else {
                    print!(".");
                }
            }
            println!();
        }
        println!();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_square_lives() {
        let mut life = Life::new(5, 5);

        // ....
        // .OO.
        // .OO.
        // ....
        for (x, y) in [
            (1, 1), //
            (2, 1), //
            (1, 2), //
            (2, 2), //
        ] {
            life.set(x, y, true);
        }

        // life.print_ascii();
        let updated = life.step();
        // life.print_ascii();

        // Nothing changes; this pattern is stable
        assert_eq!(updated, 0);
    }

    #[test]
    fn check_spinner_spins() {
        let mut life = Life::new(5, 5);

        // ...
        // .O.
        // .O.
        // .O.
        // ...
        for (x, y) in [
            (1, 1), //
            (1, 2), //
            (1, 3), //
        ] {
            life.set(x, y, true);
        }

        life.print_ascii();
        let updated = life.step();
        life.print_ascii();

        // The spinner should spin - that means the 2 edges set are unset, and the rotated-edges that are unset are set
        // So 4.
        assert_eq!(updated, 4);
    }
}
