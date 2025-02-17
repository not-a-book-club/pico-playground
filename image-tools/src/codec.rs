#![allow(unused)]

use core::fmt::Debug;

use bytemuck::{Pod, Zeroable};
use static_assertions::*;

use simulations::BitGrid;

/// `BITVIDEO🍎`
const MAGIC: [u8; 12] = *b"BITVIDEO\xF0\x9F\x8D\x8E";

const VERSION: u32 = 1;

#[derive(Copy, Clone, Pod, Zeroable, PartialEq, Eq)]
#[repr(C)]
pub struct CodecHeader {
    /// `BITVIDEO🍎`
    pub magic: [u8; MAGIC.len()],

    /// Version of the codec that this was saved with
    pub version: u32,

    /// The number of frames (and thus chunks) stored in this blob
    pub n_frames: u32,

    // If you're overflowing a 16-bit int on dimensions, you need a real video codec.
    pub width: u16,
    pub height: u16,

    /// Reserved for future use
    pub reserved: [u32; 26],
}
assert_eq_size!(CodecHeader, [u32; 32]);

impl CodecHeader {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn new(n_frames: usize, width: u32, height: u32) -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            n_frames: n_frames as u32,
            width: width as u16,
            height: height as u16,
            ..Zeroable::zeroed()
        }
    }

    pub fn read(bytes: &[u8]) -> Option<Self> {
        Some(bytemuck::pod_read_unaligned(bytes.get(..Self::SIZE)?))
    }
}

impl Debug for CodecHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CodecHeader")
            .field(
                "magic",
                &core::str::from_utf8(&self.magic).unwrap_or("INVALID_MAGIC"),
            )
            .field("version", &self.version)
            .field("n_frames", &self.n_frames)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
pub struct CompressionKind(u8);

impl CompressionKind {
    /// "Compression" that stores the complete bitmap.
    ///
    /// Frames "compressed" with this method can be read straight into a [`BitGrid`] object:
    /// ```rust
    /// # use simulations::BitGrid;
    /// let frame_bytes = &[0b1111_1111_u8, 0b1111_1111_u8];
    /// let mut bitmap = BitGrid::new(16, 1);
    /// bitmap.as_mut_bytes().copy_from_slice(frame_bytes);
    /// ```
    pub const UNCOMPRESSED: Self = Self(0);

    /// The entire bitmap is encoded as runs of set and unset pixels
    ///
    /// Counts alternate, and always start with unset.
    ///
    /// - A `32` x `32` black image would be encoded like `[32*32]`.
    /// - A `32` x `32` white image would be encoded like `[0, 32*32]`
    /// - A `32` x `32` half-n-half image (left half is black, right half is white) would
    ///     be encoded like `[16, 16, 16, 16, ...]` for a total of 2 per line for 32 lines.
    ///
    /// Note: The codec stores the dimensions for the frames
    pub const RUN_LENGTH_ENCODING: Self = Self(1);
}
assert_eq_size!(CompressionKind, u8);

#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq, Eq)]
#[repr(C)]
pub struct CodecChunkFrame {
    /// The count of bytes immediately after this header that are part of this frame
    pub size: u16,

    /// What kind of compression was used for this frame
    pub compression: CompressionKind,

    /// If this is `0`, the "background" players should use is "unset" aka BLACK.
    /// If this is `1`, the "background" players should use is "set" aka WHITE.
    /// Other values are reserved.
    pub background_set: u8,

    pub reserved: [u32; 1],
}
assert_eq_size!(CodecChunkFrame, [u32; 2]);

impl CodecChunkFrame {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn read(bytes: &[u8]) -> Option<Self> {
        Some(bytemuck::pod_read_unaligned(bytes.get(..Self::SIZE)?))
    }
}
