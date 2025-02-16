#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused)]

// Note: Encoding DOES require "std"
#[cfg(feature = "encoder")]
pub mod encoder;
pub use encoder::VideoEncoder;

// Note: Decoding DOES NOT require "std"
#[cfg(feature = "decoder")]
pub mod decoder;
pub use decoder::VideoDecoder;
