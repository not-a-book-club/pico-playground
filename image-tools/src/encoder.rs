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

    pub fn encode_to_vec(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = vec![];
        self.encode_to(&mut io::Cursor::new(&mut buf))?;

        Ok(buf)
    }

    pub fn encode_to(&mut self, w: &mut impl io::Write) -> io::Result<()> {
        for frame in self.frames.drain(..) {
            let bytes = frame.as_bytes();
            let len = bytes.len() as u32;
            w.write_all(&len.to_le_bytes()).unwrap();
            w.write_all(bytes).unwrap();
        }

        Ok(())
    }
}
