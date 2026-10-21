use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::DataForgeEntry;
use serde::Serialize;

use crate::cli::{DcbArgs, DcbCommand, DcbInfoArgs};
use crate::{dcb_export, dcb_fields, dcb_query, dcb_refs, dcb_tags, exit, json};

#[derive(Serialize)]
struct Summary<'a> {
    archive: String,
    entry: &'a str,
    version: u32,
    record_count: usize,
    struct_count: usize,
    enum_count: usize,
}

/// Runs a `p4kx dcb` subcommand.
pub(crate) fn run(args: &DcbArgs, as_json: bool) -> Result<ExitCode> {
    match &args.command {
        DcbCommand::Info(args) => info(args, as_json).map(|()| ExitCode::from(exit::SUCCESS)),
        DcbCommand::Export(args) => dcb_export::run(args, as_json),
        DcbCommand::Query(args) => dcb_query::run(args, as_json),
        DcbCommand::Refs(args) => dcb_refs::run(args, as_json),
        DcbCommand::Tags(args) => dcb_tags::run(args, as_json),
        DcbCommand::Fields(args) => dcb_fields::run(args, as_json),
    }
}

/// Prints the DataForge version and its number of Records, structs and enums.
fn info(args: &DcbInfoArgs, as_json: bool) -> Result<()> {
    let archive = crate::key::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let summary = Summary {
        archive: json::path(archive.path()),
        entry: entry.path(),
        version: forge.version(),
        record_count: forge.record_count(),
        struct_count: forge.struct_count(),
        enum_count: forge.enum_count(),
    };
    if as_json {
        json::print(&summary)?;
    } else {
        print_lines(&summary)?;
    }
    Ok(())
}

fn print_lines(summary: &Summary<'_>) -> io::Result<()> {
    let lines = [
        ("Archive", summary.archive.clone()),
        ("Entry", summary.entry.to_owned()),
        ("Version", summary.version.to_string()),
        ("Records", summary.record_count.to_string()),
        ("Structs", summary.struct_count.to_string()),
        ("Enums", summary.enum_count.to_string()),
    ];
    let mut out = io::stdout().lock();
    for (label, value) in lines {
        writeln!(out, "{:<16} {value}", format!("{label}:"))?;
    }
    Ok(())
}
