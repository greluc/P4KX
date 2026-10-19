use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::SystemTime;

use anyhow::{Context, Result};
use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, ArchiveWatch, AudioConversion, Build, CancelToken, ConflictPolicy, DataForgeEntry,
    Entry, EntryFailure, ExtractOptions, ExtractionReport, Filter, Progress, TextureConversion,
    TextureGroups,
    create_default_target, extract, select_entries,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{AudioOutput, Conflict, ExtractArgs, TextureOutput};
use crate::dcb_export;
use crate::exit;
use crate::json;
use crate::progress::ProgressLine;

/// A Filter that matches no Entry of the Archive.
#[derive(Debug, Error)]
#[error("no Entry matches {0}")]
pub(crate) struct NoMatch(String);

#[derive(Serialize)]
struct ReportJson<'a> {
    target: String,
    extracted: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    converted: Option<usize>,
    skipped: &'a [String],
    failed: Vec<FailureJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unconverted: Option<Vec<FailureJson<'a>>>,
    cancelled: &'a [String],
    archive_changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    records: Option<RecordsJson<'a>>,
}

#[derive(Serialize)]
struct RecordsJson<'a> {
    exported: usize,
    skipped: &'a [String],
    failed: Vec<dcb_export::FailureJson<'a>>,
    cancelled: usize,
    error_values: usize,
    records_with_errors: usize,
}

#[derive(Serialize)]
struct FailureJson<'a> {
    path: &'a str,
    error: String,
}

/// Extracts the requested Entries, showing progress, stopping on Ctrl+C and quitting on a second
/// Ctrl+C and stopping when the Archive changes on disk, and prints the report: as JSON, or the
/// skipped and cancelled Entries and a summary to stdout and the failed Entries and a change of
/// the Archive to stderr.
pub(crate) fn run(args: &ExtractArgs, as_json: bool) -> Result<ExitCode> {
    let archive = Archive::open(&args.archive)?;
    let entries = requested_entries(args, &archive)?;
    let settings = Settings {
        out: args.out.as_deref(),
        conflict: args.conflict,
        convert: args.convert,
        textures: textures(args),
        audio: audio(args.audio),
        cancel_after: args.cancel_after,
    };
    execute(&archive, &entries, &settings, as_json)
}

/// How an Extraction writes its Entries.
pub(crate) struct Settings<'a> {
    /// The target folder, or `None` for a new default target folder.
    pub(crate) out: Option<&'a Path>,
    /// What to do with a file that already exists.
    pub(crate) conflict: Conflict,
    /// Whether CryXml, textures, sounds and the DataForge are converted.
    pub(crate) convert: bool,
    /// What a texture conversion writes.
    pub(crate) textures: TextureConversion,
    /// What a sound conversion writes.
    pub(crate) audio: AudioConversion,
    /// Cancels the run once this many Entries have ended.
    pub(crate) cancel_after: Option<usize>,
}

/// Extracts `entries` of `archive`, which may be a container, as [`run`] does.
pub(crate) fn execute(
    archive: &Archive,
    entries: &[&Entry],
    settings: &Settings<'_>,
    as_json: bool,
) -> Result<ExitCode> {
    let watch = ArchiveWatch::new(archive);
    let target = prepare_target(settings.out, archive)?;
    let cancel = cancel_on_interrupt()?;

    let line = ProgressLine::new();
    let on_progress = |progress: Progress| {
        if settings
            .cancel_after
            .is_some_and(|count| progress.entries_done >= count)
        {
            cancel.cancel();
        }
        line.show(progress);
    };
    let options = ExtractOptions {
        policy: policy(settings.conflict),
        convert: settings.convert,
        textures: settings.textures,
        audio: settings.audio,
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        watch: Some(&watch),
    };
    let report = extract(archive, entries, &target, &options);
    line.finish();
    let printed = if as_json {
        json::print(&report_json(&report, &target))
    } else {
        print_report(&report, &target, settings.conflict).and_then(|()| {
            report.records().map_or(Ok(()), |records| {
                let folder = records_folder(archive, &target);
                dcb_export::print_report(&records.into(), &folder, settings.conflict)
            })
        })
    };
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    if report.archive_changed()
        && let Err(changed) = watch.check()
    {
        let _ignored = writeln!(io::stderr().lock(), "error: {changed}");
    }
    Ok(ExitCode::from(exit_code(&report)))
}

