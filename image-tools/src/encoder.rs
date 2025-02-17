use simulations::BitGrid;

use std::io;
use std::io::Write;

use crate::codec::*;

#[derive(Clone)]
pub struct VideoEncoder {
    frames: Vec<BitGrid>,

    dims: Option<(i16, i16)>,
}

impl Default for VideoEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoEncoder {
    pub fn new() -> Self {
        Self {
            frames: vec![],
            dims: None,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn push(&mut self, frame: BitGrid) {
        if self.dims.is_none() {
            self.dims = Some(frame.dims());
        }
        self.frames.push(frame);
    }

    pub fn encode_to_vec(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = vec![];
        self.encode_to(&mut io::Cursor::new(&mut buf))?;

        Ok(buf)
    }

    pub fn encode_to(&mut self, w: &mut impl io::Write) -> io::Result<()> {
        // Write out a header, even if we have no frames to encode
        let header: CodecHeader;
        if let Some((width, height)) = self.dims {
            header = CodecHeader::new(self.frame_count(), width as u32, height as u32);
        } else {
            // No data, write a boring header
            header = CodecHeader::new(0, 0, 0);
        }
        w.write_all(bytemuck::bytes_of(&header))?;

        for (id, frame) in self.frames.drain(..).enumerate() {
            let uncompressed_bytes = compress_uncompressed(&frame);
            let runlength_bytes = compress_runlength(&frame);

            if uncompressed_bytes.len() <= runlength_bytes.len() {
                println!(
                    "Frame #{} is smaller uncompressed than RLE: {} vs {}",
                    id + 1,
                    uncompressed_bytes.len(),
                    runlength_bytes.len()
                );
                w.write_all(&uncompressed_bytes)?;
            } else {
                w.write_all(&runlength_bytes)?;
            }
        }

        Ok(())
    }
}

fn compress_uncompressed(frame: &BitGrid) -> Vec<u8> {
    let bytes = frame.as_bytes();

    let mut chunk = CodecChunkCompressedFrame::new(bytes.len() as u16);
    chunk.compression = FrameCompressionKind::UNCOMPRESSED;
    chunk.background_set = 0;

    let mut buf = vec![];
    let mut cursor = io::Cursor::new(&mut buf);

    cursor.write_all(bytemuck::bytes_of(&chunk)).unwrap();
    cursor.write_all(bytes).unwrap();

    buf
}

fn compress_runlength(frame: &BitGrid) -> Vec<u8> {
    let mut runlen_buf = vec![];
    let mut cursor = io::Cursor::new(&mut runlen_buf);

    let mut color: bool = false;
    let mut count: u8 = 0;
    for y in 0..frame.height() {
        for x in 0..frame.width() {
            if frame.get(x, y) != color {
                cursor.write_all(&[count]).unwrap();

                color = frame.get(x, y);
                count = 1;
            } else {
                count += 1;
            }
        }
    }

    // Finish whatever color we were on
    if count > 0 {
        cursor.write_all(&[count]).unwrap();
    }

    let mut chunk = CodecChunkCompressedFrame::new(runlen_buf.len() as u16);
    chunk.compression = FrameCompressionKind::RUN_LENGTH_ENCODING;
    chunk.background_set = 0;

    let mut buf = vec![];
    let mut cursor = io::Cursor::new(&mut buf);

    cursor.write_all(bytemuck::bytes_of(&chunk)).unwrap();
    cursor.write_all(&runlen_buf).unwrap();

    buf
}
