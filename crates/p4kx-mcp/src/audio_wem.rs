// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::{AudioError, Bank, ConversionError, EmbeddedWem, Sound, Wem};

use crate::archives::Opened;
use crate::audio_convert as convert;
use crate::audio_params::{MAX_NAME_LEN, SoundbankParams, WemParams};
use crate::audio_shapes::{LocationShape, SoundbankWemsResult, WemInfoResult, WemShape};
use crate::budget::fill;
use crate::error::{ToolError, capped};
use crate::models::converted;
use crate::page::Page;

const MAX_LOCATIONS: usize = 50;

/// What the header of a WEM states about the sound.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Facts {
    pub(crate) codec: &'static str,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
    pub(crate) sample_count: Option<u64>,
}

/// What the tools do with a parsed WEM, whichever buffer holds its bytes.
pub(crate) trait WemOps {
    /// Describes the WEM.
    fn shape(&self, id: Option<u32>, size: u64) -> WemShape;
    /// Returns what the header states.
    fn facts(&self) -> Facts;
    /// Decodes the whole sound to a WAV of at most `limit` bytes.
    fn wav(&self, limit: u64, name: &str) -> Result<Vec<u8>, ToolError>;
}

impl<B: AsRef<[u8]>> WemOps for Wem<B> {
    fn shape(&self, id: Option<u32>, size: u64) -> WemShape {
        WemShape {
            id,
            size,
            prefetch: false,
            codec: Some(self.codec().id()),
            codec_name: Some(self.codec().to_string()),
            channels: Some(self.channels()),
            sample_rate: Some(self.sample_rate()),
            sample_count: self.sample_count(),
            duration_seconds: self.duration().map(|duration| duration.as_secs_f64()),
            looping: Some(self.loop_points().is_some()),
            plugin_media: false,
            error: None,
        }
    }

    fn facts(&self) -> Facts {
        Facts {
            codec: self.codec().id(),
            channels: self.channels(),
            sample_rate: self.sample_rate(),
            sample_count: self.sample_count(),
        }
    }

    fn wav(&self, limit: u64, name: &str) -> Result<Vec<u8>, ToolError> {
        let mut out = Capped {
            bytes: Vec::new(),
            limit: usize::try_from(limit).unwrap_or(usize::MAX),
            exceeded: false,
        };
        match self.write_wav(&mut out) {
            Ok(_) => Ok(out.bytes),
            Err(_) if out.exceeded => Err(ToolError::ExcerptSource { limit }),
            Err(source) => Err(audio_error(name, source)),
        }
    }
}

/// A buffer that refuses bytes beyond its limit.
struct Capped {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl Write for Capped {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(buf.len()) > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("the decoded sound exceeds its cap"));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn audio_error(name: &str, source: AudioError) -> ToolError {
    ToolError::Convert {
        path: name.to_owned(),
        what: "a sound to decode",
        source: Box::new(ConversionError::Audio {
            sound: name.to_owned(),
            source,
        }),
    }
}

/// A WEM that cannot be parsed as a sound.
fn unreadable(id: Option<u32>, size: u64, error: Option<&AudioError>) -> WemShape {
    let plugin = matches!(error, Some(AudioError::PluginMedia));
    WemShape {
        id,
        size,
        prefetch: error.is_none(),
        codec: None,
        codec_name: None,
        channels: None,
        sample_rate: None,
        sample_count: None,
        duration_seconds: None,
        looping: None,
        plugin_media: plugin,
        error: error.filter(|_| !plugin).map(ToString::to_string),
    }
}

fn embedded_shape(wem: &EmbeddedWem<'_>) -> WemShape {
    let size = wem.bytes().len() as u64;
    let id = Some(wem.id());
    if wem.is_prefetch() {
        return unreadable(id, size, None);
    }
    match wem.parse() {
        Ok(parsed) => parsed.shape(id, size),
        Err(error) => unreadable(id, size, Some(&error)),
    }
}

/// Returns one page of the WEMs the soundbank at `params.soundbank` embeds, each described by its
/// header.
pub(crate) fn soundbank_wems(
    opened: &Opened,
    params: &SoundbankParams,
    page: Page,
) -> Result<SoundbankWemsResult, ToolError> {
    let path = capped("soundbank", &params.soundbank, MAX_NAME_LEN * 4)?;
    let bank = Bank::from_archive(opened.archive(), path)
        .map_err(|error| converted(path, "a soundbank", error))?;
    let soundbank = bank.soundbank();
    let wems: Vec<EmbeddedWem<'_>> = soundbank.wems().collect();
    let items = page.slice(&wems).iter().map(|wem| Ok(embedded_shape(wem)));
    let (listed, next_cursor) = fill(page, wems.len(), items, |wem: &WemShape| {
        format!("WEM {}", wem.id.unwrap_or_default())
    })?;
    Ok(SoundbankWemsResult {
        archive: opened.archive().path().display().to_string(),
        soundbank: bank.name().to_owned(),
        version: soundbank.header().version,
        bank_id: soundbank.header().id,
        total: wems.len(),
        wems: listed,
        next_cursor,
    })
}

/// Where the bytes of a WEM are read from.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    /// A loose WEM Entry.
    Entry(String),
    /// A WEM embedded in the soundbank Entry.
    Embedded {
        /// The Entry path of the soundbank.
        bank: String,
        /// The WEM ID.
        id: u32,
    },
}

