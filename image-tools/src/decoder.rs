use core::fmt::Debug;

use simulations::BitGrid;

use crate::codec::*;

#[derive(Clone)]
pub struct VideoDecoder<'a> {
    bytes: &'a [u8],
    curr: usize,
    bitmap: BitGrid,
    frame_num: usize,
}

impl Debug for VideoDecoder<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VideoDecoder")
            .field("bytes #", &self.bytes.len())
            .field("curr", &self.curr)
            .field("bitmap dims", &self.bitmap.dims())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    pub id: usize,
    pub bitmap: &'a BitGrid,
    pub background_set: bool,
}

impl Debug for Frame<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Frame")
            .field("id", &self.id)
            .field("background_set", &self.background_set)
            .field("bitmap dims", &self.bitmap.dims())
            .finish()
    }
}

impl<'a> VideoDecoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        let curr = CodecHeader::SIZE;
        let header = CodecHeader::read(&bytes[..curr])
            // This is a fixed size so easy to catch
            .expect("Need more bytes to read CodecHeader");
        if header.version != 2 {
            panic!(
                "Unsupported video codec version: {}, we support version: 2",
                header.version
            );
        }
        let bitmap = BitGrid::new(header.width as _, header.height as _);

        Self {
            bytes,
            curr,
            bitmap,
            frame_num: 0,
        }
    }

    pub fn header(&self) -> CodecHeader {
        CodecHeader::read(&self.bytes[..CodecHeader::SIZE]).unwrap()
    }

    pub fn is_finished(&self) -> bool {
        self.curr == self.bytes.len()
    }

    pub fn reset(&mut self) {
        // self.bytes is unchanged
        self.curr = CodecHeader::SIZE;
        self.bitmap.clear();
        self.frame_num = 0;
    }

    /// Splits off the next `n` bytes, if there are that many, and adjusts `curr``
    fn next(&mut self, n: usize) -> Option<&[u8]> {
        assert!(self.curr <= self.bytes.len());
        if let Some(bytes) = self.bytes[self.curr..].get(..n) {
            self.curr += n;
            Some(bytes)
        } else {
            self.curr = self.bytes.len();
            None
        }
    }

    pub fn next_frame(&mut self) -> Option<Frame> {
        let chunk = CodecChunkCompressedFrame::read(self.next(CodecChunkCompressedFrame::SIZE)?)?;

        // Move our `bitmap` into the stackframe to convince the borrow checker that
        //`self.next()` doesn't introduce aliasing.
        // We use 0x0 dimensions to avoid allocating (and should never read from it anyway).
        let mut bitmap = BitGrid::new(0, 0);
        core::mem::swap(&mut bitmap, &mut self.bitmap);

        let bytes = self.next(chunk.common.size as usize)?;

        if chunk.compression == FrameCompressionKind::UNCOMPRESSED {
            bitmap.clear();
            expand_uncompressed(&mut bitmap, bytes);
        } else if chunk.compression == FrameCompressionKind::RUN_LENGTH_ENCODING {
            bitmap.clear();
            expand_runlength(&mut bitmap, bytes);
        } else {
            unimplemented!("Unsupported compression kind: {:?}", chunk.compression);
        }

        // Move it back
        core::mem::swap(&mut bitmap, &mut self.bitmap);

        self.frame_num += 1;
        Some(Frame {
            id: self.frame_num,
            bitmap: &self.bitmap,
            background_set: false,
        })
    }
}

fn expand_uncompressed(bitmap: &mut BitGrid, in_bytes: &[u8]) {
    // Bulk-copy everything
    bitmap.as_mut_bytes().copy_from_slice(in_bytes);
}

fn expand_runlength(bitmap: &mut BitGrid, in_bytes: &[u8]) {
    let mut x = 0;
    let mut y = 0;

    for pair in in_bytes.chunks(2) {
        let [num_black, num_white] = [pair[0], *pair.get(1).unwrap_or(&0)];

        // Skip black pixels
        for _ in 0..num_black {
            x += 1;
            if x >= bitmap.width() {
                x = 0;
                y += 1;
            }

            // skip
        }

        // Write white pixels
        for _ in 0..num_white {
            bitmap.set(x, y, true);

            x += 1;
            if x >= bitmap.width() {
                x = 0;
                y += 1;
            }
        }
    }
}
