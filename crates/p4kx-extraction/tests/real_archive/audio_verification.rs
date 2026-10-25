// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A sample of the WEMs and soundbanks of a real Archive converts without a panic; the share that
//! converts is reported per codec. Reads unencrypted Entries only and needs no Archive key.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use p4kx_extraction::{
    Archive, AudioError, Bank, ConversionError, Entry, Soundbank, Wem, WemFormat,
};
use p4kx_testkit::{real_archive_b_path, real_archive_path};
use rayon::prelude::*;

use crate::TestResult;

const LOOSE_SAMPLE: usize = 1_500;
const BANK_SAMPLE: usize = 150;
const WEMS_PER_BANK: usize = 12;
const WRITTEN_PER_CODEC: usize = 3;
const EXTRACTED_BANKS: usize = 3;

fn has_extension(entry: &Entry, extension: &[u8]) -> bool {
    let path = entry.path().as_bytes();
    path.len() > extension.len()
        && path
            .get(path.len() - extension.len()..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(extension))
}

fn stride_sample<'a>(entries: &[&'a Entry], count: usize) -> Vec<&'a Entry> {
    let stride = (entries.len() / count).max(1);
    entries.iter().copied().step_by(stride).collect()
}

fn read_entry(archive: &Archive, entry: &Entry) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

#[derive(Default)]
struct CodecTally {
    sampled: usize,
    ogg_ok: usize,
    ogg_not_offered: usize,
    wav_ok: usize,
    wav_not_offered: usize,
    panics: usize,
    errors: Vec<String>,
    ogg: Vec<Vec<u8>>,
    wav: Vec<Vec<u8>>,
}

#[derive(Default)]
struct Tally {
    codecs: BTreeMap<String, CodecTally>,
    plugin_media: usize,
    prefetch: usize,
    encrypted_skipped: usize,
    unreadable: Vec<String>,
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        self.plugin_media += other.plugin_media;
        self.prefetch += other.prefetch;
        self.encrypted_skipped += other.encrypted_skipped;
        self.unreadable.extend(other.unreadable);
        for (codec, theirs) in other.codecs {
            let ours = self.codecs.entry(codec).or_default();
            ours.sampled += theirs.sampled;
            ours.ogg_ok += theirs.ogg_ok;
            ours.ogg_not_offered += theirs.ogg_not_offered;
            ours.wav_ok += theirs.wav_ok;
            ours.wav_not_offered += theirs.wav_not_offered;
            ours.panics += theirs.panics;
            ours.errors.extend(theirs.errors);
            if ours.ogg.len() < WRITTEN_PER_CODEC {
                ours.ogg.extend(theirs.ogg);
            }
            if ours.wav.len() < WRITTEN_PER_CODEC {
                ours.wav.extend(theirs.wav);
            }
        }
        self
    }

    fn convert(&mut self, label: &str, bytes: &[u8]) {
        let wem = match Wem::parse(bytes) {
            Ok(wem) => wem,
            Err(AudioError::PluginMedia | AudioError::MidiMedia) => {
                self.plugin_media += 1;
                return;
            }
            Err(error) => {
                self.unreadable.push(format!("{label}: {error}"));
                return;
            }
        };
        let tally = self.codecs.entry(wem.codec().to_string()).or_default();
        tally.sampled += 1;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            (
                wem.write_ogg(&mut Vec::new()),
                wem.write_wav(&mut Vec::new()),
            )
        }));
        let Ok((ogg, wav)) = outcome else {
            tally.panics += 1;
            tally.errors.push(format!("{label}: panic"));
            return;
        };
        match ogg {
            Ok(_) => {
                tally.ogg_ok += 1;
                if tally.ogg.len() < WRITTEN_PER_CODEC {
                    let mut out = Vec::new();
                    if wem.write_ogg(&mut out).is_ok() {
                        tally.ogg.push(out);
                    }
                }
            }
            Err(AudioError::UnsupportedOggCodec(_)) => tally.ogg_not_offered += 1,
            Err(error) => tally.errors.push(format!("{label}: Ogg: {error}")),
        }
        match wav {
            Ok(_) => {
                tally.wav_ok += 1;
                if tally.wav.len() < WRITTEN_PER_CODEC {
                    let mut out = Vec::new();
                    if wem.write_wav(&mut out).is_ok() {
                        tally.wav.push(out);
                    }
                }
            }
            Err(AudioError::UnsupportedCodec(_)) => tally.wav_not_offered += 1,
            Err(error) => tally.errors.push(format!("{label}: WAV: {error}")),
        }
    }
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        f64::from(u32::try_from(part).unwrap_or(u32::MAX)) * 100.0
            / f64::from(u32::try_from(whole).unwrap_or(u32::MAX))
    }
}

fn render(name: &str, tally: &Tally, banks: usize) -> String {
    let mut text = format!(
        "{name}: {banks} soundbanks sampled; {} plug-in media, {} prefetched starts, {} encrypted Entries skipped\n",
        tally.plugin_media, tally.prefetch, tally.encrypted_skipped
    );
    for (codec, row) in &tally.codecs {
        let _ = writeln!(
            text,
            "  {codec}: {} sampled; Ogg {} ({:.1} %, {} not offered); WAV {} ({:.1} %, {} not offered); {} errors, {} panics",
            row.sampled,
            row.ogg_ok,
            percent(row.ogg_ok, row.sampled),
            row.ogg_not_offered,
            row.wav_ok,
            percent(row.wav_ok, row.sampled),
            row.wav_not_offered,
            row.errors.len(),
            row.panics
        );
    }
    text
}

