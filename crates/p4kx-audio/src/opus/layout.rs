// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::OpusError;

/// The channel mapping value of a silent channel.
pub(crate) const SILENT: u8 = 255;
const FAMILY_VORBIS: u8 = 1;
const FAMILY_UNDEFINED: u8 = 255;
const VORBIS_FAMILY_CHANNELS: usize = 8;
const WWISE_STANDARD_CONFIG: u32 = 1;
const WWISE_MAPPING_VORBIS: u8 = 1;
const WWISE_MAPPING_UNDEFINED: u8 = 255;
const STEREO: u32 = 0x003;
const THREE_STEREO: u32 = 0x007;
const QUAD_SIDE: u32 = 0x603;
const FIVE_SURROUND: u32 = 0x607;
const FIVE_ONE_SURROUND: u32 = 0x60F;
const SEVEN_ONE_SURROUND: u32 = 0x63F;
const VORBIS_ORDER: [[u8; 8]; 8] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 2, 1, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 0, 0, 0, 0],
    [0, 2, 1, 3, 4, 0, 0, 0],
    [0, 2, 1, 5, 3, 4, 0, 0],
    [0, 2, 1, 6, 5, 3, 4, 0],
    [0, 2, 1, 7, 5, 6, 3, 4],
];
const WWISE_MAPPING: [[u8; 8]; 8] = [
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 2, 1, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 0, 0, 0, 0],
    [0, 4, 1, 2, 3, 0, 0, 0],
    [0, 4, 1, 2, 3, 5, 0, 0],
    [0, 6, 1, 2, 3, 4, 5, 0],
    [0, 6, 1, 2, 3, 4, 5, 7],
];

/// How the Opus streams of a packet become the channels of the sound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The channel mapping family of the `OpusHead`.
    pub(crate) family: u8,
    /// Opus streams per packet, coupled ones first.
    pub(crate) streams: u8,
    /// Streams that carry a stereo pair.
    pub(crate) coupled: u8,
    /// The `OpusHead` channel mapping: per channel in the family's order, the decoded channel.
    pub(crate) mapping: Vec<u8>,
    /// Per channel in the WEM's speaker order, the decoded channel; 255 for silence.
    pub(crate) wav_order: Vec<u8>,
}

impl Layout {
    /// Returns the layout Wwise implies for `channels`, the raw `fmt` channel layout field and
    /// the mapping family byte of a Wwise Opus `fmt` chunk, as vgmstream derives it.
    pub(crate) fn wwise(channels: u16, raw_layout: u32, mapping: u8) -> Result<Self, OpusError> {
        let (config, mask) = if raw_layout & 0xFF == u32::from(channels) {
            ((raw_layout >> 8) & 0x0F, raw_layout >> 12)
        } else {
            (0, raw_layout)
        };
        let unsupported = || OpusError::UnsupportedLayout { channels, mapping };
        let count = usize::from(channels);
        if config == WWISE_STANDARD_CONFIG && mapping == WWISE_MAPPING_VORBIS {
            let row = count
                .checked_sub(1)
                .and_then(|index| WWISE_MAPPING.get(index))
                .ok_or_else(unsupported)?;
            let coupled: u8 = match mask {
                SEVEN_ONE_SURROUND => 3,
                FIVE_ONE_SURROUND | FIVE_SURROUND | QUAD_SIDE => 2,
                THREE_STEREO | STEREO => 1,
                _ => 0,
            };
            let streams = u8::try_from(channels)
                .ok()
                .and_then(|channels| channels.checked_sub(coupled))
                .filter(|&streams| streams > 0)
                .ok_or_else(unsupported)?;
            let family = if channels > 2 || streams > 1 {
                FAMILY_VORBIS
            } else {
                0
            };
            let mapping = row.get(..count).ok_or_else(unsupported)?.to_vec();
            return Self::new(family, streams, coupled, mapping);
        }
        if config != 0 && mapping == WWISE_MAPPING_UNDEFINED {
            let streams = u8::try_from(channels).map_err(|_| unsupported())?;
            return Self::new(FAMILY_UNDEFINED, streams, 0, (0..streams).collect());
        }
        if (1..=2).contains(&channels) {
            let coupled = u8::from(channels == 2);
            return Self::new(0, 1, coupled, (0..=coupled).collect());
        }
        Err(unsupported())
    }

    /// Checks the channel mapping of an `OpusHead` for `channels` channels.
    pub(crate) fn new(
        family: u8,
        streams: u8,
        coupled: u8,
        mapping: Vec<u8>,
    ) -> Result<Self, OpusError> {
        let decoded = u16::from(streams) + u16::from(coupled);
        if streams == 0 || coupled > streams || decoded > 255 {
            return Err(OpusError::InvalidHeader("invalid stream counts"));
        }
        if mapping
            .iter()
            .any(|&index| index != SILENT && u16::from(index) >= decoded)
        {
            return Err(OpusError::InvalidHeader(
                "the channel mapping is out of range",
            ));
        }
        let channels = mapping.len();
        if channels == 0
            || (family == 0 && (channels > 2 || streams != 1))
            || (family == FAMILY_VORBIS && channels > VORBIS_FAMILY_CHANNELS)
        {
            return Err(OpusError::InvalidHeader(
                "the channel mapping does not fit its family",
            ));
        }
        let wav_order = match VORBIS_ORDER.get(channels - 1) {
            Some(order) if family == FAMILY_VORBIS => order
                .iter()
                .take(channels)
                .map(|&position| {
                    mapping
                        .get(usize::from(position))
                        .copied()
                        .unwrap_or(SILENT)
                })
                .collect(),
            _ => mapping.clone(),
        };
        Ok(Self {
            family,
            streams,
            coupled,
            mapping,
            wav_order,
        })
    }

    /// Returns the channels the streams of a packet decode to, before the mapping.
    pub(crate) fn decoded_channels(&self) -> usize {
        usize::from(self.streams) + usize::from(self.coupled)
    }
}
