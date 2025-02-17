use simulations::BitGrid;

use std::io;

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

        for frame in self.frames.drain(..) {
            let compression = FrameCompressionKind::UNCOMPRESSED;

            if compression == FrameCompressionKind::UNCOMPRESSED {
                let bytes = frame.as_bytes();

                let mut chunk = CodecChunkCompressedFrame::new();
                chunk.common.size = bytes.len() as u16;
                chunk.compression = compression;
                chunk.background_set = 0;

                w.write_all(bytemuck::bytes_of(&chunk))?;
                w.write_all(bytes)?;
            } else {
                unimplemented!("Compression kind {compression:?} is not supported");
            }
        }

        Ok(())
    }
}
