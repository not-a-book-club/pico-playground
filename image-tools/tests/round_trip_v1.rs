use image_tools::{decoder::Frame, VideoDecoder, VideoEncoder};
use simulations::BitGrid;

use pretty_assertions::assert_eq;

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
    let mut life = simulations::Life::new(20, 10);
    life.write_left_glider(0, 0);
    encoder.push(life.as_bitgrid().clone());

    let bytes = encoder.encode_to_vec().expect("Failed to encode");

    // ## Decode
    let mut decoder = VideoDecoder::new(&bytes);

    let header = decoder.header();
    dbg!(header);
    assert_eq!(header.n_frames, 1);
    assert_eq!(header.width, 20);
    assert_eq!(header.height, 10);

    // Reserved are always set to 0
    assert_eq!(
        header.reserved,
        vec![0_u32; header.reserved.len()].as_slice()
    );

    dbg!(&decoder);
    assert_eq!(
        decoder.next_frame(),
        Some(Frame {
            id: 1,
            bitmap: life.as_bitgrid(),
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
    assert_eq!(
        decoder.next_frame(),
        Some(Frame {
            id: 1,
            bitmap: &left,
            background_set: false,
        })
    );

    dbg!(&decoder);
    assert_eq!(
        decoder.next_frame(),
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
        assert_eq!(
            decoder.next_frame(),
            Some(Frame {
                id: 1,
                bitmap: &left,
                background_set: false,
            })
        );

        dbg!(&decoder);
        assert_eq!(
            decoder.next_frame(),
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
