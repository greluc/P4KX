use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};
use std::time::SystemTime;

use anyhow::{Context, Result};
use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, ArchiveWatch, AudioConversion, Build, CancelToken, ConflictPolicy, DataForgeEntry,
    Entry, ExtractOptions, ExtractionReport, Filter, Progress, TextureConversion, TextureGroups,
    create_default_target, extract, select_entries,
};
use thiserror::Error;

use crate::cli::{AudioOutput, Conflict, ExtractArgs, TextureOutput};
use crate::dcb_export;
use crate::exit;
use crate::extract_report::{print_report, report_json};
use crate::json;
use crate::progress::ProgressLine;

/// A Filter that matches no Entry of the Archive.
#[derive(Debug, Error)]
#[error("no Entry matches {0}")]
pub(crate) struct NoMatch(String);

impl NoMatch {
    /// Creates the error for the Filter `text`.
    pub(crate) fn new(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// Extracts the requested Entries, showing progress, stopping on Ctrl+C and quitting on a second
/// Ctrl+C and stopping when the Archive changes on disk, and prints the report: as JSON, or the
/// skipped and cancelled Entries and a summary to stdout and the failed Entries and a change of
/// the Archive to stderr.
pub(crate) fn run(args: &ExtractArgs, as_json: bool) -> Result<ExitCode> {
    let archive = crate::key::open(&args.archive)?;
    crate::issues::warn(&archive)?;
    let entries = requested_entries(args, &archive)?;
    let settings = Settings {
        out: args.out.as_deref(),
        conflict: args.conflict,
        convert: args.convert,
        textures: textures(args),
        audio: audio(args.audio),
        verify: args.verify,
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
    /// Whether each Entry is checked against its checksum while it is written.
    pub(crate) verify: bool,
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
    crate::key::require(archive, entries)?;
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
        verify: settings.verify,
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        watch: Some(&watch),
    };
    let report = extract(archive, entries, &target, &options);
    line.finish();
    let printed = if as_json {
        json::print(&report_json(&report, &target, settings.verify))
    } else {
        print_report(&report, &target, settings.conflict, settings.verify).and_then(|()| {
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

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use p4kx_extraction::{ArchiveWatch, ExtractOptions, extract};
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
        let archive = crate::key::open(&path)?;
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
