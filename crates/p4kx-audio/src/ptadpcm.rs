//! The Platinum ADPCM frame decoder Wwise uses for PTADPCM WEMs.

/// Bytes of the header in front of every channel's frame: two history samples and a step index.
pub(crate) const HEADER_LEN: usize = 5;

const MAX_STEP_INDEX: u8 = 12;

/// The step and the next step index for every step index and nibble; index 12 has neither.
#[rustfmt::skip]
const STEPS: [[(i32, u8); 16]; 12] = [
    [
        (-14, 2), (-10, 2), (-7, 1), (-5, 1), (-3, 0), (-2, 0), (-1, 0), (0, 0),
        (0, 0), (1, 0), (2, 0), (3, 0), (5, 1), (7, 1), (10, 2), (14, 2),
    ],
    [
        (-28, 3), (-20, 3), (-14, 2), (-10, 2), (-7, 1), (-5, 1), (-3, 1), (-1, 0),
        (1, 0), (3, 1), (5, 1), (7, 1), (10, 2), (14, 2), (20, 3), (28, 3),
    ],
    [
        (-56, 4), (-40, 4), (-28, 3), (-20, 3), (-14, 2), (-10, 2), (-6, 2), (-2, 1),
        (2, 1), (6, 2), (10, 2), (14, 2), (20, 3), (28, 3), (40, 4), (56, 4),
    ],
    [
        (-112, 5), (-80, 5), (-56, 4), (-40, 4), (-28, 3), (-20, 3), (-12, 3), (-4, 2),
        (4, 2), (12, 3), (20, 3), (28, 3), (40, 4), (56, 4), (80, 5), (112, 5),
    ],
    [
        (-224, 6), (-160, 6), (-112, 5), (-80, 5), (-56, 4), (-40, 4), (-24, 4), (-8, 3),
        (8, 3), (24, 4), (40, 4), (56, 4), (80, 5), (112, 5), (160, 6), (224, 6),
    ],
    [
        (-448, 7), (-320, 7), (-224, 6), (-160, 6), (-112, 5), (-80, 5), (-48, 5), (-16, 4),
        (16, 4), (48, 5), (80, 5), (112, 5), (160, 6), (224, 6), (320, 7), (448, 7),
    ],
    [
        (-896, 8), (-640, 8), (-448, 7), (-320, 7), (-224, 6), (-160, 6), (-96, 6), (-32, 5),
        (32, 5), (96, 6), (160, 6), (224, 6), (320, 7), (448, 7), (640, 8), (896, 8),
    ],
    [
        (-1792, 9), (-1280, 9), (-896, 8), (-640, 8), (-448, 7), (-320, 7), (-192, 7), (-64, 6),
        (64, 6), (192, 7), (320, 7), (448, 7), (640, 8), (896, 8), (1280, 9), (1792, 9),
    ],
    [
        (-3584, 10), (-2560, 10), (-1792, 9), (-1280, 9), (-896, 8), (-640, 8), (-384, 8),
        (-128, 7), (128, 7), (384, 8), (640, 8), (896, 8), (1280, 9), (1792, 9), (2560, 10),
        (3584, 10),
    ],
    [
        (-7168, 11), (-5120, 11), (-3584, 10), (-2560, 10), (-1792, 9), (-1280, 9), (-768, 9),
        (-256, 8), (256, 8), (768, 9), (1280, 9), (1792, 9), (2560, 10), (3584, 10), (5120, 11),
        (7168, 11),
    ],
    [
        (-14336, 11), (-10240, 11), (-7168, 11), (-5120, 11), (-3584, 10), (-2560, 10),
        (-1536, 10), (-512, 9), (512, 9), (1536, 10), (2560, 10), (3584, 10), (5120, 11),
        (7168, 11), (10240, 11), (14336, 11),
    ],
    [
        (-28672, 11), (-20480, 11), (-14336, 11), (-10240, 11), (-7168, 11), (-5120, 11),
        (-3072, 11), (-1024, 10), (1024, 10), (3072, 11), (5120, 11), (7168, 11), (10240, 11),
        (14336, 11), (20480, 11), (28672, 11),
    ],
];

/// Returns the samples a frame of `len` bytes holds: the two history samples and every nibble.
pub(crate) const fn samples_per_frame(len: usize) -> usize {
    2 + (len - HEADER_LEN) * 2
}

/// Decodes one channel's frame into `out`, one sample per slot.
///
/// The header is little-endian in every WEM; the frame must be longer than [`HEADER_LEN`].
pub(crate) fn decode<'a>(frame: &[u8], out: impl Iterator<Item = &'a mut [u8; 2]>) {
    let sample_at = |offset: usize| {
        frame
            .get(offset..)
            .and_then(<[u8]>::first_chunk::<2>)
            .map_or(0, |bytes| i32::from(i16::from_le_bytes(*bytes)))
    };
    let mut older = sample_at(0);
    let mut newer = sample_at(2);
    let mut index = frame.get(4).copied().unwrap_or(0).min(MAX_STEP_INDEX);
    let nibbles = frame
        .get(HEADER_LEN..)
        .unwrap_or_default()
        .iter()
        .flat_map(|&byte| [byte & 0x0F, byte >> 4]);
    let mut out = out;
    for history in [older, newer] {
        if let Some(slot) = out.next() {
            *slot = to_bytes(history);
        }
    }
    for (slot, nibble) in out.zip(nibbles) {
        let (step, next) = STEPS
            .get(usize::from(index))
            .and_then(|row| row.get(usize::from(nibble)))
            .copied()
            .unwrap_or((0, 0));
        let sample = (step + 2 * newer - older).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
        older = newer;
        newer = sample;
        index = next;
        *slot = to_bytes(sample);
    }
}

fn to_bytes(sample: i32) -> [u8; 2] {
    i16::try_from(sample).unwrap_or_default().to_le_bytes()
}
