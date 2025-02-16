use simulations::BitGrid;

use std::io;

pub struct VideoEncoder {
    frames: Vec<BitGrid>,
}

impl Default for VideoEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoEncoder {
    pub fn new() -> Self {
        Self { frames: vec![] }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn push(&mut self, frame: BitGrid) {
        self.frames.push(frame);
    }

    pub fn encode_to_vec(&self) -> io::Result<Vec<u8>> {
        let mut buf = vec![];
        self.encode_to(&mut io::Cursor::new(&mut buf))?;

        Ok(buf)
    }

    pub fn encode_to(&self, w: &mut impl io::Write) -> io::Result<()> {
        //

        w.write_all(b"BAD APPLE")?;

        Ok(())
    }
}
