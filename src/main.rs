use pico_life::Life;

use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};

fn main() {
    const WIDTH: usize = 128;
    const HEIGHT: usize = WIDTH / 2;

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

    let mut life = Life::new(WIDTH as i32, HEIGHT as i32);
    life.write_right_glider(0, 4);

    pub const AOC_BLUE: u32 = 0x0f_0f_23;
    pub const AOC_GOLD: u32 = 0xff_ff_66;

    let palette = [
        AOC_BLUE, // dead
        AOC_GOLD, // alive
    ];

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        // TODO: We should update every N ms, not every frame.
        let updated = life.step();

        // Copy any updated cells to the framebuffer
        if updated != 0 {
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
