use crate::TestAnimation;
use crate::TestChunkFile;
use crate::bytes::put_u32;

const DBA_DATA: u32 = 0x194F_BC50;
const DBA_METADATA: u32 = 0xF735_1608;
const POOL_FILLER: u32 = 0xDEAD_BEEF;
const DBA_DATA_VERSION: u32 = 0x900;
const DBA_METADATA_VERSION: u32 = 0x902;
const DBA_MAGIC: u16 = 0xAA55;
const PREFIX_LEN: usize = 4;

/// A synthetic `.dba` animation database: block headers back to back, one shared pool of keys
/// after them, and the metadata chunk with the animations' names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TestDba {
    animations: Vec<TestAnimation>,
    pool_after_metadata: bool,
}

impl TestDba {
    /// Starts an empty database.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            animations: Vec::new(),
            pool_after_metadata: false,
        }
    }

    /// Adds an animation.
    #[must_use]
    pub fn animation(mut self, animation: TestAnimation) -> Self {
        self.animations.push(animation);
        self
    }

    /// Puts the shared pool of keys into a chunk after the metadata instead of behind the
    /// headers, so the controllers' offsets reach past the `DbaData` chunk's span.
    #[must_use]
    pub const fn pool_after_metadata(mut self) -> Self {
        self.pool_after_metadata = true;
        self
    }

    fn headers(&self, pool_at: usize) -> (Vec<u8>, Vec<u8>) {
        let mut headers = vec![0; PREFIX_LEN];
        let mut pool = Vec::new();
        for animation in &self.animations {
            let block = animation.block(*b"#dba", DBA_MAGIC, headers.len(), pool_at, &mut pool);
            headers.extend_from_slice(&block);
        }
        let total = u32::try_from(headers.len() - PREFIX_LEN + pool.len()).unwrap_or(u32::MAX);
        if let Some(prefix) = headers.get_mut(..PREFIX_LEN) {
            prefix.copy_from_slice(&total.to_le_bytes());
        }
        (headers, pool)
    }

    fn metadata(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u32(&mut out, u32::try_from(self.animations.len()).unwrap_or(0));
        put_u32(&mut out, 0);
        for animation in &self.animations {
            out.extend_from_slice(&animation.record());
        }
        for animation in &self.animations {
            out.extend_from_slice(animation.name.as_bytes());
            out.push(0);
        }
        out
    }

    /// Returns the bytes of the `.dba` file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let headers_len = PREFIX_LEN
            + self
                .animations
                .iter()
                .map(TestAnimation::headers_len)
                .sum::<usize>();
        if !self.pool_after_metadata {
            let (mut data, pool) = self.headers(headers_len);
            data.extend_from_slice(&pool);
            return TestChunkFile::ivo()
                .chunk(DBA_DATA, DBA_DATA_VERSION, data)
                .chunk(DBA_METADATA, DBA_METADATA_VERSION, self.metadata())
                .bytes();
        }
        let (headers, pool) = self.headers(headers_len);
        let layout = TestChunkFile::ivo()
            .chunk(DBA_DATA, DBA_DATA_VERSION, headers)
            .chunk(DBA_METADATA, DBA_METADATA_VERSION, self.metadata())
            .chunk(POOL_FILLER, 0, pool);
        let offsets = layout.offsets();
        let pool_at = match offsets.as_slice() {
            [first, _, third] => usize::try_from(third - first).unwrap_or(0),
            _ => 0,
        };
        let (headers, pool) = self.headers(pool_at);
        TestChunkFile::ivo()
            .chunk(DBA_DATA, DBA_DATA_VERSION, headers)
            .chunk(DBA_METADATA, DBA_METADATA_VERSION, self.metadata())
            .chunk(POOL_FILLER, 0, pool)
            .bytes()
    }
}
