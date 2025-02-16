use simulations::BitGrid;

#[derive(Clone)]
pub struct VideoDecoder<'a> {
    bytes: &'a [u8],
    curr: usize,
    bitmap: BitGrid,
}

pub struct Frame<'a> {
    pub bitmap: &'a BitGrid,
    pub background_set: bool,
}

impl<'a> VideoDecoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        // TODO: Skip header or something
        let curr = 0;

        let (width, height) = (85, 64);
        let bitmap = BitGrid::new(width, height);

        Self {
            bytes,
            curr: 0,
            bitmap,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.curr == self.bytes.len()
    }

    pub fn reset(&mut self) {
        // TODO: Skip header or something
        self.curr = 0;
        self.bitmap.clear();
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
        let size = *bytemuck::from_bytes::<u32>(self.next(4)?) as usize;
        if size == 0 {
            return None;
        }

        // Move our `bitmap` into the stackframe to convince the borrow checker that
        // 1self.next()` doesn't introduce aliasing.
        // We use 0x0 dimensions to avoid allocating (and should never read from it anyway).
        let mut bitmap = BitGrid::new(0, 0);
        core::mem::swap(&mut bitmap, &mut self.bitmap);

        // Bulk-copy everything
        let bytes = self.next(size)?;
        bitmap.as_mut_bytes().copy_from_slice(bytes);

        // Move it back
        core::mem::swap(&mut bitmap, &mut self.bitmap);

        Some(Frame {
            bitmap: &self.bitmap,
            background_set: true,
        })
    }
}
