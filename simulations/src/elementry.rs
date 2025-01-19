use alloc::{vec, vec::Vec};

#[derive(Clone)]
pub struct Elementry {
    /// Current state of the simulation
    cells: Vec<u8>,

    /// Shadow copy of cells used when stepping the simulation
    shadow: Vec<u8>,

    rule: u8,
    width: i32,
}

/// Basic Usage
impl Elementry {
    /// Creates a new `Elementry` simulation with the given dimensions where all cells are initially **dead**.
    pub fn new(rule: u8, width: usize) -> Self {
        let cells = vec![0; (width + 7) / 8];
        let shadow = cells.clone();
        let width = width as i32;

        Self {
            cells,
            shadow,
            rule,
            width,
        }
    }

    /// The width of the simulation
    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn cells(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.width()).map(|i| self.get(i))
    }

    pub fn get(&self, x: i32) -> bool {
        // Toggle this enable/disable wrapping along the edge
        let x = (x + self.width()) % self.width();
        if 0 <= x && x < self.width() {
            let x0 = x / 8;
            let x1 = x % 8;
            let mask = 1 << x1;

            (self.cells[x0 as usize] & mask) != 0
        } else {
            false
        }
    }

    pub fn set(&mut self, x: i32, is_alive: bool) {
        // Toggle this enable/disable wrapping along the edge
        let x = (x + self.width()) % self.width();
        if 0 <= x && x < self.width() {
            let x0 = x / 8;
            let x1 = x % 8;
            let mask = 1 << x1;

            self.cells[x0 as usize] &= !mask;
            self.cells[x0 as usize] |= (is_alive as u8) << x1;
        }
    }

    fn set_shadow(&mut self, x: i32, is_alive: bool) {
        if 0 <= x && x < self.width() {
            let x0 = x / 8;
            let x1 = x % 8;
            let mask = 1 << x1;

            self.shadow[x0 as usize] &= !mask;
            self.shadow[x0 as usize] |= (is_alive as u8) << x1;
        }
    }

    /// Steps the simulation once, returning the number of cells updated
    ///
    /// Note: If this ever returns `0`, the simulation will henceforth never change, because nothing is changing anymore.
    pub fn step(&mut self) -> u32 {
        let mut count = 0;

        for x in 0..self.width() {
            let old = self.get(x);
            let c = (self.get(x - 1) as u8) << 2
                | (self.get(x + 0) as u8) << 1
                | (self.get(x + 1) as u8) << 0;
            let mask = 1 << c;

            let is_alive = (self.rule & mask) != 0;
            self.set_shadow(x, is_alive);

            count += (old != is_alive) as u32;
        }

        core::mem::swap(&mut self.cells, &mut self.shadow);

        count
    }

    /// Marks all cells as **dead**
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Marks all cells as **alive**
    pub fn clear_alive(&mut self) {
        self.cells.fill(0xff);
    }
}

