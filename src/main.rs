use pico_life::Life;

fn main() {
    let mut life = Life::new();
    println!("Life is {} bytes", std::mem::size_of_val(&life));

    // Gliders
    life.write_right_glider(0, 4);
    // life.write_left_glider(12, 0);

    // Spinner
    // life.set_cells([(1, 1), (1, 2), (1, 3)]);

    #[cfg(feature = "std")]
    life.print_ascii();

    for _ in 0..1_000 {
        if life.step() == 0 {
            break;
        }

        #[cfg(feature = "std")]
        life.print_ascii();
    }
}
