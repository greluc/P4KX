// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_assembly::character::{CHF_SIZE, Chf, ChfChange, diff_chf};
use serde::Serialize;

use crate::chf::{InvalidChf, read_capped};
use crate::chf_names;
use crate::cli::{ChfDiffArgs, ChfDiffFormat};
use crate::compare::finish;
use crate::json;

#[derive(Serialize)]
struct Report<'a> {
    a: String,
    b: String,
    identical: bool,
    changes: Vec<ChangeJson<'a>>,
}

#[derive(Serialize)]
struct ChangeJson<'a> {
    path: &'a str,
    raw_path: &'a str,
    old: Option<&'a str>,
    new: Option<&'a str>,
}

fn read(path: &Path) -> Result<Chf> {
    let bytes = read_capped(path, CHF_SIZE as u64)?;
    Chf::read(&bytes).map_err(|source| {
        InvalidChf {
            path: path.to_owned(),
            source,
        }
        .into()
    })
}

/// Runs `p4kx chf diff`.
pub(crate) fn run(args: &ChfDiffArgs, as_json: bool) -> Result<ExitCode> {
    let (old, new) = (read(&args.a)?, read(&args.b)?);
    let names = chf_names::names_for_all(args.archive.as_deref(), &[&old, &new])?;
    let changes = diff_chf(&old, &new, &names);
    if as_json || args.format == ChfDiffFormat::Json {
        let report = Report {
            a: json::path(&args.a),
            b: json::path(&args.b),
            identical: changes.is_empty(),
            changes: changes
                .iter()
                .map(|change| ChangeJson {
                    path: &change.path,
                    raw_path: &change.raw_path,
                    old: change.old.as_deref(),
                    new: change.new.as_deref(),
                })
                .collect(),
        };
        return finish(json::print(&report), !changes.is_empty());
    }
    finish(print_text(&changes), !changes.is_empty())
}

fn print_text(changes: &[ChfChange]) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    if changes.is_empty() {
        writeln!(out, "no differences")?;
    }
    for change in changes {
        match (&change.old, &change.new) {
            (Some(old), Some(new)) => writeln!(out, "{}: {old} -> {new}", change.path)?,
            (Some(old), None) => writeln!(out, "{}: removed ({old})", change.path)?,
            (None, Some(new)) => writeln!(out, "{}: added ({new})", change.path)?,
            (None, None) => {}
        }
    }
    out.flush()
}