/// A place that holds a WEM.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub(crate) source: Source,
    pub(crate) location: LocationShape,
}

/// The places that hold the WEM `params` names, loose Entries first, with its ID where there is
/// one.
pub(crate) fn candidates(
    opened: &Opened,
    wem_id: Option<u32>,
    path: Option<&str>,
) -> Result<(Option<u32>, Vec<Candidate>), ToolError> {
    match (wem_id, path) {
        (Some(id), None) => {
            let index = opened.audio_index();
            let places: Vec<Candidate> = index
                .loose_wem(id)
                .filter_map(|place| convert::location(&place))
                .map(|location| Candidate {
                    source: Source::Entry(location.path.clone()),
                    location,
                })
                .chain(
                    index
                        .embedded_wem(id)
                        .filter_map(|place| convert::location(&place))
                        .map(|location| Candidate {
                            source: Source::Embedded {
                                bank: location.path.clone(),
                                id,
                            },
                            location,
                        }),
                )
                .collect();
            if places.is_empty() {
                return Err(ToolError::UnknownWem(id));
            }
            Ok((Some(id), places))
        }
        (None, Some(path)) => {
            let path = capped("path", path, MAX_NAME_LEN * 4)?;
            let index = opened.index();
            let entry = index
                .find(path)
                .ok_or_else(|| ToolError::UnknownEntry(path.to_owned()))?;
            let stem = entry.path().rsplit(['/', '\\']).next().and_then(|name| {
                name.strip_suffix(".wem")
                    .or_else(|| name.strip_suffix(".WEM"))
            });
            let location = LocationShape {
                kind: "entry",
                path: entry.path().to_owned(),
                language: None,
                offset: None,
                len: entry.uncompressed_size(),
            };
            Ok((
                stem.and_then(|stem| stem.parse().ok()),
                vec![Candidate {
                    source: Source::Entry(location.path.clone()),
                    location,
                }],
            ))
        }
        _ => Err(ToolError::WemSelector),
    }
}

/// What reading a place found.
pub(crate) enum Held<'a> {
    /// A WEM that parsed.
    Parsed(&'a dyn WemOps),
    /// The prefetched start of a streamed WEM.
    Prefetch,
    /// Bytes that are no sound.
    Unreadable(AudioError),
}

/// Reads the WEM at `candidate` and hands what was found to `visit`.
pub(crate) fn read<R>(
    opened: &Opened,
    candidate: &Candidate,
    visit: impl FnOnce(Held<'_>) -> Result<R, ToolError>,
) -> Result<R, ToolError> {
    match &candidate.source {
        Source::Entry(path) => {
            let sound = Sound::from_archive(opened.archive(), path)
                .map_err(|error| converted(path, "a WEM", error))?;
            visit(Held::Parsed(sound.wem()))
        }
        Source::Embedded { bank, id } => {
            let held = Bank::from_archive(opened.archive(), bank)
                .map_err(|error| converted(bank, "a soundbank", error))?;
            let Some(wem) = held.soundbank().wem(*id) else {
                return Err(ToolError::UnknownWem(*id));
            };
            if wem.is_prefetch() {
                return visit(Held::Prefetch);
            }
            match wem.parse() {
                Ok(parsed) => visit(Held::Parsed(&parsed)),
                Err(error) => visit(Held::Unreadable(error)),
            }
        }
    }
}

/// Describes the WEM `params` names from the first place that holds all of it.
pub(crate) fn info(opened: &Opened, params: &WemParams) -> Result<WemInfoResult, ToolError> {
    let (id, places) = candidates(opened, params.wem_id, params.path.as_deref())?;
    let mut first: Option<(LocationShape, WemShape)> = None;
    let mut failure: Option<ToolError> = None;
    for place in &places {
        let size = place.location.len;
        let outcome = read(opened, place, |held| {
            Ok(match held {
                Held::Parsed(wem) => (wem.shape(id, size), true),
                Held::Prefetch => (unreadable(id, size, None), false),
                Held::Unreadable(error) => (unreadable(id, size, Some(&error)), false),
            })
        });
        match outcome {
            Ok((shape, true)) => {
                first = Some((place.location.clone(), shape));
                break;
            }
            Ok((shape, false)) => {
                first.get_or_insert_with(|| (place.location.clone(), shape));
            }
            Err(error) => {
                failure.get_or_insert(error);
            }
        }
    }
    let Some((described, wem)) = first else {
        return Err(failure.unwrap_or(ToolError::WemSelector));
    };
    Ok(WemInfoResult {
        archive: opened.archive().path().display().to_string(),
        wem_id: id,
        locations_total: places.len(),
        locations: places
            .into_iter()
            .take(MAX_LOCATIONS)
            .map(|place| place.location)
            .collect(),
        described,
        wem,
    })
}
