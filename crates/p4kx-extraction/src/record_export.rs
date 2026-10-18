use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use p4kx_dataforge::{DataForge, Record, RecordXml, TreeBuilder};
use rayon::prelude::*;

use crate::dependent::split_dependent;
use crate::entry_path::relative_path;
use crate::record_files::{RecordFiles, fold};
use crate::record_format::RecordFileFormat;
use crate::write::{create_folders, create_temporary, read_existing_error, write_error};
use crate::{
    ConflictPolicy, ExtractionError, RecordExportError, RecordExportOptions, RecordExportReport,
    RecordFailure,
};

const READ_BUFFER_LEN: usize = 64 << 10;

/// Writes `records` of `forge` as Record XML, one file per Record, below the existing folder
/// `target`; [`export_record_files`] with [`RecordXml`].
#[must_use]
pub fn export_records(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> RecordExportReport {
    export_record_files::<RecordXml>(forge, records, target, options)
}

/// Writes `records` of `forge` in the format `F`, one file per Record, below the existing folder
/// `target`.
///
/// A Record alone at its record path is written to `<record path><extension>`; Records that
/// share one go into the folder `<record path>/` as `<name><extension>`, with the GUID added when
/// names collide (ADR-0008). Files that already exist are handled by the [`ConflictPolicy`] of
/// `options`, where [`ConflictPolicy::IfChanged`] compares size and CRC-32C with the new file.
/// Records are built and written in parallel with the same files and report as a sequential run
/// in the order of `records`. Record paths that could leave `target` are rejected and existing
/// links below `target` are never followed; a failed or cancelled Record leaves no partial file.
/// A Record of another DataForge than `forge` fails with [`RecordExportError::OtherDataForge`].
#[must_use]
pub fn export_record_files<F: RecordFileFormat>(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> RecordExportReport {
    let cancel = options.cancel;
    let run = RecordRun {
        policy: options.policy,
        stop: &|| cancel.is_some_and(crate::CancelToken::is_cancelled),
        ended: &|| {},
    };
    write_records::<F>(forge, records, target, &run)
}

/// How a run writes Record files: the Conflict policy, whether to stop before the next Record,
/// and what to call once a Record ended.
pub(crate) struct RecordRun<'a> {
    pub(crate) policy: ConflictPolicy,
    pub(crate) stop: &'a (dyn Fn() -> bool + Sync),
    pub(crate) ended: &'a (dyn Fn() + Sync),
}

/// Writes `records` of `forge` in the format `F` like [`export_record_files`], under the control
/// of `run`.
pub(crate) fn write_records<'f, F: RecordFileFormat>(
    forge: &DataForge<'_>,
    records: &[Record<'f>],
    target: &Path,
    run: &RecordRun<'_>,
) -> RecordExportReport {
    let plan = RecordFiles::plan(forge, F::EXTENSION);
    let files = select_files(forge, &plan, records);
    let (independent, dependent) = split_dependent(files.iter().map(|file| match file {
        Selected::File(file) => *file,
        Selected::Taken(_) | Selected::Unsafe => "",
    }));
    let writer = Writer {
        target,
        policy: run.policy,
        stop: run.stop,
    };
    let ended = run.ended;
    let run = |scratch: &mut Scratch<'f, F>, index: usize| {
        let outcome = match (records.get(index), files.get(index)) {
            (Some(record), _) if !record.is_from(forge) => {
                RecordOutcome::Failed(RecordExportError::OtherDataForge)
            }
            (Some(record), Some(Selected::File(file))) => writer.write(record, file, scratch),
            (_, Some(Selected::Taken(file))) => {
                RecordOutcome::Failed(RecordExportError::FileTaken {
                    path: (*file).to_owned(),
                })
            }
            _ => RecordOutcome::Failed(ExtractionError::UnsafePath.into()),
        };
        ended();
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
/// takes, ignoring case, is taken, and a Record of another DataForge gets none.
fn select_files<'p>(
    forge: &DataForge<'_>,
    plan: &'p RecordFiles,
    records: &[Record<'_>],
) -> Vec<Selected<'p>> {
    let mut taken = HashSet::new();
    records
        .iter()
        .map(|record| match plan.file(record) {
            _ if !record.is_from(forge) => Selected::Unsafe,
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

impl<F: Default> Default for Scratch<'_, F> {
    fn default() -> Self {
        Self {
            trees: TreeBuilder::default(),
            format: F::default(),
            bytes: Vec::new(),
            read: Vec::new(),
        }
    }
}

struct Scratch<'f, F> {
    trees: TreeBuilder<'f>,
    format: F,
    bytes: Vec<u8>,
    read: Vec<u8>,
}

struct Writer<'a> {
    target: &'a Path,
    policy: ConflictPolicy,
    stop: &'a (dyn Fn() -> bool + Sync),
}

impl Writer<'_> {
    fn write<'f, F: RecordFileFormat>(
        &self,
        record: &Record<'f>,
        file: &str,
        scratch: &mut Scratch<'f, F>,
    ) -> RecordOutcome {
        if (self.stop)() {
            return RecordOutcome::Cancelled;
        }
        match self.try_write(record, file, scratch) {
            Ok(outcome) => outcome,
            Err(error) => RecordOutcome::Failed(error),
        }
    }

    fn try_write<'f, F: RecordFileFormat>(
        &self,
        record: &Record<'f>,
        file: &str,
        scratch: &mut Scratch<'f, F>,
    ) -> Result<RecordOutcome, RecordExportError> {
        let relative = relative_path(file).ok_or(ExtractionError::UnsafePath)?;
        let name = relative.file_name().ok_or(ExtractionError::UnsafePath)?;
        let tree = scratch.trees.build(record)?;
        let errors = tree.error_count();
        let folder = create_folders(self.target, relative.parent())?;
        let destination = folder.join(name);
        scratch.bytes.clear();
        scratch
            .format
            .write_file(&tree, &mut scratch.bytes)
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

    fn keep<F>(
        &self,
        destination: &Path,
        scratch: &mut Scratch<'_, F>,
    ) -> Result<bool, ExtractionError> {
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

pub(crate) fn file_crc(path: &Path, buffer: &mut Vec<u8>) -> Result<u32, ExtractionError> {
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
