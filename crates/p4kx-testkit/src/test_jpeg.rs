/// Returns a baseline greyscale JPEG of `width` x `height` pixels that all have the value
/// `gray`, written with a quantisation table of ones so it decodes exactly.
#[must_use]
pub fn gray_jpeg(width: u16, height: u16, gray: u8) -> Vec<u8> {
    let dc = 8 * (i32::from(gray) - 128);
    let category = 32 - dc.unsigned_abs().leading_zeros();
    let category = u8::try_from(category).unwrap_or(0);
    let mut jpeg = vec![0xFF, 0xD8];
    segment(&mut jpeg, 0xDB, &[[0_u8].as_slice(), &[1; 64]].concat());
    let mut frame = vec![8];
    frame.extend_from_slice(&height.to_be_bytes());
    frame.extend_from_slice(&width.to_be_bytes());
    frame.extend_from_slice(&[1, 1, 0x11, 0]);
    segment(&mut jpeg, 0xC0, &frame);
    let dc_symbols = if category == 0 { [0, 1] } else { [0, category] };
    let mut dc_table = vec![0x00, 0, 2];
    dc_table.extend_from_slice(&[0; 14]);
    dc_table.extend_from_slice(&dc_symbols);
    segment(&mut jpeg, 0xC4, &dc_table);
    let mut ac_table = vec![0x10, 0, 1];
    ac_table.extend_from_slice(&[0; 14]);
    ac_table.push(0x00);
    segment(&mut jpeg, 0xC4, &ac_table);
    segment(&mut jpeg, 0xDA, &[1, 1, 0x00, 0, 63, 0]);
    let blocks = usize::from(width.div_ceil(8)) * usize::from(height.div_ceil(8));
    let mut scan = ScanWriter::default();
    for block in 0..blocks {
        if block == 0 && category > 0 {
            scan.bits(0b01, 2);
            let value = if dc < 0 { dc - 1 } else { dc };
            scan.bits(
                value.cast_unsigned() & ((1 << category) - 1),
                u32::from(category),
            );
        } else {
            scan.bits(0b00, 2);
        }
        scan.bits(0b00, 2);
    }
    jpeg.extend_from_slice(&scan.finish());
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    jpeg
}

fn segment(jpeg: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    jpeg.extend_from_slice(&[0xFF, marker]);
    let len = u16::try_from(payload.len() + 2).unwrap_or(u16::MAX);
    jpeg.extend_from_slice(&len.to_be_bytes());
    jpeg.extend_from_slice(payload);
}

#[derive(Default)]
struct ScanWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u32,
}

impl ScanWriter {
    fn bits(&mut self, value: u32, count: u32) {
        for index in (0..count).rev() {
            self.current = (self.current << 1) | u8::from((value >> index) & 1 == 1);
            self.used += 1;
            if self.used == 8 {
                self.flush_byte();
            }
        }
    }

    fn flush_byte(&mut self) {
        self.bytes.push(self.current);
        if self.current == 0xFF {
            self.bytes.push(0x00);
        }
        self.current = 0;
        self.used = 0;
    }

    fn finish(mut self) -> Vec<u8> {
        while self.used != 0 {
            self.bits(1, 1);
        }
        self.bytes
    }
}