/// Returns a token that the first Ctrl+C cancels; a second Ctrl+C ends the process at once.
pub(crate) fn cancel_on_interrupt() -> Result<CancelToken> {
    let cancel = CancelToken::new();
    let on_interrupt = cancel.clone();
    ctrlc::set_handler(move || {
        if on_interrupt.is_cancelled() {
            process::exit(exit::INTERRUPTED.into());
        }
        on_interrupt.cancel();
    })
    .context("cannot install the Ctrl+C handler")?;
    Ok(cancel)
}

const fn exit_code(report: &ExtractionReport) -> u8 {
    if report.archive_changed() {
        exit::ARCHIVE_CHANGED
    } else if report.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }
}

fn textures(args: &ExtractArgs) -> TextureConversion {
    TextureConversion {
        merge: args.textures.contains(&TextureOutput::Merge),
        png: args
            .textures
            .contains(&TextureOutput::Png)
            .then_some(args.mip),
        keep_raw: args.textures.contains(&TextureOutput::Raw),
    }
}

const fn audio(output: AudioOutput) -> AudioConversion {
    match output {
        AudioOutput::Ogg => AudioConversion::Ogg,
        AudioOutput::Wav => AudioConversion::Wav,
        AudioOutput::Raw => AudioConversion::Raw,
    }
}

pub(crate) const fn policy(conflict: Conflict) -> ConflictPolicy {
    match conflict {
        Conflict::Skip => ConflictPolicy::Skip,
        Conflict::Overwrite => ConflictPolicy::Overwrite,
        Conflict::IfChanged => ConflictPolicy::IfChanged,
    }
}

fn requested_entries<'a>(args: &ExtractArgs, archive: &'a Archive) -> Result<Vec<&'a Entry>> {
    match &args.filter {
        Some(text) => filtered_entries(archive, text),
        None => Ok(select_entries(archive, &args.entries)?),
    }
}

/// Returns the Entries of `archive` the Filter `text` matches, and all Entries of each split
/// texture one of them matches.
///
/// # Errors
///
/// Returns [`NoMatch`] when it matches none.
pub(crate) fn filtered_entries<'a>(archive: &'a Archive, text: &str) -> Result<Vec<&'a Entry>> {
    let groups = TextureGroups::build(archive);
    let entries = Filter::new(text).select_with_parts(archive, &groups);
    if entries.is_empty() {
        return Err(NoMatch(text.to_owned()).into());
    }
    Ok(entries)
}

/// Creates the target folder `out`, or a new default target folder in the current directory.
pub(crate) fn prepare_target(out: Option<&Path>, archive: &Archive) -> Result<PathBuf> {
    if let Some(out) = out {
        fs::create_dir_all(out)
            .with_context(|| format!("cannot create the target folder {}", out.display()))?;
        return Ok(out.to_path_buf());
    }
    let parent = std::env::current_dir().context("cannot read the current directory")?;
    let channel = channel_name(archive.path());
    let version = archive.build().map(Build::version);
    create_default_target(&parent, channel.as_deref(), version, SystemTime::now())
        .with_context(|| format!("cannot create a target folder in {}", parent.display()))
}

