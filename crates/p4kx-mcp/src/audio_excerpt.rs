// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use p4kx_extraction::AudioError;
use rmcp::ErrorData;
use rmcp::handler::server::tool::IntoCallToolResult;
use rmcp::model::{CallToolResponse, CallToolResult, ContentBlock};

use crate::archives::Opened;
use crate::audio_params::ExcerptParams;
use crate::audio_shapes::ExcerptResult;
use crate::audio_wem::{Candidate, Held, audio_error, candidates, read};
use crate::error::ToolError;
use crate::noted::add_notice;

/// The size cap of the WAV of an excerpt, 1 MiB.
pub(crate) const MAX_EXCERPT_BYTES: usize = 1 << 20;
/// The size cap of a whole decoded sound, 64 MiB.
pub(crate) const MAX_DECODED_BYTES: u64 = 64 << 20;
/// The longest excerpt.
const MAX_EXCERPT: Duration = Duration::from_secs(10);
/// The length of an excerpt when a call gives none.
const DEFAULT_EXCERPT: Duration = Duration::from_secs(5);
const WAV_HEAD_LEN: usize = 12;
const CHUNK_HEAD_LEN: usize = 8;

/// A tool result of structured metadata and one WAV as an audio content block.
pub(crate) struct Excerpted {
    pub(crate) value: ExcerptResult,
    pub(crate) wav: Vec<u8>,
    pub(crate) notice: Option<String>,
}

impl IntoCallToolResult for Excerpted {
    fn into_call_tool_result(self) -> Result<CallToolResponse, ErrorData> {
        let value = serde_json::to_value(self.value).map_err(|error| {
            ErrorData::internal_error(format!("Failed to serialize the result: {error}"), None)
        })?;
        let mut result = CallToolResult::structured(value);
        result
            .content
            .push(ContentBlock::audio(STANDARD.encode(&self.wav), "audio/wav"));
        add_notice(&mut result, self.notice);
        Ok(result.into())
    }
}

/// A piece of a decoded sound as a WAV.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Cut {
    pub(crate) wav: Vec<u8>,
    pub(crate) frames: u64,
    pub(crate) start: u64,
    pub(crate) total: u64,
    pub(crate) truncated: bool,
}

/// Cuts `frames` frames from frame `start` out of the WAV `whole` a WEM decoded to, to a WAV of
/// at most `max_bytes`; the `fmt ` chunk is kept as it is.
pub(crate) fn cut(
    whole: &[u8],
    start: u64,
    frames: u64,
    max_bytes: usize,
) -> Result<Cut, AudioError> {
    let bad = |what| AudioError::InvalidFormat(what);
    let (format, pcm) = chunks(whole).ok_or_else(|| bad("the decoded WAV has no fmt and data"))?;
    let channels = format
        .get(CHUNK_HEAD_LEN + 2..CHUNK_HEAD_LEN + 4)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(u16::from_le_bytes)
        .filter(|channels| *channels > 0)
        .ok_or_else(|| bad("the decoded WAV has no channels"))?;
    let frame_len = usize::from(channels) * 2;
    let total = (pcm.len() / frame_len) as u64;
    let fixed = WAV_HEAD_LEN + format.len() + CHUNK_HEAD_LEN;
    let fits = (max_bytes.saturating_sub(fixed) / frame_len) as u64;
    let available = total.saturating_sub(start);
    let taken = frames.min(fits).min(available);
    let from = usize::try_from(start.min(total)).unwrap_or(usize::MAX) * frame_len;
    let len = usize::try_from(taken).unwrap_or(usize::MAX) * frame_len;
    let body = pcm
        .get(from..from + len)
        .ok_or_else(|| bad("the excerpt lies outside the decoded sound"))?;
    let riff_len =
        u32::try_from(fixed - CHUNK_HEAD_LEN + len).map_err(|_| AudioError::WavTooLarge)?;
    let data_len = u32::try_from(len).map_err(|_| AudioError::WavTooLarge)?;
    let mut wav = Vec::with_capacity(fixed + len);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(format);
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(body);
    Ok(Cut {
        wav,
        frames: taken,
        start,
        total,
        truncated: taken < frames,
    })
}

/// Returns the whole `fmt ` chunk with its header, and the body of the `data` chunk of a WAV.
fn chunks(wav: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut at = WAV_HEAD_LEN;
    let (mut format, mut data) = (None, None);
    while let Some(head) = wav.get(at..at + CHUNK_HEAD_LEN) {
        let size = usize::try_from(u32::from_le_bytes(head.get(4..8)?.try_into().ok()?)).ok()?;
        let end = (at + CHUNK_HEAD_LEN).checked_add(size)?.min(wav.len());
        let body = wav.get(at + CHUNK_HEAD_LEN..end)?;
        match head.get(..4)? {
            b"fmt " => format = wav.get(at..end),
            b"data" => data = Some(body),
            _ => {}
        }
        at = end + (size & 1);
    }
    format.zip(data)
}

/// Returns `seconds` as a duration, or the error that names `name`.
fn seconds(
    name: &'static str,
    seconds: Option<f64>,
    default: Duration,
) -> Result<Duration, ToolError> {
    seconds.map_or(Ok(default), |value| {
        Duration::try_from_secs_f64(value).map_err(|_| ToolError::BadSeconds(name))
    })
}

fn frames_of(time: Duration, rate: u32) -> u64 {
    let frames = time.as_nanos() * u128::from(rate) / 1_000_000_000;
    u64::try_from(frames).unwrap_or(u64::MAX)
}

/// Returns the playing time of `frames` frames at `rate` Hz in seconds.
fn seconds_of(frames: u64, rate: u32) -> f64 {
    let rate = u64::from(rate.max(1));
    let nanos = (frames % rate) * 1_000_000_000 / rate;
    Duration::new(frames / rate, u32::try_from(nanos).unwrap_or(0)).as_secs_f64()
}

