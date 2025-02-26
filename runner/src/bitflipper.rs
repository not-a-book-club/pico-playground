use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use rand::prelude::*;

pub const AOC_BLUE: u32 = 0x0f_0f_23;
pub const AOC_GOLD: u32 = 0xff_ff_66;

fn main() {
    // TODO: Drive these with clap
    const WIDTH: usize = 128;
    const HEIGHT: usize = 64;

    let mut pixels = vec![AOC_BLUE; WIDTH * HEIGHT];
    let mut window = Window::new(
        &format!("👾 Pico BitFlipper~! {WIDTH}x{HEIGHT}"),
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

    let dx = rand::rng().random_range(0..100_000);
    let dy = rand::rng().random_range(0..100_000);
    println!("New Sim: {dx}, {dy}");
    let mut sim = simulations::BitFlipper::new(WIDTH as i32, HEIGHT as i32, dx, dy);

    let palette = [
        AOC_BLUE, // dead
        AOC_GOLD, // alive
    ];

    let mut is_running = true;
    let mut speed: i32 = 1;

    while window.is_open() {
        // We don't want to update the framebuffer unless the sim changed.
        let mut cells_were_updated = false;

        if window.is_key_pressed(Key::Escape, KeyRepeat::No) {
            break;
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            is_running ^= true;
            println!("+ speed={speed}");
            cells_were_updated = true;
        }
        if window.is_key_pressed(Key::C, KeyRepeat::No) {
            sim.bits.clear();
            cells_were_updated = true;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            let dx = rand::rng().random_range(0..100_000);
            let dy = rand::rng().random_range(0..100_000);
            sim = simulations::BitFlipper::new(WIDTH as i32, HEIGHT as i32, dx, dy);
            println!("New Sim: {dx}, {dy}");
            cells_were_updated = true;
        }

        const SPEED_ADJ: f64 = 1.05;
        if window.is_key_pressed(Key::E, KeyRepeat::Yes) {
            if speed > 20 {
                speed = (speed as f64 * SPEED_ADJ) as i32;
            } else if speed < -20 {
                speed = (speed as f64 / SPEED_ADJ) as i32;
            } else {
                speed += 1;
            }
        } else if window.is_key_pressed(Key::Q, KeyRepeat::Yes) {
            if speed > 20 {
                speed = (speed as f64 / SPEED_ADJ) as i32;
            } else if speed < -20 {
                speed = (speed as f64 * SPEED_ADJ) as i32;
            } else {
                speed -= 1;
            }
        }

        if is_running {
            // TODO: We should update every N ms, not every frame.
            for _ in 0..speed.abs() {
                sim.flip_and_advance(speed.signum());
            }
            cells_were_updated = true;
        }

        // Copy any updated cells to the framebuffer
        if cells_were_updated {
            // TODO: We could dirty track ranges to speed up low-life simulation frames.
            //       This quickly turns into quad-tree dirty state tracking.
            for y in 0..sim.bits.height() {
                for x in 0..sim.bits.width() {
                    let idx = (x as usize) + (y as usize) * WIDTH;
                    pixels[idx] = palette[sim.bits.get(x, y) as usize];
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
