use crate::TestEntry;
use crate::bytes::{len_u16, put_u16, put_u32};

const LOCAL_HEADER: u32 = 0x0403_4B50;
const CENTRAL_HEADER: u32 = 0x0201_4B50;
const END: u32 = 0x0605_4B50;
const VERSION: u16 = 20;

/// Builds a synthetic container in the byte layout of real socpak containers.
///
/// A container is a plain ZIP archive: standard local headers, a central directory without ZIP64
/// or CIG extra blocks, 32-bit sizes and offsets, the CRC-32 of each Entry's data and an end
/// record without comment. Its bytes become the data of an Entry, which may itself hold further
/// containers.
#[derive(Debug, Clone, Default)]
pub struct TestContainer {
    entries: Vec<TestEntry>,
}

impl TestContainer {
    /// Creates a builder for a container without Entries.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an Entry.
    #[must_use]
    pub fn entry(mut self, entry: TestEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Adds `container` as a stored Entry at `path`.
    #[must_use]
    pub fn container(self, path: impl Into<String>, container: &Self) -> Self {
        self.entry(TestEntry::stored(path, container.build()))
    }

    /// Returns the bytes of the container.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let offsets: Vec<u32> = self
            .entries
            .iter()
            .map(|entry| write_local_entry(&mut out, entry))
            .collect();
        let directory_offset = len_u32(&out);
        for (entry, offset) in self.entries.iter().zip(offsets) {
            write_central_entry(&mut out, entry, offset);
        }
        let directory_size = len_u32(&out) - directory_offset;
        put_u32(&mut out, END);
        put_u16(&mut out, 0);
        put_u16(&mut out, 0);
        put_u16(&mut out, len_u16(&self.entries));
        put_u16(&mut out, len_u16(&self.entries));
        put_u32(&mut out, directory_size);
        put_u32(&mut out, directory_offset);
        put_u16(&mut out, 0);
        out
    }
}

fn write_local_entry(out: &mut Vec<u8>, entry: &TestEntry) -> u32 {
    let offset = len_u32(out);
    put_u32(out, LOCAL_HEADER);
    put_u16(out, VERSION);
    put_u16(out, 0);
    put_u16(out, entry.method);
    put_u32(out, 0);
    put_u32(out, entry.crc32);
    put_u32(out, len_u32(&entry.payload));
    put_u32(out, size_u32(entry.uncompressed_size));
    put_u16(out, len_u16(entry.path_bytes()));
    put_u16(out, 0);
    out.extend_from_slice(entry.path_bytes());
    out.extend_from_slice(&entry.payload);
    offset
}

fn write_central_entry(out: &mut Vec<u8>, entry: &TestEntry, local_offset: u32) {
    put_u32(out, CENTRAL_HEADER);
    put_u16(out, VERSION);
    put_u16(out, VERSION);
    put_u16(out, 0);
    put_u16(out, entry.method);
    put_u32(out, 0);
    put_u32(out, entry.crc32);
    put_u32(out, len_u32(&entry.payload));
    put_u32(out, size_u32(entry.uncompressed_size));
    put_u16(out, len_u16(entry.path_bytes()));
    put_u16(out, 0);
    put_u16(out, 0);
    put_u16(out, 0);
    put_u16(out, 0);
    put_u32(out, 0);
    put_u32(out, local_offset);
    out.extend_from_slice(entry.path_bytes());
}

fn len_u32(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).unwrap_or(u32::MAX)
}

fn size_u32(size: u64) -> u32 {
    u32::try_from(size).unwrap_or(u32::MAX)
}
