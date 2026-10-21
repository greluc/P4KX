use crate::TestChunkFile;
use crate::animation_keys::{
    TestPositionFormat, TestRotationFormat, TestTimes, encode_positions, encode_rotations,
    encode_times,
};
use crate::bytes::{put_u16, put_u32};

const ANIM_INFO: u32 = 0x4733_C6ED;
const CAF_DATA: u32 = 0xA949_6CB5;
const ANIM_INFO_VERSION: u32 = 0x901;
const CAF_DATA_VERSION: u32 = 0x900;
const BLOCK_HEADER_LEN: usize = 12;
const CONTROLLER_LEN: usize = 24;
const ENTRY_LEN: usize = 48;
const CAF_MAGIC: u16 = 0xFFFF;
const ADDITIVE: u32 = 1;
const STORED: u32 = 2;

/// One bone's keys in a [`TestAnimation`]: rotations and positions at frames, stored in a chosen
/// format.
#[derive(Debug, Clone, PartialEq)]
pub struct TestTrack {
    controller_id: u32,
    rotations: Vec<(u16, [f32; 4])>,
    positions: Vec<(u16, [f32; 3])>,
    rotation_format: TestRotationFormat,
    position_format: TestPositionFormat,
    times: TestTimes,
    rotation_code: Option<u8>,
}

impl TestTrack {
    /// Starts an empty track of the bone with the controller ID `controller_id`, stored as
    /// small-tree rotations, planar positions and byte times.
    #[must_use]
    pub const fn new(controller_id: u32) -> Self {
        Self {
            controller_id,
            rotations: Vec::new(),
            positions: Vec::new(),
            rotation_format: TestRotationFormat::SmallTree,
            position_format: TestPositionFormat::Planar,
            times: TestTimes::Bytes,
            rotation_code: None,
        }
    }

    /// Sets the rotation keys: a frame and a quaternion `x, y, z, w` each.
    #[must_use]
    pub fn rotations(mut self, keys: &[(u16, [f32; 4])]) -> Self {
        self.rotations = keys.to_vec();
        self
    }

    /// Sets the position keys: a frame and a position in metres each.
    #[must_use]
    pub fn positions(mut self, keys: &[(u16, [f32; 3])]) -> Self {
        self.positions = keys.to_vec();
        self
    }

    /// Stores the rotation keys as `format`.
    #[must_use]
    pub const fn rotation_format(mut self, format: TestRotationFormat) -> Self {
        self.rotation_format = format;
        self
    }

    /// Stores the position keys as `format`.
    #[must_use]
    pub const fn position_format(mut self, format: TestPositionFormat) -> Self {
        self.position_format = format;
        self
    }

    /// Stores the key times as `times`.
    #[must_use]
    pub const fn times(mut self, times: TestTimes) -> Self {
        self.times = times;
        self
    }

    /// Writes `code` as the rotation format byte, whatever the keys are stored as.
    #[must_use]
    pub const fn rotation_code(mut self, code: u8) -> Self {
        self.rotation_code = Some(code);
        self
    }

    fn last_frame(&self) -> u16 {
        let rotations = self.rotations.iter().map(|&(frame, _)| frame);
        let positions = self.positions.iter().map(|&(frame, _)| frame);
        rotations.chain(positions).max().unwrap_or(0)
    }

    /// Returns the controller entry and appends the track's times and keys to `pool`; `entry_at`
    /// and `pool_at` are the positions of the entry and of the pool start in the chunk.
    fn encode(&self, entry_at: usize, pool_at: usize, pool: &mut Vec<u8>) -> Vec<u8> {
        let mut entry = Vec::with_capacity(CONTROLLER_LEN);
        let relative = |pool: &Vec<u8>| u32::try_from(pool_at + pool.len() - entry_at).unwrap_or(0);
        let rotation_frames: Vec<u16> = self.rotations.iter().map(|&(frame, _)| frame).collect();
        let position_frames: Vec<u16> = self.positions.iter().map(|&(frame, _)| frame).collect();
        let rotation_keys: Vec<[f32; 4]> = self.rotations.iter().map(|&(_, key)| key).collect();
        let position_keys: Vec<[f32; 3]> = self.positions.iter().map(|&(_, key)| key).collect();
        let sides = [
            (
                rotation_frames,
                self.rotation_code
                    .unwrap_or_else(|| self.rotation_format.code()),
                encode_rotations(self.rotation_format, &rotation_keys),
            ),
            (
                position_frames,
                self.position_format.code(),
                encode_positions(self.position_format, &position_keys),
            ),
        ];
        for (frames, code, data) in sides {
            if frames.is_empty() {
                entry.extend_from_slice(&[0; CONTROLLER_LEN / 2]);
                continue;
            }
            put_u16(&mut entry, u16::try_from(frames.len()).unwrap_or(u16::MAX));
            put_u16(
                &mut entry,
                (u16::from(code) << 8) | u16::from(self.times.code()),
            );
            put_u32(&mut entry, relative(pool));
            pool.extend_from_slice(&encode_times(self.times, &frames));
            put_u32(&mut entry, relative(pool));
            pool.extend_from_slice(&data);
        }
        entry
    }
}