fn write_outputs(directory: &Path, tally: &Tally) -> TestResult {
    for (codec, row) in &tally.codecs {
        let safe: String = codec
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        for (index, ogg) in row.ogg.iter().enumerate() {
            fs::write(directory.join(format!("{safe}-{index}.ogg")), ogg)?;
        }
        for (index, wav) in row.wav.iter().enumerate() {
            fs::write(directory.join(format!("{safe}-{index}.wav")), wav)?;
        }
    }
    Ok(())
}

fn verify(name: &str, path: &Path) -> TestResult {
    let archive = Archive::open(path)?;
    let entries = archive.entries();
    let loose: Vec<&Entry> = entries
        .iter()
        .filter(|entry| has_extension(entry, b".wem"))
        .collect();
    let bank_entries: Vec<&Entry> = entries
        .iter()
        .filter(|entry| has_extension(entry, b".bnk"))
        .collect();
    assert!(
        !loose.is_empty() || !bank_entries.is_empty(),
        "{name}: no audio Entries"
    );
    let loose_sample = stride_sample(&loose, LOOSE_SAMPLE);
    let bank_sample = stride_sample(&bank_entries, BANK_SAMPLE);

    let from_loose = loose_sample
        .par_iter()
        .map(|entry| {
            let mut tally = Tally::default();
            if entry.is_encrypted() {
                tally.encrypted_skipped += 1;
            } else {
                match read_entry(&archive, entry) {
                    Ok(bytes) => tally.convert(entry.path(), &bytes),
                    Err(error) => tally.unreadable.push(format!("{}: {error}", entry.path())),
                }
            }
            tally
        })
        .reduce(Tally::default, Tally::merge);
    let from_banks = bank_sample
        .par_iter()
        .map(|entry| convert_bank(&archive, entry))
        .reduce(Tally::default, Tally::merge);
    let tally = from_loose.merge(from_banks);

    let report = render(name, &tally, bank_sample.len());
    let directory = tempfile::tempdir()?;
    write_outputs(directory.path(), &tally)?;
    fs::write(directory.path().join("report.txt"), &report)?;
    crate::report(&report);
    assert_clean(name, &tally);
    Ok(())
}

fn assert_clean(name: &str, tally: &Tally) {
    assert!(
        tally.unreadable.is_empty(),
        "{name}: unreadable: {:#?}",
        tally.unreadable
    );
    assert!(!tally.codecs.is_empty(), "{name}: no WEM was sampled");
    for (codec, row) in &tally.codecs {
        assert_eq!(row.panics, 0, "{name}: {codec} panicked");
        assert!(
            row.errors.is_empty(),
            "{name}: {codec}: {} errors: {:#?}",
            row.errors.len(),
            row.errors.iter().take(10).collect::<Vec<_>>()
        );
        assert!(
            row.wav_ok + row.wav_not_offered == row.sampled,
            "{name}: {codec}: every sampled WEM converts to WAV or reports an unsupported codec"
        );
    }
}

fn convert_bank(archive: &Archive, entry: &Entry) -> Tally {
    let mut tally = Tally::default();
    if entry.is_encrypted() {
        tally.encrypted_skipped += 1;
        return tally;
    }
    let bytes = match read_entry(archive, entry) {
        Ok(bytes) => bytes,
        Err(error) => {
            tally.unreadable.push(format!("{}: {error}", entry.path()));
            return tally;
        }
    };
    let bank = match Soundbank::parse(bytes.as_slice()) {
        Ok(bank) => bank,
        Err(error) => {
            tally.unreadable.push(format!("{}: {error}", entry.path()));
            return tally;
        }
    };
    let wems: Vec<_> = bank.wems().collect();
    let stride = (wems.len() / WEMS_PER_BANK).max(1);
    for wem in wems.iter().step_by(stride) {
        if wem.is_prefetch() {
            tally.prefetch += 1;
            continue;
        }
        let label = format!("{} WEM {}", entry.path(), wem.id());
        tally.convert(&label, wem.bytes());
    }
    tally
}

fn extract_banks(path: &Path) -> TestResult {
    let archive = Archive::open(path)?;
    let banks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| has_extension(entry, b".bnk") && !entry.is_encrypted())
        .filter(|entry| entry.uncompressed_size() > 0)
        .collect();
    let target = tempfile::tempdir()?;
    for entry in stride_sample(&banks, EXTRACTED_BANKS) {
        let bank = Bank::from_archive(&archive, entry.path())?;
        let listed = bank.soundbank().wems().len();
        let extraction = bank.extract_wems(target.path(), WemFormat::Ogg)?;
        assert_eq!(
            extraction.written.len() + extraction.failures.len(),
            listed,
            "{}: every embedded WEM is written or reported",
            entry.path()
        );
        let unexpected: Vec<_> = extraction
            .failures
            .iter()
            .filter(|failure| {
                !matches!(
                    failure.error,
                    ConversionError::PrefetchedWem { .. }
                        | ConversionError::Audio {
                            source: AudioError::PluginMedia | AudioError::MidiMedia,
                            ..
                        }
                )
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "{}: unexpected failures: {unexpected:?}",
            entry.path()
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_every_codec_converts_and_banks_list_readable_wems() -> TestResult {
    let path = real_archive_path()?;
    verify("primary", &path)?;
    extract_banks(&path)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K_B and pass --ignored"]
fn a_sample_of_the_second_archive_converts_and_banks_list_readable_wems() -> TestResult {
    let Some(path) = real_archive_b_path() else {
        crate::report("P4KX_DATA_P4K_B is not set; the second Archive is skipped");
        return Ok(());
    };
    verify("second", &path)?;
    extract_banks(&path)
}
