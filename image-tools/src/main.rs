use std::path::{Path, PathBuf};

use clap::Parser;
use image::imageops;
use indicatif::*;
use rayon::prelude::*;
use regex::Regex;
use simulations::BitGrid;

#[derive(Parser, Debug)]
struct Opts {
    /// Path to folder with frames as numbed image files (e.g. "bad_apple_1234.png")
    #[arg(value_name = "DIR", default_value = "frames")]
    frames_dir: PathBuf,

    #[arg(short, long = "output", default_value = "out.bin")]
    output: PathBuf,

    /// Resize the video from the input dimensions.
    /// If only one of --width/--height are provided, aspect ratio is preserved.
    #[arg(long)]
    width: Option<u32>,

    /// Resize the video from the input dimensions.
    /// If only one of --width/--height are provided, aspect ratio is preserved.
    #[arg(long)]
    height: Option<u32>,

    /// Discard the first N frames, and continue taking frames after
    #[arg(long)]
    skip: Option<usize>,

    /// Take N frames and discard the rest
    #[arg(long)]
    take: Option<usize>,

    /// Drop frames to reduce framerates "1" keeps every frame, "2" keeps every other, "3" keeps every 3rd, etc
    #[arg(long, default_value = "1")]
    frame_rate_div: usize,
}

fn main() {
    let opts = Opts::parse();

    let pattern = Regex::new("[a-zA-Z_-]+([0-9]+).png").unwrap();

    println!("+ Looking for frames in {:?}", opts.frames_dir.display());
    let mut file_paths = find_files(&opts.frames_dir, pattern);
    println!("+ Found {} frames", file_paths.len());

    if let Some(skip) = opts.skip {
        println!("+ Skipping first {skip} frames");
        file_paths.drain(..skip);
        println!("+ Done (now have {} frames)", file_paths.len());
        println!();
    }

    if opts.frame_rate_div != 1 {
        println!("+ Dropping 1 in {} frames", opts.frame_rate_div);
        // we could do this better but... meh.
        file_paths = file_paths
            .into_iter()
            .enumerate()
            .filter_map(|(i, path)| {
                if i % opts.frame_rate_div == 0 {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        println!("+ Done (now have {} frames)", file_paths.len());
        println!();
    }

    if let Some(take) = opts.take {
        println!("+ Truncating to {take} frames");
        let take = file_paths.len().min(take);
        file_paths.drain(take..);
        println!("+ Done (now have {} frames)", file_paths.len());
        println!();
    }

    println!("+ Loading {} frames", file_paths.len());
    let full_frames: Vec<_> = file_paths
        .par_iter()
        .progress()
        .map(|(_id, path)| {
            let img = image::open(path).unwrap();
            img.to_luma8()
        })
        .collect();
    println!("+ Done");
    println!();

    let (out_width, out_height) = resolve_dimensions(
        opts.width,
        opts.height,
        full_frames[0].width(),
        full_frames[0].height(),
    );

    // Note: HumanCount doesn't respect format controls, so we `to_string()` and format that.
    println!(
        "+ OLD Dimensions: {:>5} x {:>5}",
        HumanCount(full_frames[0].width() as u64).to_string(),
        HumanCount(full_frames[0].height() as u64).to_string(),
    );
    println!(
        "+ NEW dimensions: {:>5} x {:>5}",
        HumanCount(out_width as u64).to_string(),
        HumanCount(out_height as u64).to_string(),
    );

    // These are our resized, adjusted frames!
    println!("+ Processing frames");
    let frames: Vec<_> = (0..full_frames.len())
        .into_par_iter()
        .progress()
        .map(|i| {
            let img = imageops::resize(
                &full_frames[i],
                out_width,
                out_height,
                imageops::FilterType::Nearest,
            );

            let mut bitmap = BitGrid::new(img.width() as usize, img.height() as usize);
            for (x, y, px) in img.enumerate_pixels() {
                // TODO: Would be nice to dither or something
                let is_white = px.0[0] > 0x80;
                bitmap.set(x as _, y as _, is_white);
            }
            bitmap
        })
        .collect();
    println!("+ Done");
    println!();

    println!("+ Encoding");
    let mut encoder = image_tools::VideoEncoder::new();
    for frame in frames {
        encoder.push(frame);
    }
    let packed_buffer: Vec<u8> = encoder.encode_to_vec().unwrap();
    println!("+ Encoded as {}.", BinaryBytes(packed_buffer.len() as u64));

    let mut output = opts.output;
    if output.is_dir() {
        output.push("out.bin");
    }
    std::fs::write(&output, &packed_buffer).unwrap();
}

fn find_files(dir: &Path, pattern: Regex) -> Vec<(usize, PathBuf)> {
    assert!(dir.is_dir());

    let mut files = vec![];

    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            if let Some(matches) = pattern.captures(&path.as_os_str().to_string_lossy()) {
                let id = matches.get(1).expect("pattern matched but no capture?");
                let id: usize = id.as_str().parse().unwrap();

                files.push((id, path));
            }
        }
    }

    files.sort_by_key(|(id, _f)| *id);

    let mut seen = vec![];
    for (id, _f) in &files {
        if *id >= seen.len() {
            seen.resize_with(*id + 1, || false);
        }

        assert!(!seen[*id]);
        seen[*id] = true;
    }

    let missing: Vec<_> = seen
        .into_iter()
        .enumerate()
        .filter(|(id, seen)| (!*seen) && (*id > 0) && (*id < files.len()))
        .collect();
    if !missing.is_empty() {
        println!(
            "[WARNING] Missing frames from the range 1..{}:",
            files.len()
        );
        for (id, _) in missing {
            println!("    Missing frame {id}");
        }
    }

    files
}

fn resolve_dimensions(
    opts_width: Option<u32>,
    opts_height: Option<u32>,
    img_width: u32,
    img_height: u32,
) -> (u32, u32) {
    match (opts_width, opts_height) {
        // If neither are provided, we don't do resizing
        (None, None) => (img_width, img_height),

        // If both are provided, we use them as-is
        (Some(w), Some(h)) => (w, h),

        // If one was provided, we want to preservethe aspect ratio
        (Some(w), None) => {
            let img_ratio = img_width as f32 / img_height as f32;
            let h = (w as f32 / img_ratio) as u32;
            (w, h)
        }
        (None, Some(h)) => {
            let img_ratio = img_width as f32 / img_height as f32;
            let w = (h as f32 * img_ratio) as u32;
            (w, h)
        }
    }
}

#[cfg(test)]
mod t {
    use crate::resolve_dimensions;

    #[test]
    fn check_resolve_dimensions() {
        assert_eq!(
            resolve_dimensions(None, None, 150, 50),
            (150, 50),
            "Failed to resolve (None, None)"
        );
        assert_eq!(
            resolve_dimensions(Some(300), Some(100), 150, 50),
            (300, 100),
            "Failed to resolve (Some(300), Some(100))"
        );

        assert_eq!(
            resolve_dimensions(None, Some(100), 150, 50),
            (300, 100),
            "Failed to resolve (None, Some(100))"
        );
        assert_eq!(
            resolve_dimensions(Some(300), None, 150, 50),
            (300, 100),
            "Failed to resolve (Some(300), None)"
        );
    }
}