fn report_json<'a>(report: &'a ExtractionReport, target: &Path) -> ReportJson<'a> {
    ReportJson {
        target: json::path(target),
        extracted: report.extracted(),
        converted: report.converted(),
        skipped: report.skipped(),
        failed: failures_json(report.failures()),
        unconverted: report
            .converted()
            .map(|_| failures_json(report.unconverted())),
        cancelled: report.cancelled(),
        archive_changed: report.archive_changed(),
        records: report.records().map(|records| RecordsJson {
            exported: records.written(),
            skipped: records.skipped(),
            failed: records
                .failures()
                .iter()
                .map(dcb_export::failure_json)
                .collect(),
            cancelled: records.cancelled(),
            error_values: records.error_values(),
            records_with_errors: records.records_with_errors(),
        }),
    }
}

/// Returns the folder the Records of the converted DataForge Entry were written into: the folder
/// of that Entry below `target`.
fn records_folder(archive: &Archive, target: &Path) -> PathBuf {
    let entry = DataForgeEntry::locate(archive).map_or("", Entry::path);
    entry
        .rsplit_once(['\\', '/'])
        .map_or_else(String::new, |(folder, _)| folder.replace('\\', "/"))
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(target.to_path_buf(), |folder, part| folder.join(part))
}

fn failures_json(failures: &[EntryFailure]) -> Vec<FailureJson<'_>> {
    failures
        .iter()
        .map(|failure| FailureJson {
            path: failure.path(),
            error: reason(failure),
        })
        .collect()
}

fn reason(failure: &EntryFailure) -> String {
    let mut reason = failure.error().to_string();
    let mut source = failure.error().source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}

fn print_report(report: &ExtractionReport, target: &Path, conflict: Conflict) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in report.failures() {
        writeln!(err, "failed {}: {}", failure.path(), reason(failure))?;
    }
    for unconverted in report.unconverted() {
        writeln!(
            err,
            "not converted {}: {}",
            unconverted.path(),
            reason(unconverted)
        )?;
    }
    let mut out = io::stdout().lock();
    let kept = if conflict == Conflict::IfChanged {
        "unchanged"
    } else {
        "already exists"
    };
    for path in report.skipped() {
        writeln!(out, "skipped {path}: {kept}")?;
    }
    for path in report.cancelled() {
        writeln!(out, "cancelled {path}")?;
    }
    let converted = match (report.converted(), report.unconverted().len()) {
        (None, _) => String::new(),
        (Some(count), 0) => format!(" ({count} converted)"),
        (Some(count), unconverted) => format!(" ({count} converted, {unconverted} not converted)"),
    };
    writeln!(
        out,
        "extracted {}{converted}, skipped {}, failed {}, cancelled {} into {}",
        report.extracted(),
        report.skipped().len(),
        report.failures().len(),
        report.cancelled().len(),
        target.display()
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use p4kx_extraction::{Archive, ArchiveWatch, ExtractOptions, extract};
    use p4kx_testkit::{ArchiveBuilder, TestEntry};

    use super::exit_code;
    use crate::exit;

    fn builder() -> ArchiveBuilder {
        ArchiveBuilder::new().entry(TestEntry::stored("Data\\readme.txt", "hello"))
    }

    fn extract_after(change: bool) -> Result<u8, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        builder().write_to(&path)?;
        let archive = Archive::open(&path)?;
        let watch = ArchiveWatch::new(&archive);
        if change {
            builder()
                .entry(TestEntry::stored("Data\\patch.txt", "new"))
                .write_to(&path)?;
        }
        let target = dir.path().join("out");
        fs::create_dir(&target)?;
        let entries: Vec<_> = archive.entries().iter().collect();
        let options = ExtractOptions {
            watch: Some(&watch),
            ..ExtractOptions::default()
        };
        Ok(exit_code(&extract(&archive, &entries, &target, &options)))
    }

    #[test]
    fn an_unchanged_archive_ends_with_success() -> Result<(), Box<dyn Error>> {
        assert_eq!(extract_after(false)?, exit::SUCCESS, "nothing changed");
        Ok(())
    }

    #[test]
    fn a_changed_archive_ends_with_its_own_exit_code() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            extract_after(true)?,
            exit::ARCHIVE_CHANGED,
            "the Archive was rewritten after it was opened"
        );
        Ok(())
    }
}
