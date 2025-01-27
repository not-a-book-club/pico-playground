use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

pub const AOC_BLUE: u32 = 0x0f_0f_23;
pub const AOC_GOLD: u32 = 0xff_ff_66;

fn main() {
    // TODO: Drive these with clap
    const WIDTH: usize = 128;
    const HEIGHT: usize = 64;

    let mut pixels = vec![AOC_BLUE; WIDTH * HEIGHT];
    let mut window = Window::new(
        "👾 Pico BitFlipper~!",
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

    let mut life = simulations::BitFlipper::new(WIDTH as i32, HEIGHT as i32);

    let palette = [
        AOC_BLUE, // dead
        AOC_GOLD, // alive
    ];

    let mut is_running = true;

    while window.is_open() {
        if window.is_key_pressed(Key::Escape, KeyRepeat::No)
            || window.is_key_pressed(Key::Q, KeyRepeat::No)
        {
            break;
        }

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            is_running ^= true;
        } else if window.is_key_pressed(Key::E, KeyRepeat::No) {
            life.step_index_forward();
        } else if window.is_key_pressed(Key::Q, KeyRepeat::No) {
            life.step_index_bakward();
        }

        // We don't want to update the framebuffer unless the sim changed.
        let mut cells_were_updated = false;

        if is_running {
            // TODO: We should update every N ms, not every frame.
            life.step();
            cells_were_updated = true;
        }

        // Copy any updated cells to the framebuffer
        if cells_were_updated {
            // TODO: We could dirty track ranges to speed up low-life simulation frames.
            //       This quickly turns into quad-tree dirty state tracking.
            for y in 0..life.bits.height() {
                for x in 0..life.bits.width() {
                    let idx = x + y * WIDTH as i16;
                    pixels[idx as usize] = palette[life.bits.get(x, y) as usize];
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