/// Decodes the WEM `params` names and returns an excerpt of it as a WAV.
pub(crate) fn excerpt(
    opened: &Opened,
    params: &ExcerptParams,
) -> Result<(ExcerptResult, Vec<u8>), ToolError> {
    let start = seconds("start_seconds", params.start_seconds, Duration::ZERO)?;
    let length =
        seconds("duration_seconds", params.duration_seconds, DEFAULT_EXCERPT)?.min(MAX_EXCERPT);
    let (id, places) = candidates(opened, params.wem_id, params.path.as_deref())?;
    let mut prefetched = false;
    let mut failure: Option<ToolError> = None;
    for place in &places {
        match decode(opened, place, id, (start, length)) {
            Ok(Some(done)) => return Ok(done),
            Ok(None) => prefetched = true,
            Err(error) => {
                failure.get_or_insert(error);
            }
        }
    }
    match (failure, prefetched, id) {
        (Some(error), _, _) => Err(error),
        (None, true, Some(id)) => Err(ToolError::PrefetchOnly(id)),
        (None, _, _) => Err(ToolError::WemSelector),
    }
}

fn decode(
    opened: &Opened,
    place: &Candidate,
    id: Option<u32>,
    (start, length): (Duration, Duration),
) -> Result<Option<(ExcerptResult, Vec<u8>)>, ToolError> {
    read(opened, place, |held| {
        let wem = match held {
            Held::Parsed(wem) => wem,
            Held::Prefetch => return Ok(None),
            Held::Unreadable(error) => return Err(audio_error(&place.location.path, error)),
        };
        let facts = wem.facts();
        let frame_bytes = u64::from(facts.channels) * 2;
        if facts
            .sample_count
            .is_some_and(|frames| frames.saturating_mul(frame_bytes) > MAX_DECODED_BYTES)
        {
            return Err(ToolError::ExcerptSource {
                limit: MAX_DECODED_BYTES,
            });
        }
        let whole = wem.wav(MAX_DECODED_BYTES, &place.location.path)?;
        let wanted = frames_of(length, facts.sample_rate);
        let first = frames_of(start, facts.sample_rate);
        let piece = cut(&whole, first, wanted, MAX_EXCERPT_BYTES)
            .map_err(|error| audio_error(&place.location.path, error))?;
        if (piece.start >= piece.total && piece.total > 0) || (piece.frames == 0 && wanted > 0) {
            return Err(ToolError::ExcerptStart {
                seconds: seconds_of(piece.total, facts.sample_rate),
            });
        }
        let result = ExcerptResult {
            archive: opened.archive().path().display().to_string(),
            wem_id: id,
            source: place.location.clone(),
            codec: facts.codec,
            mime_type: "audio/wav",
            channels: facts.channels,
            sample_rate: facts.sample_rate,
            sound_seconds: seconds_of(piece.total, facts.sample_rate),
            start_seconds: seconds_of(piece.start, facts.sample_rate),
            duration_seconds: seconds_of(piece.frames, facts.sample_rate),
            frames: piece.frames,
            bytes: piece.wav.len(),
            max_bytes: MAX_EXCERPT_BYTES,
            truncated: piece.truncated,
        };
        Ok(Some((result, piece.wav)))
    })
}

#[cfg(test)]
mod tests {
    use super::{cut, frames_of, seconds_of};
    use std::time::Duration;

    fn wav(channels: u16, frames: u16) -> Vec<u8> {
        let data: Vec<u8> = (0..frames * channels).flat_map(u16::to_le_bytes).collect();
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(
            &(4 + 8 + 16 + 8 + u32::try_from(data.len()).unwrap_or(0)).to_le_bytes(),
        );
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&1000_u32.to_le_bytes());
        out.extend_from_slice(&(1000 * u32::from(channels) * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16_u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn a_cut_keeps_the_format_and_the_frames_asked_for() -> Result<(), Box<dyn std::error::Error>> {
        let piece = cut(&wav(2, 100), 10, 20, 1 << 20)?;
        assert_eq!(
            (piece.frames, piece.start, piece.total),
            (20, 10, 100),
            "frames"
        );
        assert!(!piece.truncated, "as asked");
        assert_eq!(piece.wav.len(), 44 + 20 * 4, "header and samples");
        let riff = u32::from_le_bytes(piece.wav.get(4..8).ok_or("riff")?.try_into()?);
        assert_eq!(usize::try_from(riff)?, piece.wav.len() - 8, "RIFF size");
        let first = u16::from_le_bytes(piece.wav.get(44..46).ok_or("sample")?.try_into()?);
        assert_eq!(first, 20, "frame 10 of 2 channels starts at sample 20");
        Ok(())
    }

    #[test]
    fn a_cut_is_shortened_by_the_end_of_the_sound_and_by_the_size_cap()
    -> Result<(), Box<dyn std::error::Error>> {
        let end = cut(&wav(1, 100), 90, 50, 1 << 20)?;
        assert_eq!(
            (end.frames, end.truncated),
            (10, true),
            "the end of the sound"
        );
        let capped = cut(&wav(1, 100), 0, 100, 44 + 2 * 30)?;
        assert_eq!(
            (capped.frames, capped.truncated),
            (30, true),
            "the size cap"
        );
        assert_eq!(capped.wav.len(), 44 + 60, "within the cap");
        Ok(())
    }

    #[test]
    fn seconds_and_frames_convert_without_rounding_up() {
        assert_eq!(
            frames_of(Duration::from_millis(1500), 48_000),
            72_000,
            "frames"
        );
        assert!((seconds_of(72_000, 48_000) - 1.5).abs() < 1e-9, "seconds");
    }
}
