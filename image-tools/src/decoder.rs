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
        f.debug_struct("Frame")
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
            .field("bitmap dims", &self.bitmap.dims())
            .field("background_set", &self.background_set)
            .finish()
    }
}

impl<'a> VideoDecoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        let curr = CodecHeader::SIZE;
        let header = CodecHeader::read(&bytes[..curr])
            // This is a fixed size so easy to catch
            .expect("Need more bytes to read CodecHeader");
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
        let chunk = CodecChunkFrame::read(self.next(CodecChunkFrame::SIZE)?)?;

        // Move our `bitmap` into the stackframe to convince the borrow checker that
        //`self.next()` doesn't introduce aliasing.
        // We use 0x0 dimensions to avoid allocating (and should never read from it anyway).
        let mut bitmap = BitGrid::new(0, 0);
        core::mem::swap(&mut bitmap, &mut self.bitmap);

        if chunk.compression == CompressionKind::UNCOMPRESSED {
            // Bulk-copy everything
            let bytes = self.next(chunk.size as usize)?;
            bitmap.as_mut_bytes().copy_from_slice(bytes);
        } else {
            unimplemented!();
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
