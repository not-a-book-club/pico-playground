use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use rand::{rngs::SmallRng, RngCore, SeedableRng};

pub const AOC_BLUE: u32 = 0x0f_0f_23;
pub const AOC_GOLD: u32 = 0xff_ff_66;

fn main() {
    // TODO: Drive these with clap
    let width: usize = 192;
    // With rule 90 + wrapping this rocks!
    // let width: usize = 192 + 3;
    let height: usize = 128;
    let scale = Scale::X8;

    // let width: usize = 720;
    // let height: usize = 3 * width / 2;
    // let scale = Scale::X2;

    // let rule = 30;
    // let rule = 45;
    // let rule = 89;
    // let rule = 90;
    // let rule = 110;
    // let rule = 184;
    let rule: u8 = std::env::args()
        .nth(1)
        .as_deref()
        .unwrap_or("90")
        .parse()
        .unwrap();
    let mut sim = simulations::Elementry::new(rule, width);

    let mut pixels = vec![AOC_BLUE; width * height];
    let mut window = Window::new(
        &format!("👾 Pico Rule {rule}~!"),
        width,
        height,
        WindowOptions {
            title: true,
            resize: true,
            scale,
            scale_mode: ScaleMode::Stretch,

            ..WindowOptions::default()
        },
    )
    .expect("Failed to create a window");

    // TODO: We should query the display's preferred refresh rate instead of assuming 60
    window.set_target_fps(60);

    let palette = [
        AOC_BLUE, // dead
        AOC_GOLD, // alive
    ];

    let mut is_running = true;
    let mut rng = SmallRng::from_seed(core::array::from_fn(|_| 7));

    let mut curr_y = 0;

    // Initial state sets 1 cell
    sim.set(width as i32 / 2, true);
    pixels[width / 2] = palette[1];

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
            sim.clear();
            pixels.fill(palette[0]);

            cells_were_updated = true;
            curr_y = 0;
        } else if window.is_key_pressed(Key::F, KeyRepeat::No) {
            sim.clear_alive();
            pixels.fill(palette[0]);

            cells_were_updated = true;
            curr_y = 0;
        } else if window.is_key_pressed(Key::R, KeyRepeat::No) {
            for x in 0..sim.width() {
                sim.set(x, rng.next_u32() % 2 == 0);
            }
            pixels.fill(palette[0]);

            cells_were_updated = true;
            curr_y = 0;
        } else if window.is_key_pressed(Key::G, KeyRepeat::No) {
            sim.clear();
            pixels.fill(palette[0]);

            sim.set(width as i32 / 2, true);
            pixels[width / 2] = palette[1];

            cells_were_updated = true;
            curr_y = 0;
        }

        if is_running && !cells_were_updated {
            // TODO: We should update every N ms, not every frame.
            let updated = sim.step();

            cells_were_updated |= updated != 0;
            is_running |= updated != 0;
            curr_y += 1;
        }

        // When we reach the bottom of the screen, clear and start at the top again. Ish.
        // TODO: We can do this much better than these giant blits every frame.
        if curr_y >= height as i32 {
            // let num_rows_preserved = height as i32 / 16;
            // let num_rows_preserved = (15 * height as i32) / 16;
            let num_rows_preserved = height as i32 - 1;
            if height as i32 > num_rows_preserved {
                let src_start = ((curr_y - num_rows_preserved) * sim.width()) as usize;
                let src_end = pixels.len();

                pixels.copy_within(src_start..src_end, 0);

                pixels[(src_end - src_start)..].fill(palette[0]);
                cells_were_updated = true;
                curr_y = num_rows_preserved;
            } else {
                pixels.fill(palette[0]);
                curr_y = 0;
            }
        }

        // Copy any updated cells to the framebuffer
        if cells_were_updated {
            // TODO: We could dirty track ranges to speed up low-life simulation frames.
            //       This quickly turns into quad-tree dirty state tracking.

            for x in 0..sim.width() {
                let idx = x + curr_y * sim.width();
                pixels[idx as usize] = palette[sim.get(x) as usize];
            }
        }

        // Present the framebuffer, updated or otherwise, to the screen
        match window.update_with_buffer(&pixels, width, height) {
            Ok(()) => {}
            Err(err) => {
                println!("[ERROR] minifb encountered an error updating the framebuffer: {err:#?}")
            }
        }
    }
}
