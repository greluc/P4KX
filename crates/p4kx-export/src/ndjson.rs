// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use p4kx_extraction::{
    Annotations, CancelToken, ConflictPolicy, DataForge, ExtractionError, JsonLayout, Record,
    RecordExportError, RecordExportOptions, RecordFailure, RecordJson, TreeBuilder,
};
use rayon::prelude::*;
use tempfile::NamedTempFile;

use crate::{NDJSON_FILE, RecordExportSummary, RecordForm};

const BATCH_LEN: usize = 4096;
const BATCH_BYTES: usize = 64 << 20;
const PART_LEN: usize = 64;
const READ_BUFFER_LEN: usize = 64 << 10;

/// One selected Record, in the order of the file.
struct Line<'r, 'f> {
    record: &'r Record<'f>,
}

/// The lines of one part of a batch and what happened to its Records.
struct Part<'r, 'f> {
    bytes: Vec<u8>,
    error_counts: Vec<usize>,
    failures: Vec<(&'r Record<'f>, RecordExportError)>,
}

struct Scratch<'f> {
    trees: TreeBuilder<'f>,
    json: RecordJson,
}

/// The NDJSON file being written: a temporary file with the size and CRC-32C written so far.
struct Output {
    file: BufWriter<NamedTempFile>,
    len: u64,
    crc: u32,
}

/// Writes the NDJSON form of a Record export, see [`crate::write_record_export`].
pub(crate) fn write(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> RecordExportSummary {
    let mut summary = RecordExportSummary {
        form: RecordForm::Ndjson,
        ..RecordExportSummary::default()
    };
    if let Err(error) = run(forge, records, target, options, &mut summary) {
        discard(&mut summary);
        summary.error = Some(error);
    }
    summary
}

fn run(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    target: &Path,
    options: &RecordExportOptions<'_>,
    summary: &mut RecordExportSummary,
) -> Result<(), ExtractionError> {
    p4kx_extraction::ensure_writable(target).map_err(ExtractionError::Installation)?;
    let destination = target.join(NDJSON_FILE);
    let existing = existing_file(&destination)?;
    if existing.is_some() && options.policy == ConflictPolicy::Skip {
        summary.skipped.push(NDJSON_FILE.to_owned());
        return Ok(());
    }
    let lines = ordered_lines(forge, records, summary);
    let mut output = Output::create(target)?;
    if !stream(&lines, &mut output, &destination, options, summary)? {
        return Ok(());
    }
    let kept = match existing {
        Some(len) if options.policy == ConflictPolicy::IfChanged && len == output.len => {
            file_crc(&destination)? == output.crc
        }
        _ => false,
    };
    if kept {
        summary.skipped.push(NDJSON_FILE.to_owned());
        summary.written = 0;
        return Ok(());
    }
    if output.persist(&destination, existing.is_some())? {
        summary.skipped.push(NDJSON_FILE.to_owned());
        discard(summary);
    }
    Ok(())
}

/// Builds and writes `lines` in batches of at most [`BATCH_LEN`] Records, sized from the previous
/// batch to hold about [`BATCH_BYTES`]; returns `false` when the run was cancelled.
fn stream(
    lines: &[Line<'_, '_>],
    output: &mut Output,
    destination: &Path,
    options: &RecordExportOptions<'_>,
    summary: &mut RecordExportSummary,
) -> Result<bool, ExtractionError> {
    let mut failed_lines = 0_usize;
    let mut start = 0;
    let mut batch_len = BATCH_LEN;
    while start < lines.len() {
        let end = start.saturating_add(batch_len).min(lines.len());
        let batch = lines.get(start..end).unwrap_or_default();
        start = end;
        if options.cancel.is_some_and(CancelToken::is_cancelled) {
            discard(summary);
            summary.cancelled = lines.len().saturating_sub(failed_lines);
            return Ok(false);
        }
        let parts: Vec<Part<'_, '_>> = batch
            .par_chunks(PART_LEN)
            .map_init(
                || Scratch {
                    trees: TreeBuilder::new(),
                    json: RecordJson::new(JsonLayout::Compact),
                },
                |scratch, lines| {
                    write_part(scratch, lines, options.progress, options.annotations())
                },
            )
            .collect();
        batch_len = next_batch_len(batch.len(), parts.iter().map(|part| part.bytes.len()).sum());
        for part in parts {
            output
                .append(&part.bytes)
                .map_err(|error| write_error(destination, error))?;
            failed_lines = failed_lines.saturating_add(part.failures.len());
            summary.written = summary.written.saturating_add(part.error_counts.len());
            for errors in part.error_counts {
                summary.error_values = summary.error_values.saturating_add(errors);
                summary.records_with_errors = summary
                    .records_with_errors
                    .saturating_add(usize::from(errors > 0));
            }
            summary.failures.extend(
                part.failures
                    .into_iter()
                    .map(|(record, error)| failure(record, error)),
            );
        }
    }
    Ok(true)
}

/// Returns how many Records the next batch takes after `len` Records gave `bytes`.
fn next_batch_len(len: usize, bytes: usize) -> usize {
    len.saturating_mul(BATCH_BYTES)
        .checked_div(bytes)
        .unwrap_or(BATCH_LEN)
        .clamp(PART_LEN, BATCH_LEN)
}

/// Forgets the Records counted as written, since the file is not put in place.
const fn discard(summary: &mut RecordExportSummary) {
    summary.written = 0;
    summary.error_values = 0;
    summary.records_with_errors = 0;
}

/// Returns the selected Records of `forge` in Record order ([`DataForge::path_order`]), each once,
/// with those whose record path cannot be read last in definition order; Records of another
/// DataForge become failures.
fn ordered_lines<'r, 'f>(
    forge: &DataForge<'_>,
    records: &'r [Record<'f>],
    summary: &mut RecordExportSummary,
) -> Vec<Line<'r, 'f>> {
    let mut selected: Vec<Option<&'r Record<'f>>> = vec![None; forge.record_count()];
    for record in records {
        match selected.get_mut(record.index() as usize) {
            Some(slot) if record.is_from(forge) => *slot = Some(record),
            _ => summary
                .failures
                .push(failure(record, RecordExportError::OtherDataForge)),
        }
    }
    let mut lines: Vec<Line<'r, 'f>> = forge
        .path_order()
        .iter()
        .filter_map(|index| selected.get_mut(*index as usize).and_then(Option::take))
        .map(|record| Line { record })
        .collect();
    lines.extend(selected.into_iter().flatten().map(|record| Line { record }));
    lines
}
fn write_part<'r, 'f>(
    scratch: &mut Scratch<'f>,
    lines: &[Line<'r, 'f>],
    progress: Option<&(dyn Fn() + Sync)>,
    annotations: Annotations<'_>,
) -> Part<'r, 'f> {
    let mut part = Part {
        bytes: Vec::new(),
        error_counts: Vec::with_capacity(lines.len()),
        failures: Vec::new(),
    };
    for line in lines {
        match scratch.trees.build(line.record) {
            Ok(tree) => {
                let start = part.bytes.len();
                match scratch
                    .json
                    .write_annotated(&tree, annotations, &mut part.bytes)
                {
                    Ok(()) => {
                        part.bytes.push(b'\n');
                        part.error_counts.push(tree.error_count());
                    }
                    Err(error) => {
                        part.bytes.truncate(start);
                        part.failures.push((
                            line.record,
                            ExtractionError::Write {
                                path: PathBuf::from(NDJSON_FILE),
                                source: error,
                            }
                            .into(),
                        ));
                    }
                }
            }
            Err(error) => part.failures.push((line.record, error.into())),
        }
        if let Some(progress) = progress {
            progress();
        }
    }
    part
}

