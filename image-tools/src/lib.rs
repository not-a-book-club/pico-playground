#![cfg_attr(not(feature = "std"), no_std)]
// We would prefer not to need unsafe code for this. Defer that to bytemuck
// If this is too strict, `#[allow(unsafe_code)]` is a local workaround.
#![deny(unsafe_code)]

// Note: Encoding DOES require "std"
#[cfg(feature = "encoder")]
pub mod encoder;
#[cfg(feature = "encoder")]
pub use encoder::VideoEncoder;

#[cfg(feature = "encoder")]
pub fn encode(frames: impl IntoIterator<Item = simulations::BitGrid>) -> std::io::Result<Vec<u8>> {
    let mut encoder = VideoEncoder::new();
    for frame in frames.into_iter() {
        encoder.push(frame);
    }

    encoder.encode_to_vec()
}

// Note: Decoding DOES NOT require "std"
#[cfg(feature = "decoder")]
pub mod decoder;
#[cfg(feature = "decoder")]
pub use decoder::VideoDecoder;

pub mod codec;
