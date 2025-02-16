#![allow(unused)]

use std::{
    fs::File,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use image::{imageops, ImageBuffer};
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

    /// Skip the first N frames
    #[arg(long)]
    skip_first: Option<usize>,

    /// Only take N frames
    #[arg(long)]
    n_frames: Option<usize>,

    /// Drop every Nth frame to reduce framerates
    #[arg(long, default_value = "1")]
    frame_rate_div: usize,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct VideoBufferHeader {
    version: u32,
    n_frames: u32,

    // Reserve some space
    reserved: [u32; 31],
}

impl VideoBufferHeader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VideoBufferHeader {
    fn default() -> Self {
        Self {
            version: 1,
            ..bytemuck::Zeroable::zeroed()
        }
    }
}

unsafe impl bytemuck::Pod for VideoBufferHeader {}
unsafe impl bytemuck::Zeroable for VideoBufferHeader {}

fn main() {
    let opts = Opts::parse();

    let pattern = Regex::new("[a-zA-Z_-]+([0-9]+).png").unwrap();

    println!("+ Looking for frames in {:?}", opts.frames_dir.display());
    let mut file_paths = find_files(&opts.frames_dir, pattern);
    println!("+ Found {} frames", file_paths.len());

    if let Some(skip_first) = opts.skip_first {
        println!("+ Skipping first {skip_first} frames");
        file_paths.drain(..skip_first);
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

    if let Some(n_frames) = opts.n_frames {
        println!("+ Truncating to {n_frames} frames");
        let n_frames = file_paths.len().min(n_frames);
        file_paths.drain(n_frames..);
        println!("+ Done (now have {} frames)", file_paths.len());
        println!();
    }

    println!("+ Loading frames");
    let full_frames: Vec<_> = file_paths
        .par_iter()
        .progress()
        .map(|(id, path)| {
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

    println!(
        "+ Dimensions: {} x {}",
        HumanCount(full_frames[0].width() as u64),
        HumanCount(full_frames[0].height() as u64),
    );
    let estimated_pixels: u64 = full_frames
        .iter()
        .inspect(|img| {
            assert_eq!(
                (img.width(), img.height()),
                (full_frames[0].width(), full_frames[0].height())
            )
        })
        .map(memory_estimate)
        .sum();
    let estimated_pixels = HumanCount(estimated_pixels);
    println!("+ Estimated pixel total: {estimated_pixels}");
    println!();

    // These are our resized, adjusted frames!
    println!("+ Resizing video data");
    let frames: Vec<_> = (0..full_frames.len())
        .into_par_iter()
        .progress()
        .map(|i| {
            imageops::resize(
                &full_frames[i],
                out_width,
                out_height,
                imageops::FilterType::Nearest,
            )
        })
        .collect();
    println!("+ Done");
    println!();

    println!("+ Filtering colors");
    let frames: Vec<BitGrid> = frames
        .into_par_iter()
        .progress()
        .map(|img| {
            // .
            let mut bitmap = BitGrid::new(img.width() as usize, img.height() as usize);
            for (x, y, px) in img.enumerate_pixels() {
                let is_white = px.0[0] > 0x80;
                bitmap.set(x as _, y as _, is_white);
            }
            bitmap
        })
        .collect();
    println!("+ Done");
    println!();

    println!("+ NEW dimensions: {out_width} x {out_height}");
    let new_estimate = frames
        .iter()
        .map(|bitmap| bitmap.as_bytes().len() as u64)
        .sum();
    let new_estimate = HumanCount(new_estimate);
    println!("+ Estimated NEW pixel total: {new_estimate}");
    println!();

    // Pack everything into a buffer
    let mut packed_buffer: Vec<u8> = vec![];
    let mut cursor = Cursor::new(&mut packed_buffer);

    // TODO: Real encoding/decoding lib + unit tests
    // let header = VideoBufferHeader {
    //     n_frames: frames.len() as u32,
    //     ..VideoBufferHeader::new()
    // };
    // cursor.write_all(bytemuck::bytes_of(&header)).unwrap();

    for frame in &frames {
        let bytes = frame.as_bytes();
        let len = bytes.len() as u32;
        cursor.write_all(&len.to_le_bytes()).unwrap();
        cursor.write_all(bytes).unwrap();
        assert_eq!(cursor.position() % (4 + len) as u64, 0);
    }

    println!(
        "+ Packed into {} bytes",
        BinaryBytes(packed_buffer.len() as u64)
    );

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

                // let file = File::open(&path).unwrap();

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
        .filter(|(id, seen)| !*seen)
        .filter(|(id, _)| *id > 0 && *id < files.len())
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

fn memory_estimate<P, C>(img: &ImageBuffer<P, C>) -> u64
where
    P: image::Pixel,
    C: std::ops::Deref<Target = [P::Subpixel]>,
{
    let image::flat::SampleLayout {
        width,
        height,
        channels,
        ..
    } = img.sample_layout();
    // width as u64 * height as u64 * channels as u64 * 8 /* bits per channel */
    img.as_raw().len() as u64 * std::mem::size_of::<P::Subpixel>() as u64
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
