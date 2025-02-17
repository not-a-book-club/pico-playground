use image::{imageops, Luma};
use image_tools::{decoder::Frame, VideoDecoder, VideoEncoder};
use simulations::BitGrid;

use pretty_assertions::assert_eq;

fn save_test_image(scope: &str, label: &str, frame: &BitGrid) {
    eprintln!("+ Saving {scope}_{label}:");

    // TODO: Should probably sanitize scope incase it contains "::" or something that makes for bad filenames.

    // Usually the folder with the Cargo.toml
    // let _ = dbg!(std::env::current_dir());

    let out_dir = "./target/test-images";
    std::fs::create_dir_all(out_dir).unwrap();
    let out_path = format!("{out_dir}/{scope}_{label}.png");
    eprintln!(
        "+ Saving to {out_path} ({}x{})",
        frame.width(),
        frame.height()
    );

    let mut img = image::GrayImage::from_fn(frame.width() as u32, frame.height() as u32, |x, y| {
        if frame.get(x as _, y as _) {
            Luma([0xFF])
        } else {
            Luma([0x00])
        }
    });

    let max_dim = i16::max(frame.width(), frame.height()) as f32;

    if max_dim < 500. {
        let nw = (img.width() as f32 * (500. / max_dim)) as u32;
        let nh = (img.height() as f32 * (500. / max_dim)) as u32;
        img = imageops::resize(&img, nw, nh, imageops::FilterType::Nearest);
    }

    img.save(out_path).unwrap();
}

#[test]
fn check_zero_frames() {
    // ## Encode
    let mut encoder = VideoEncoder::new();
    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);

    let header = decoder.header();
    dbg!(header);
    assert_eq!(header.n_frames, 0);
    assert_eq!(header.width, 0);
    assert_eq!(header.height, 0);

    // Reserved are always set to 0
    assert_eq!(
        header.reserved,
        vec![0_u32; header.reserved.len()].as_slice()
    );

    // Decoding zero frames should result in no frames
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
}

#[test]
fn check_one_frame() {
    // ## Encode
    let mut encoder = VideoEncoder::new();

    // Encode a lone glider
    let mut life = simulations::Life::new(9, 4);
    life.write_left_glider(0, 0);
    let left: BitGrid = life.as_bitgrid().clone();
    save_test_image("check_one_frame", "left_good", &left);

    encoder.push(left.clone());

    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);
    dbg!(&decoder);

    let header = decoder.header();
    dbg!(header);
    assert_eq!(header.n_frames, 1);
    assert_eq!(header.width, life.width() as _);
    assert_eq!(header.height, life.height() as _);

    // Reserved are always set to 0
    assert_eq!(
        header.reserved,
        vec![0_u32; header.reserved.len()].as_slice()
    );

    let frame = decoder.next_frame();
    if let Some(frame) = &frame {
        save_test_image("check_one_frame", "left", frame.bitmap);
    }
    assert_eq!(
        frame,
        Some(Frame {
            id: 1,
            bitmap: &left,
            background_set: false,
        })
    );

    // No more frames
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
}

#[test]
fn check_two_frames() {
    // ## Encode
    let mut encoder = VideoEncoder::new();

    let mut life = simulations::Life::new(20, 10);
    // LEFT
    life.write_left_glider(0, 0);
    let left: BitGrid = life.as_bitgrid().clone();
    save_test_image("check_two_frames", "left_good", &left);

    // RIGHT
    life.clear();
    life.write_right_glider(0, 0);
    let right: BitGrid = life.as_bitgrid().clone();
    save_test_image("check_two_frames", "right_good", &right);

    // Done with this now!
    drop(life);

    encoder.push(left.clone());
    encoder.push(right.clone());

    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);

    let header = decoder.header();
    dbg!(header);
    assert_eq!(header.n_frames, 2);
    assert_eq!(header.width, 20);
    assert_eq!(header.height, 10);

    // Reserved are always set to 0
    assert_eq!(
        header.reserved,
        vec![0_u32; header.reserved.len()].as_slice()
    );

    dbg!(&decoder);
    let frame = decoder.next_frame();
    if let Some(frame) = &frame {
        save_test_image("check_two_frames", "left", frame.bitmap);
    }
    assert_eq!(
        frame,
        Some(Frame {
            id: 1,
            bitmap: &left,
            background_set: false,
        })
    );

    dbg!(&decoder);
    let frame = decoder.next_frame();
    if let Some(frame) = &frame {
        save_test_image("check_two_frames", "right", frame.bitmap);
    }
    assert_eq!(
        frame,
        Some(Frame {
            id: 2,
            bitmap: &right,
            background_set: false,
        })
    );

    // No more frames
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
}

#[test]
fn check_two_frames_with_reset() {
    // ## Encode
    let mut encoder = VideoEncoder::new();

    let mut life = simulations::Life::new(20, 10);
    // LEFT
    life.write_left_glider(0, 0);
    let left: BitGrid = life.as_bitgrid().clone();

    // RIGHT
    life.clear();
    life.write_right_glider(0, 0);
    let right: BitGrid = life.as_bitgrid().clone();

    // Done with this now!
    drop(life);

    encoder.push(left.clone());
    encoder.push(right.clone());

    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);

    for i in 0..2 {
        println!("i={i}");

        let header = decoder.header();
        dbg!(header);
        assert_eq!(header.n_frames, 2);
        assert_eq!(header.width, 20);
        assert_eq!(header.height, 10);

        // Reserved are always set to 0
        assert_eq!(
            header.reserved,
            vec![0_u32; header.reserved.len()].as_slice()
        );

        dbg!(&decoder);
        let frame = decoder.next_frame();
        if let Some(frame) = &frame {
            save_test_image(module_path!(), "1", frame.bitmap);
        }
        assert_eq!(
            frame,
            Some(Frame {
                id: 1,
                bitmap: &left,
                background_set: false,
            })
        );

        dbg!(&decoder);
        let frame = decoder.next_frame();
        if let Some(frame) = &frame {
            save_test_image(module_path!(), "2", frame.bitmap);
        }
        assert_eq!(
            frame,
            Some(Frame {
                id: 2,
                bitmap: &right,
                background_set: false,
            })
        );

        // No more frames
        assert_eq!(decoder.next_frame(), None);
        assert_eq!(decoder.next_frame(), None);
        assert_eq!(decoder.next_frame(), None);
        assert_eq!(decoder.next_frame(), None);

        decoder.reset();
    }
}

#[test]
fn check_one_frame_runlength_1() {
    // ## Encode
    let mut encoder = VideoEncoder::new();

    // Encode something simple
    let mut bitmap = BitGrid::new(2, 2);
    // Should look like:
    //    ..
    //    .#
    bitmap.set(1, 1, true);
    save_test_image("check_one_frame_runlength_1", "2x2_good", &bitmap);

    encoder.push(bitmap.clone());

    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);
    dbg!(&decoder);

    let frame = decoder.next_frame();
    if let Some(frame) = &frame {
        save_test_image("check_one_frame_runlength_1", "2x2", frame.bitmap);
        assert_eq!(frame.bitmap.as_bytes(), bitmap.as_bytes());
    }

    assert_eq!(
        frame,
        Some(Frame {
            id: 1,
            bitmap: &bitmap,
            background_set: false,
        })
    );

    // No more frames
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
    assert_eq!(decoder.next_frame(), None);
}
