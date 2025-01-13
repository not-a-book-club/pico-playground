#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::identity_op, clippy::collapsible_else_if)]

// const ROWS: usize = 64;
// const COLS: usize = 128;

const ROWS: usize = 12;
const COLS: usize = 16;

#[derive(Copy, Clone)]
pub struct Life {
    /// Current state of the simulation
    cells: [[u8; (COLS + 7) / 8]; ROWS],

    /// Shadow copy of cells used when stepping the simulation
    // TODO: I think it's faster to store this in the object rather than each call into step, but need to benchmark
    shadow: [[u8; (COLS + 7) / 8]; ROWS],
}

/// Basic Usage
impl Life {
    /// Creates a new `Life` simulation with all cells initially **dead**.
    pub fn new() -> Self {
        debug_assert_eq!(COLS % 8, 0);

        Self::default()
    }

    /// Checks whether the cell at `(x, y)` is **alive** or **dead**.
    ///
    /// Cells outside the bounds of the simulation are always considered **dead** and
    /// calls to [`get()`](Life::get) with out of bounds coordinates are always **dead**.
    #[track_caller]
    pub fn get(&self, x: usize, y: usize) -> bool {
        if (x >= COLS) || (y >= ROWS) {
            // unreachable!("({x}, {y}) is out of bounds ({COLS}, {ROWS})");
            // Out of bounds reads are dead cells
            return false;
        }

        let x0 = x / 8;
        let x1 = x % 8;
        let mask = 1 << x1;

        (self.cells[y][x0] & mask) != 0
    }

    /// Sets the cell at `(x, y)` to either **alive** or **dead**.
    ///
    /// Cells outside the bounds of the simulation are always considered **dead** and
    /// calls to [`set()`](Life::set) with out of bounds coordinates are **ignored**.
    ///
    /// # Return value
    /// The previous state at this cell is returned.
    ///
    /// # Example
    /// ```rust
    /// # use pico_life::Life;
    /// # fn main() {
    /// let mut life = Life::new();
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
    pub fn set(&mut self, x: usize, y: usize, is_alive: bool) -> bool {
        if (x >= COLS) || (y >= ROWS) {
            unreachable!("({x}, {y}) is out of bounds ({COLS}, {ROWS})");
            // // Out of bounds reads are dead cells
            // return false;
        }

        let x0 = x / 8;
        let x1 = x % 8;
        let mask = 1 << x1;
        // println!("x={x}, y={y}");

        let old = (self.cells[y][x0] & mask) != 0;

        // Clear existing bit
        self.cells[y][x0] &= !mask;
        // Write newe one
        self.cells[y][x0] |= (is_alive as u8) << x1;

        old
    }

    /// Sets each cell generated by `cells` to **alive**.
    ///
    /// This function is calls [`set()`](Life::set) for each item generated by `celles`.
    ///
    /// Cells outside the bounds of the simulation are always considered **dead** and
    /// calls to [`set()`](Life::set) with out of bounds coordinates are **ignored**.
    pub fn set_cells(&mut self, cells: impl IntoIterator<Item = (usize, usize)>) {
        for (x, y) in cells.into_iter() {
            self.set(x, y, true);
        }
    }

    /// Sets a cell in the shadow cells to be **alive** or **dead**.
    ///
    /// This is mostly identical to [`set()`](Life::set) except it operates on [`shadow`](Life::shadow) instead.
    fn set_shadow(&mut self, x: usize, y: usize, is_alive: bool) -> bool {
        if (x >= COLS) || (y >= ROWS) {
            unreachable!("({x}, {y}) is out of bounds ({COLS}, {ROWS})");
        }

        let x0 = x / 8;
        let x1 = x % 8;
        let mask = 1 << x1;

        let old = (self.shadow[y][x0] & mask) != 0;

        // Clear existing bit
        self.shadow[y][x0] &= !mask;
        // Write newe one
        self.shadow[y][x0] |= (is_alive as u8) << x1;

        old
    }

    /// Steps the simulation once, returning the number of cells updated
    ///
    /// Note: If this ever returns `0`, the simulation will henceforth never change, because nothing is changing anymore.
    pub fn step(&mut self) -> u32 {
        let mut count = 0;

        for y in 0..ROWS {
            for x in 0..COLS {
                let mut live_count = 0;

                if x != 0 && y != 0 {
                    live_count += self.get(x - 1, y - 1) as u8;
                }

                if y != 0 {
                    live_count += self.get(x + 0, y - 1) as u8;
                    live_count += self.get(x + 1, y - 1) as u8;
                }

                if x != 0 {
                    live_count += self.get(x - 1, y + 0) as u8;
                    live_count += self.get(x - 1, y + 1) as u8;
                }

                live_count += self.get(x + 1, y + 0) as u8;

                live_count += self.get(x + 0, y + 1) as u8;
                live_count += self.get(x + 1, y + 1) as u8;

                let is_alive = if self.get(x, y) {
                    // Continues to live
                    (live_count == 2) || (live_count == 3)
                } else {
                    // lives, as if by reproduction
                    live_count == 3
                };

                self.set_shadow(x, y, is_alive);

                if self.get(x, y) != is_alive {
                    count += 1;
                }
            }
        }

        self.cells = self.shadow;

        count
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
    pub fn write_right_glider(&mut self, x: usize, y: usize) {
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
    pub fn write_left_glider(&mut self, x: usize, y: usize) {
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
        for y in 0..ROWS {
            for x in 0..COLS {
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

impl Default for Life {
    fn default() -> Self {
        Self {
            cells: [[0; (COLS + 7) / 8]; ROWS],
            shadow: [[0; (COLS + 7) / 8]; ROWS],
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_square_lives() {
        let mut life = Life::new();

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
        let mut life = Life::new();

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
