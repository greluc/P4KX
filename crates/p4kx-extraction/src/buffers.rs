const CHUNK_LEN: usize = 1 << 20;
const RETAINED_LEN: usize = 64 << 20;

/// The buffers one worker of an Extraction reuses for every Entry it writes.
pub(crate) struct Buffers {
    /// Holds one chunk of Entry data or of an existing file.
    pub(crate) chunk: Vec<u8>,
    /// Holds the whole data of a `CryXml` or DataForge Entry.
    pub(crate) data: Vec<u8>,
    /// Holds a converted file before it is written: the XML of a `CryXml` Entry or a PNG.
    pub(crate) output: Vec<u8>,
}

impl Buffers {
    pub(crate) fn new() -> Self {
        Self {
            chunk: vec![0; CHUNK_LEN],
            data: Vec::new(),
            output: Vec::new(),
        }
    }

    /// Frees the conversion buffers once an unusually large Entry has grown them.
    pub(crate) fn trim(&mut self) {
        for buffer in [&mut self.data, &mut self.output] {
            buffer.clear();
            if buffer.capacity() > RETAINED_LEN {
                *buffer = Vec::new();
            }
        }
    }
}