/// A synthetic animation: the `.caf` file of one, or one entry of a [`crate::TestDba`].
///
/// Frames run at `frames_per_second`; the end frame is the last frame any track has a key at.
#[derive(Debug, Clone, PartialEq)]
pub struct TestAnimation {
    pub(crate) name: String,
    pub(crate) frames_per_second: u16,
    pub(crate) additive: bool,
    pub(crate) tracks: Vec<TestTrack>,
}

impl TestAnimation {
    /// Starts an animation without tracks named `name` (a file name or a path as a database
    /// stores it) at `frames_per_second`.
    #[must_use]
    pub fn new(name: &str, frames_per_second: u16) -> Self {
        Self {
            name: name.to_owned(),
            frames_per_second,
            additive: false,
            tracks: Vec::new(),
        }
    }

    /// Adds a track.
    #[must_use]
    pub fn track(mut self, track: TestTrack) -> Self {
        self.tracks.push(track);
        self
    }

    /// Returns the name the animation was started with.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Marks the animation as additive.
    #[must_use]
    pub const fn additive(mut self) -> Self {
        self.additive = true;
        self
    }

    pub(crate) fn end_frame(&self) -> u16 {
        self.tracks
            .iter()
            .map(TestTrack::last_frame)
            .max()
            .unwrap_or(0)
    }

    /// Returns the 48-byte record both the `AnimInfo` chunk and the entries of a database's
    /// metadata hold.
    pub(crate) fn record(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(ENTRY_LEN);
        put_u32(&mut out, STORED | if self.additive { ADDITIVE } else { 0 });
        put_u16(&mut out, self.frames_per_second);
        put_u16(&mut out, u16::try_from(self.tracks.len()).unwrap_or(0));
        put_u16(&mut out, 0);
        put_u16(&mut out, 9);
        put_u32(&mut out, 0);
        put_u32(&mut out, u32::from(self.end_frame()));
        for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// Returns the length of the block's headers: the signature line, the hashes and the
    /// controller entries.
    pub(crate) const fn headers_len(&self) -> usize {
        BLOCK_HEADER_LEN + (4 + CONTROLLER_LEN) * self.tracks.len()
    }

    /// Returns the block: signature, track count, magic, size, hashes and controller entries,
    /// with the keys appended to `pool`. `block_at` is the position of the block in the chunk and
    /// `pool_at` that of the pool.
    pub(crate) fn block(
        &self,
        signature: [u8; 4],
        magic: u16,
        block_at: usize,
        pool_at: usize,
        pool: &mut Vec<u8>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&signature);
        put_u16(&mut out, u16::try_from(self.tracks.len()).unwrap_or(0));
        put_u16(&mut out, magic);
        put_u32(&mut out, 0);
        for track in &self.tracks {
            put_u32(&mut out, track.controller_id);
        }
        let controllers_at = block_at + BLOCK_HEADER_LEN + 4 * self.tracks.len();
        for (index, track) in self.tracks.iter().enumerate() {
            let entry_at = controllers_at + index * CONTROLLER_LEN;
            out.extend_from_slice(&track.encode(entry_at, pool_at, pool));
        }
        out
    }

    /// Returns the data of the `AnimInfo` chunk.
    #[must_use]
    pub fn anim_info(&self) -> Vec<u8> {
        self.record()
    }

    /// Returns the data of the `CafData` chunk.
    #[must_use]
    pub fn caf_data(&self) -> Vec<u8> {
        let mut pool = Vec::new();
        let mut data = self.block(*b"#caf", CAF_MAGIC, 0, self.headers_len(), &mut pool);
        data.extend_from_slice(&pool);
        data
    }

    /// Returns the bytes of the `.caf` file.
    #[must_use]
    pub fn caf(&self) -> Vec<u8> {
        TestChunkFile::ivo()
            .chunk(ANIM_INFO, ANIM_INFO_VERSION, self.anim_info())
            .chunk(CAF_DATA, CAF_DATA_VERSION, self.caf_data())
            .bytes()
    }
}
