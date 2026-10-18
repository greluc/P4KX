use std::fmt::Display;
use std::panic::{self, AssertUnwindSafe};

use lewton::audio::{PreviousWindowRight, read_audio_packet_generic};
use lewton::header::{read_header_comment, read_header_ident, read_header_setup};

use super::VorbisError;
use super::packets::AudioPackets;

const CHECKED_PACKETS: usize = 16;
const CLIPPED_SHARE: usize = 10;

/// How well rebuilt headers fit the audio packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fit {
    /// The packets decode to plausible samples.
    Clean,
    /// The packets decode, but more than a tenth of the samples are clipped, which garbage from
    /// codebooks of another library usually is.
    Clipped,
}

/// Decodes the rebuilt headers and the first audio packets with a standard Vorbis decoder.
pub(crate) fn validate(
    ident: &[u8],
    comment: &[u8],
    setup: &[u8],
    audio: AudioPackets<'_>,
) -> Result<Fit, VorbisError> {
    panic::catch_unwind(AssertUnwindSafe(|| decode(ident, comment, setup, audio)))
        .unwrap_or_else(|_| Err(VorbisError::Rejected("the decoder failed".to_owned())))
}

fn rejected(error: &dyn Display) -> VorbisError {
    VorbisError::Rejected(error.to_string())
}

fn decode(
    ident: &[u8],
    comment: &[u8],
    setup: &[u8],
    mut audio: AudioPackets<'_>,
) -> Result<Fit, VorbisError> {
    let ident = read_header_ident(ident).map_err(|error| rejected(&error))?;
    read_header_comment(comment).map_err(|error| rejected(&error))?;
    let blocksizes = (ident.blocksize_0, ident.blocksize_1);
    let setup = read_header_setup(setup, ident.audio_channels, blocksizes)
        .map_err(|error| rejected(&error))?;
    let mut window = PreviousWindowRight::new();
    let mut packet = Vec::new();
    let (mut samples, mut clipped) = (0, 0);
    for _ in 0..CHECKED_PACKETS {
        match audio.next_into(&mut packet) {
            Some(rebuilt) => rebuilt?,
            None => break,
        };
        let channels: Vec<Vec<f32>> =
            read_audio_packet_generic(&ident, &setup, &packet, &mut window)
                .map_err(|error| rejected(&error))?;
        for &sample in channels.iter().flatten() {
            if !sample.is_finite() {
                return Err(VorbisError::Rejected(
                    "the decoder returns no number".to_owned(),
                ));
            }
            samples += 1;
            if sample.abs() >= 1.0 {
                clipped += 1;
            }
        }
    }
    Ok(if clipped * CLIPPED_SHARE > samples {
        Fit::Clipped
    } else {
        Fit::Clean
    })
}