/// `std`-only functions
#[cfg(feature = "std")]
impl Elementry {
    pub fn to_ascii(&self) -> String {
        self.cells()
            .map(|is_alive| if is_alive { 'O' } else { '.' })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn check_single_seed() {
        let mut sim = Elementry::new(30, 64);
        sim.set(32, true);

        assert_eq!(sim.to_ascii(), "................................O...............................");
        sim.step();

        assert_eq!(sim.to_ascii(), "...............................OOO..............................");
        sim.step();

        assert_eq!(sim.to_ascii(), "..............................OO..O.............................");
        sim.step();

        assert_eq!(sim.to_ascii(), ".............................OO.OOOO............................");
        sim.step();

        assert_eq!(sim.to_ascii(), "............................OO..O...O...........................");
        sim.step();

        assert_eq!(sim.to_ascii(), "...........................OO.OOOO.OOO..........................");
        sim.step();

        assert_eq!(sim.to_ascii(), "..........................OO..O....O..O.........................");
        sim.step();

        assert_eq!(sim.to_ascii(), ".........................OO.OOOO..OOOOOO........................");
        sim.step();

        assert_eq!(sim.to_ascii(), "........................OO..O...OOO.....O.......................");
        sim.step();

        assert_eq!(sim.to_ascii(), ".......................OO.OOOO.OO..O...OOO......................");
        sim.step();

        assert_eq!(sim.to_ascii(), "......................OO..O....O.OOOO.OO..O.....................");
        sim.step();

        assert_eq!(sim.to_ascii(), ".....................OO.OOOO..OO.O....O.OOOO....................");
        sim.step();

        assert_eq!(sim.to_ascii(), "....................OO..O...OOO..OO..OO.O...O...................");
        sim.step();

        assert_eq!(sim.to_ascii(), "...................OO.OOOO.OO..OOO.OOO..OO.OOO..................");
        sim.step();

        assert_eq!(sim.to_ascii(), "..................OO..O....O.OOO...O..OOO..O..O.................");
        sim.step();

        assert_eq!(sim.to_ascii(), ".................OO.OOOO..OO.O..O.OOOOO..OOOOOOO................");
        sim.step();

        assert_eq!(sim.to_ascii(), "................OO..O...OOO..OOOO.O....OOO......O...............");
        sim.step();

        assert_eq!(sim.to_ascii(), "...............OO.OOOO.OO..OOO....OO..OO..O....OOO..............");
        sim.step();

        assert_eq!(sim.to_ascii(), "..............OO..O....O.OOO..O..OO.OOO.OOOO..OO..O.............");
        sim.step();

        assert_eq!(sim.to_ascii(), ".............OO.OOOO..OO.O..OOOOOO..O...O...OOO.OOOO............");
        sim.step();

        assert_eq!(sim.to_ascii(), "............OO..O...OOO..OOOO.....OOOO.OOO.OO...O...O...........");
        sim.step();

        assert_eq!(sim.to_ascii(), "...........OO.OOOO.OO..OOO...O...OO....O...O.O.OOO.OOO..........");
        sim.step();

        assert_eq!(sim.to_ascii(), "..........OO..O....O.OOO..O.OOO.OO.O..OOO.OO.O.O...O..O.........");
        sim.step();

        assert_eq!(sim.to_ascii(), ".........OO.OOOO..OO.O..OOO.O...O..OOOO...O..O.OO.OOOOOO........");
        sim.step();

        assert_eq!(sim.to_ascii(), "........OO..O...OOO..OOOO...OO.OOOOO...O.OOOOO.O..O.....O.......");
        sim.step();

        assert_eq!(sim.to_ascii(), ".......OO.OOOO.OO..OOO...O.OO..O....O.OO.O.....OOOOO...OOO......");
        sim.step();

        assert_eq!(sim.to_ascii(), "......OO..O....O.OOO..O.OO.O.OOOO..OO.O..OO...OO....O.OO..O.....");
        sim.step();

        assert_eq!(sim.to_ascii(), ".....OO.OOOO..OO.O..OOO.O..O.O...OOO..OOOO.O.OO.O..OO.O.OOOO....");
        sim.step();

        assert_eq!(sim.to_ascii(), "....OO..O...OOO..OOOO...OOOO.OO.OO..OOO....O.O..OOOO..O.O...O...");
        sim.step();

        assert_eq!(sim.to_ascii(), "...OO.OOOO.OO..OOO...O.OO....O..O.OOO..O..OO.OOOO...OOO.OO.OOO..");
        sim.step();

        assert_eq!(sim.to_ascii(), "..OO..O....O.OOO..O.OO.O.O..OOOOO.O..OOOOOO..O...O.OO...O..O..O.");
        sim.step();

        assert_eq!(sim.to_ascii(), ".OO.OOOO..OO.O..OOO.O..O.OOOO.....OOOO.....OOOO.OO.O.O.OOOOOOOOO");
        sim.step();
    }
}
