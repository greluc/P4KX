use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

/// The signature and with it the compression of a [`TestMovie`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSignature {
    /// Uncompressed SWF.
    Fws,
    /// zlib-compressed SWF.
    Cws,
    /// LZMA-compressed SWF.
    Zws,
    /// Uncompressed Scaleform GFx.
    Gfx,
    /// zlib-compressed Scaleform GFx.
    Cfx,
}

impl TestSignature {
    const fn bytes(self) -> &'static [u8; 3] {
        match self {
            Self::Fws => b"FWS",
            Self::Cws => b"CWS",
            Self::Zws => b"ZWS",
            Self::Gfx => b"GFX",
            Self::Cfx => b"CFX",
        }
    }
}

/// Writes a GFx or SWF movie: the file header, the movie header and a tag stream ending with
/// an `End` tag.
#[derive(Debug, Clone)]
pub struct TestMovie {
    signature: TestSignature,
    version: u8,
    width: i32,
    height: i32,
    frame_count: u16,
    tags: Vec<u8>,
}

impl TestMovie {
    /// Starts a 1024 x 768 pixel movie of version 10 at 30 frames per second.
    #[must_use]
    pub const fn new(signature: TestSignature) -> Self {
        Self {
            signature,
            version: 10,
            width: 1024 * 20,
            height: 768 * 20,
            frame_count: 1,
            tags: Vec::new(),
        }
    }

    /// Sets the frame count the header declares.
    #[must_use]
    pub const fn frame_count(mut self, frame_count: u16) -> Self {
        self.frame_count = frame_count;
        self
    }

    /// Appends a tag with `body`, with a short header when the body is shorter than 63 bytes.
    #[must_use]
    pub fn tag(mut self, code: u16, body: &[u8]) -> Self {
        self.tags.extend_from_slice(&tag_bytes(code, body));
        self
    }

    /// Appends a tag with `body` and always a long header.
    #[must_use]
    pub fn long_tag(mut self, code: u16, body: &[u8]) -> Self {
        self.tags.extend_from_slice(&long_tag_bytes(code, body));
        self
    }

    /// Appends raw bytes to the tag stream, e.g. a broken tag header.
    #[must_use]
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.tags.extend_from_slice(bytes);
        self
    }

    /// Returns the movie body after the file header: movie header, tags and `End`.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let mut bits = BitWriter::default();
        let fields = [0, self.width, 0, self.height];
        let size = signed_bits(&fields);
        bits.ub(5, size);
        for field in fields {
            bits.sb(size, field);
        }
        let mut body = bits.finish();
        body.extend_from_slice(&[0, 30]);
        body.extend_from_slice(&self.frame_count.to_le_bytes());
        body.extend_from_slice(&self.tags);
        body.extend_from_slice(&tag_bytes(0, &[]));
        body
    }

    /// Returns the whole movie file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let body = self.body();
        let declared = u32::try_from(body.len() + 8).unwrap_or(u32::MAX);
        let mut file = self.signature.bytes().to_vec();
        file.push(self.version);
        file.extend_from_slice(&declared.to_le_bytes());
        match self.signature {
            TestSignature::Fws | TestSignature::Gfx => file.extend_from_slice(&body),
            TestSignature::Cws | TestSignature::Cfx => file.extend_from_slice(&zlib(&body)),
            TestSignature::Zws => {
                let mut lzma = Vec::new();
                lzma_rs::lzma_compress(&mut body.as_slice(), &mut lzma).unwrap_or_default();
                let properties = lzma.get(..5).unwrap_or_default();
                let stream = lzma.get(13..).unwrap_or_default();
                let compressed = u32::try_from(stream.len()).unwrap_or(u32::MAX);
                file.extend_from_slice(&compressed.to_le_bytes());
                file.extend_from_slice(properties);
                file.extend_from_slice(stream);
            }
        }
        file
    }
}

/// Returns a tag with a short header when the body is shorter than 63 bytes, else a long one.
#[must_use]
pub fn tag_bytes(code: u16, body: &[u8]) -> Vec<u8> {
    if body.len() >= 0x3F {
        return long_tag_bytes(code, body);
    }
    let header = (code << 6) | u16::try_from(body.len()).unwrap_or_default();
    let mut tag = header.to_le_bytes().to_vec();
    tag.extend_from_slice(body);
    tag
}

fn long_tag_bytes(code: u16, body: &[u8]) -> Vec<u8> {
    let mut tag = ((code << 6) | 0x3F).to_le_bytes().to_vec();
    tag.extend_from_slice(&u32::try_from(body.len()).unwrap_or(u32::MAX).to_le_bytes());
    tag.extend_from_slice(body);
    tag
}

/// Compresses `data` with zlib.
#[must_use]
pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap_or_default();
    encoder.finish().unwrap_or_default()
}

/// Writes SWF bit fields, most significant bit first.
#[derive(Debug, Default, Clone)]
pub(crate) struct BitWriter {
    bytes: Vec<u8>,
    used: u32,
}

impl BitWriter {
    /// Writes the low `bits` bits of `value`.
    pub(crate) fn ub(&mut self, bits: u32, value: u32) {
        for index in (0..bits).rev() {
            if self.used == 0 {
                self.bytes.push(0);
            }
            let bit = u8::from((value >> index) & 1 == 1);
            if let Some(last) = self.bytes.last_mut() {
                *last |= bit << (7 - self.used);
            }
            self.used = (self.used + 1) % 8;
        }
    }

    /// Writes `value` as a signed field of `bits` bits.
    pub(crate) fn sb(&mut self, bits: u32, value: i32) {
        self.ub(bits, value.cast_unsigned());
    }

    /// Writes a flag bit.
    pub(crate) fn flag(&mut self, value: bool) {
        self.ub(1, u32::from(value));
    }

    /// Pads to the next byte boundary.
    pub(crate) const fn align(&mut self) {
        self.used = 0;
    }

    /// Appends whole bytes after padding to the next byte boundary.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        self.align();
        self.bytes.extend_from_slice(bytes);
    }

    /// Returns the bytes written, the last one padded with zero bits.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Returns the fewest bits that hold every value as a signed field.
pub(crate) fn signed_bits(values: &[i32]) -> u32 {
    values
        .iter()
        .map(|&value| {
            let magnitude = if value < 0 { !value } else { value };
            33 - magnitude.leading_zeros()
        })
        .max()
        .unwrap_or(1)
}

/// Writes a RECT of the given edges in twips.
pub(crate) fn rect(bits: &mut BitWriter, edges: [i32; 4]) {
    bits.align();
    let size = signed_bits(&edges);
    bits.ub(5, size);
    for edge in edges {
        bits.sb(size, edge);
    }
    bits.align();
}

/// Writes a MATRIX with only a translation in twips.
pub(crate) fn translation(bits: &mut BitWriter, x: i32, y: i32) {
    bits.align();
    bits.flag(false);
    bits.flag(false);
    let size = signed_bits(&[x, y]);
    bits.ub(5, size);
    bits.sb(size, x);
    bits.sb(size, y);
    bits.align();
}