impl Output {
    fn create(target: &Path) -> Result<Self, ExtractionError> {
        let file = tempfile::Builder::new()
            .prefix(".p4kx-")
            .suffix(".part")
            .tempfile_in(target)
            .map_err(|source| ExtractionError::Write {
                path: target.to_path_buf(),
                source,
            })?;
        Ok(Self {
            file: BufWriter::with_capacity(READ_BUFFER_LEN, file),
            len: 0,
            crc: 0,
        })
    }

    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file.write_all(bytes)?;
        self.len = self.len.saturating_add(bytes.len() as u64);
        self.crc = crc32c::crc32c_append(self.crc, bytes);
        Ok(())
    }

    /// Puts the file in place at `destination`, replacing a file there only when `replace`;
    /// returns `true` when a file that appeared meanwhile is kept instead.
    fn persist(self, destination: &Path, replace: bool) -> Result<bool, ExtractionError> {
        let file = self
            .file
            .into_inner()
            .map_err(|error| write_error(destination, error.into_error()))?;
        let persisted = if replace {
            file.persist(destination)
        } else {
            file.persist_noclobber(destination)
        };
        match persisted {
            Ok(_) => Ok(false),
            Err(_) if !replace && destination.exists() => Ok(true),
            Err(error) => Err(write_error(destination, error.error)),
        }
    }
}

/// Returns the size of the file at `path`, or `None` when there is none; a folder or link there
/// is an error.
fn existing_file(path: &Path) -> Result<Option<u64>, ExtractionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(metadata.len())),
        Ok(_) => Err(write_error(
            path,
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a folder or link is in the way",
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(read_existing_error(path, source)),
    }
}

fn file_crc(path: &Path) -> Result<u32, ExtractionError> {
    let mut buffer = vec![0; READ_BUFFER_LEN];
    let mut file = File::open(path).map_err(|source| read_existing_error(path, source))?;
    let mut crc = 0;
    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok(crc),
            Ok(count) => crc = crc32c::crc32c_append(crc, buffer.get(..count).unwrap_or_default()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(source) => return Err(read_existing_error(path, source)),
        }
    }
}

fn failure(record: &Record<'_>, error: RecordExportError) -> RecordFailure {
    RecordFailure::new(
        record.guid(),
        record.record_path().unwrap_or_default().to_owned(),
        error,
    )
}

fn write_error(path: &Path, source: io::Error) -> ExtractionError {
    ExtractionError::Write {
        path: path.to_path_buf(),
        source,
    }
}

fn read_existing_error(path: &Path, source: io::Error) -> ExtractionError {
    ExtractionError::ReadExisting {
        path: path.to_path_buf(),
        source,
    }
}
