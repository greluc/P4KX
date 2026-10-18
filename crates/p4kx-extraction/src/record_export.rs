use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use p4kx_dataforge::{DataForge, Record, RecordXml};
use rayon::prelude::*;

use crate::dependent::split_dependent;
use crate::entry_path::relative_path;
use crate::record_files::{RecordFiles, fold};
use crate::write::{create_folders, create_temporary, read_existing_error, write_error};
use crate::{
    ConflictPolicy, ExtractionError, RecordExportError, RecordExportOptions, RecordExportReport,
    RecordFailure,
};

const READ_BUFFER_LEN: usize = 64 << 10;

/// Writes `records` of `forge` as Record XML, one file per Record, below the existing folder
/// `target`.
///
/// A Record alone at its record path is written to `<record path>.xml`; Records that share one
/// go into the folder `<record path>/` as `<name>.xml`, with the GUID added when names collide.
/// Files that already exist are handled by the [`ConflictPolicy`] of `options`, where
/// [`ConflictPolicy::IfChanged`] compares size and CRC-32C with the new XML. Records are built
/// and written in parallel with the same files and report as a sequential run in the order of
/// `records`. Record paths that could leave `target` are rejected and existing links below
/// `target` are never followed; a failed or cancelled Record leaves no partial file. The
/// `records` must come from `forge`.
#[must_use]
pub fn export_records(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> RecordExportReport {
    let plan = RecordFiles::plan(forge);
    let files = select_files(&plan, records);
    let (independent, dependent) = split_dependent(files.iter().map(|file| match file {
        Selected::File(file) => *file,
        Selected::Taken(_) | Selected::Unsafe => "",
    }));
    let writer = Writer {
        target,
        policy: options.policy,
        cancel: options.cancel,
    };
    let run = |scratch: &mut Scratch, index: usize| {
        let outcome = match (records.get(index), files.get(index)) {
            (Some(record), Some(Selected::File(file))) => writer.write(record, file, scratch),
            (_, Some(Selected::Taken(file))) => {
                RecordOutcome::Failed(RecordExportError::FileTaken {
                    path: (*file).to_owned(),
                })
            }
            _ => RecordOutcome::Failed(ExtractionError::UnsafePath.into()),
        };
        (index, outcome)
    };
    let mut outcomes: Vec<_> = independent
        .into_par_iter()
        .map_init(Scratch::default, run)
        .collect();
    let mut scratch = Scratch::default();
    outcomes.extend(dependent.into_iter().map(|index| run(&mut scratch, index)));
    outcomes.sort_unstable_by_key(|(index, _)| *index);
    summarise(records, outcomes)
}

/// Returns the planned file of every selected Record; a file an earlier Record of the selection
/// takes, ignoring case, is taken.
fn select_files<'p>(plan: &'p RecordFiles, records: &[Record<'_>]) -> Vec<Selected<'p>> {
    let mut taken = HashSet::new();
    records
        .iter()
        .map(|record| match plan.file(record) {
            None => Selected::Unsafe,
            Some(file) if taken.insert(fold(file).collect::<String>()) => Selected::File(file),
            Some(file) => Selected::Taken(file),
        })
        .collect()
}

fn summarise(records: &[Record<'_>], outcomes: Vec<(usize, RecordOutcome)>) -> RecordExportReport {
    let mut report = RecordExportReport::default();
    for (index, outcome) in outcomes {
        let errors = match outcome {
            RecordOutcome::Written { errors } => {
                report.written += 1;
                errors
            }
            RecordOutcome::Kept { file, errors } => {
                report.skipped.push(file);
                errors
            }
            RecordOutcome::Cancelled => {
                report.cancelled += 1;
                0
            }
            RecordOutcome::Failed(error) => {
                let record = records.get(index);
                report.failures.push(RecordFailure {
                    guid: record.map(Record::guid).unwrap_or_default(),
                    record_path: record
                        .and_then(|record| record.record_path().ok())
                        .unwrap_or_default()
                        .to_owned(),
                    error,
                });
                0
            }
        };
        report.error_values += errors;
        report.records_with_errors += usize::from(errors > 0);
    }
    report
}

/// The file a selected Record is written to, if it can be.
#[derive(Clone, Copy)]
enum Selected<'p> {
    File(&'p str),
    Taken(&'p str),
    Unsafe,
}

enum RecordOutcome {
    Written { errors: usize },
    Kept { file: String, errors: usize },
    Cancelled,
    Failed(RecordExportError),
}

#[derive(Default)]
struct Scratch {
    xml: RecordXml,
    bytes: Vec<u8>,
    read: Vec<u8>,
}

struct Writer<'a> {
    target: &'a Path,
    policy: ConflictPolicy,
    cancel: Option<&'a crate::CancelToken>,
}

impl Writer<'_> {
    fn write(&self, record: &Record<'_>, file: &str, scratch: &mut Scratch) -> RecordOutcome {
        if self.cancel.is_some_and(crate::CancelToken::is_cancelled) {
            return RecordOutcome::Cancelled;
        }
        match self.try_write(record, file, scratch) {
            Ok(outcome) => outcome,
            Err(error) => RecordOutcome::Failed(error),
        }
    }

    fn try_write(
        &self,
        record: &Record<'_>,
        file: &str,
        scratch: &mut Scratch,
    ) -> Result<RecordOutcome, RecordExportError> {
        let relative = relative_path(file).ok_or(ExtractionError::UnsafePath)?;
        let name = relative.file_name().ok_or(ExtractionError::UnsafePath)?;
        let tree = record.tree()?;
        let errors = tree.errors().count();
        let folder = create_folders(self.target, relative.parent())?;
        let destination = folder.join(name);
        scratch.bytes.clear();
        scratch
            .xml
            .write(&tree, &mut scratch.bytes)
            .map_err(|source| write_error(destination.clone(), source))?;
        if self.keep(&destination, scratch)? {
            return Ok(RecordOutcome::Kept {
                file: file.to_owned(),
                errors,
            });
        }
        let (temporary, mut part) = create_temporary(&folder)?;
        let written = part
            .write_all(&scratch.bytes)
            .map_err(|source| write_error(temporary.clone(), source))
            .and_then(|()| {
                drop(part);
                fs::rename(&temporary, &destination)
                    .map_err(|source| write_error(destination.clone(), source))
            });
        if let Err(error) = written {
            let _ignored = fs::remove_file(&temporary);
            return Err(error.into());
        }
        Ok(RecordOutcome::Written { errors })
    }

    fn keep(&self, destination: &Path, scratch: &mut Scratch) -> Result<bool, ExtractionError> {
        let metadata = match fs::symlink_metadata(destination) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(source) => return Err(read_existing_error(destination, source)),
        };
        if !metadata.is_file() {
            return Err(write_error(
                destination.to_path_buf(),
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "a folder or link is in the way",
                ),
            ));
        }
        match self.policy {
            ConflictPolicy::Skip => Ok(true),
            ConflictPolicy::Overwrite => Ok(false),
            ConflictPolicy::IfChanged => Ok(metadata.len() == scratch.bytes.len() as u64
                && file_crc(destination, &mut scratch.read)? == crc32c::crc32c(&scratch.bytes)),
        }
    }
}

fn file_crc(path: &Path, buffer: &mut Vec<u8>) -> Result<u32, ExtractionError> {
    buffer.resize(READ_BUFFER_LEN, 0);
    let mut file = File::open(path).map_err(|source| read_existing_error(path, source))?;
    let mut crc = 0;
    loop {
        match file.read(buffer) {
            Ok(0) => return Ok(crc),
            Ok(count) => crc = crc32c::crc32c_append(crc, buffer.get(..count).unwrap_or_default()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(source) => return Err(read_existing_error(path, source)),
        }
    }
}
