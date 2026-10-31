// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{
    Exclusion, ExtractOptions, ExtractionReport, Filter, FolderSource, Progress,
    convert_folder_excluding,
};

use crate::cli::ConvertArgs;
use crate::dcb_export;
use crate::extract::{
    audio, cancel_on_interrupt, exit_code, policy, records_folder, texture_conversion,
};
use crate::extract_report::{print_report, report_json};
use crate::filter_pair::{self, ExcludedAll};
use crate::json;
use crate::no_match::NoMatch;
use crate::progress::ProgressLine;

/// Converts the files of the input folder into the output folder, showing progress and stopping
/// on Ctrl+C, and prints the report like `p4kx extract` does.
pub(crate) fn run(args: &ConvertArgs, as_json: bool) -> Result<ExitCode> {
    let filter = args.filter.as_deref().map(Filter::parse).transpose()?;
    let exclusion = Exclusion::parse(&args.exclude)?;
    crate::guard::ensure_writable(&args.out)?;
    let cancel = cancel_on_interrupt()?;
    if args.cancel_after == Some(0) {
        cancel.cancel();
    }
    let source = FolderSource::scan(&args.folder, Some(&cancel))?;
    let selection = (filter.as_ref(), &exclusion);
    check_selection(&source, selection, args.filter.as_deref())?;
    let line = ProgressLine::new();
    let on_progress = |progress: Progress| {
        if args
            .cancel_after
            .is_some_and(|count| progress.entries_done >= count)
        {
            cancel.cancel();
        }
        line.show(progress);
    };
    let options = ExtractOptions {
        policy: policy(args.conflict),
        convert: true,
        textures: texture_conversion(&args.textures, args.mip),
        audio: audio(args.audio),
        threads: args.threads,
        cancel: Some(&cancel),
        progress: Some(&on_progress),
        ..ExtractOptions::default()
    };
    let report =
        convert_folder_excluding(&source, selection, args.copy_other, &args.out, &options)?;
    line.finish();
    if let Err(error) = print(&report, args, as_json)
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(ExitCode::from(exit_code(&report)))
}

/// Refuses a Filter pair that selects no file and notes the excluded parts of textures that stay.
///
/// Without a Filter and without exclusion Filters every file is selected, even none.
///
/// # Errors
///
/// Returns [`NoMatch`] when the Filter `text` matches no file and [`ExcludedAll`] when the
/// exclusion Filters leave out all it matches; the message names `*` when there is no Filter.
fn check_selection(
    source: &FolderSource,
    selection: (Option<&Filter>, &Exclusion),
    text: Option<&str>,
) -> Result<()> {
    let (filter, exclusion) = selection;
    let counts = source.count_selected(filter, exclusion);
    if filter.is_some() || !exclusion.is_empty() {
        let text = text.unwrap_or("*");
        if counts.matched == 0 {
            return Err(NoMatch::new(text).into());
        }
        if counts.kept == 0 {
            return Err(ExcludedAll::new(text, counts.matched).into());
        }
    }
    filter_pair::note_excluded_parts(counts.parts_of_kept_textures, "the Conversion");
    Ok(())
}

/// Prints the report of the conversion as JSON or as text, with the Records it wrote.
fn print(report: &ExtractionReport, args: &ConvertArgs, as_json: bool) -> io::Result<()> {
    if as_json {
        return json::print(&report_json(report, &args.out, false));
    }
    print_report(report, &args.out, args.conflict, false)?;
    report.records().map_or(Ok(()), |records| {
        let entry = report.records_entry().unwrap_or_default();
        dcb_export::print_report(
            &records.into(),
            &records_folder(entry, &args.out),
            args.conflict,
            p4kx_export::RecordForm::Ndjson,
        )
    })
}
