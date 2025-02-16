#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused)]

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
