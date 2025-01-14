use life::Life;

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use rand::{rngs::SmallRng, RngCore, SeedableRng};

fn main() {
    // TODO: Drive these with clap
    // const WIDTH: usize = 192;
    // const HEIGHT: usize = 128;
    const WIDTH: usize = 128;
    const HEIGHT: usize = 96;

    let mut pixels = vec![0_u32; WIDTH * HEIGHT];
    let mut window = Window::new(
        "👾 Pico Life~!",
        WIDTH,
        HEIGHT,
        WindowOptions {
            title: true,
            resize: true,
            scale: Scale::X8,
            scale_mode: ScaleMode::Stretch,

            ..WindowOptions::default()
        },
    )
    .expect("Failed to create a window");

    // TODO: We should query the display's preferred refresh rate instead of assuming 60
    window.set_target_fps(60);

    let mut life = Life::new(WIDTH, HEIGHT);
    if cfg!(debug_assertions) {
        println!("<Life> is {} bytes", std::mem::size_of_val(&life));
    }

    // Step wide enough that gliders don't interfere
    for x in (0..life.width()).step_by(8) {
        life.write_right_glider(x, 4);
    }

    pub const AOC_BLUE: u32 = 0x0f_0f_23;
    pub const AOC_GOLD: u32 = 0xff_ff_66;

    let palette = [
        AOC_BLUE, // dead
        AOC_GOLD, // alive
    ];

    let mut is_running = true;
    let mut rng = SmallRng::from_seed([7; 32]);

    while window.is_open() {
        if window.is_key_pressed(Key::Escape, KeyRepeat::No)
            || window.is_key_pressed(Key::Q, KeyRepeat::No)
        {
            break;
        }

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            is_running ^= true;
        }

        // We don't want to update the framebuffer unless the sim changed.
        let mut cells_were_updated = false;

        if window.is_key_pressed(Key::C, KeyRepeat::No) {
            life.clear();

            cells_were_updated = true;
        } else if window.is_key_pressed(Key::R, KeyRepeat::No) {
            for y in 0..life.height() {
                for x in 0..life.width() {
                    life.set(x, y, rng.next_u32() % 2 == 0);
                }
            }

            cells_were_updated = true;
        } else if window.is_key_pressed(Key::G, KeyRepeat::No) {
            life.clear();

            // Add back just the gliders
            for x in (0..life.width()).step_by(8) {
                life.write_right_glider(x, 4);
            }

            cells_were_updated = true;
        }

        if is_running {
            // TODO: We should update every N ms, not every frame.
            cells_were_updated |= life.step() != 0;
        }

        // Copy any updated cells to the framebuffer
        if cells_were_updated {
            // TODO: We could dirty track ranges to speed up low-life simulation frames.
            //       This quickly turns into quad-tree dirty state tracking.
            for y in 0..life.height() {
                for x in 0..life.width() {
                    let idx = x + y * WIDTH as i32;
                    pixels[idx as usize] = palette[life.get(x, y) as usize];
                }
            }
        }

        // Present the framebuffer, updated or otherwise, to the screen
        match window.update_with_buffer(&pixels, WIDTH, HEIGHT) {
            Ok(()) => {}
            Err(err) => {
                println!("[ERROR] minifb encountered an error updating the framebuffer: {err:#?}")
            }
        }
    }
}
