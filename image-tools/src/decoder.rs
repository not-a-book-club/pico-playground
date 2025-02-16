use simulations::BitGrid;

pub struct VideoDecoder<'a> {
    bytes: &'a [u8],
    curr: usize,
}

impl<'a> VideoDecoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, curr: 0 }
    }

    pub fn decode_to(&mut self, frame: &mut BitGrid) {
        //
    }
}
