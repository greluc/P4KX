// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};

use p4kx_comparison::{
    Archive, ChangeKind, DiffBody, Encoding, LineDiff, LineKind, MAX_DIFF_LEN, MAX_DIFF_LINES,
    NotText, TextDiff,
};

use crate::json;

const MIB: u64 = 1024 * 1024;

/// Prints `diff` as a unified diff, as a note for an Entry that is not text or whose text did not
/// change, or as one line of JSON.
pub(crate) fn print(
    diff: &LineDiff<'_>,
    old: &Archive,
    new: &Archive,
    as_json: bool,
) -> io::Result<()> {
    if as_json {
        return json::print(diff);
    }
    let mut out = BufWriter::new(io::stdout().lock());
    match diff.body() {
        DiffBody::Text(text) => {
            if text.has_differences()
                || matches!(diff.kind(), Some(ChangeKind::Added | ChangeKind::Removed))
            {
                write_unified(&mut out, diff, text, old, new)?;
            } else if diff.kind().is_some() {
                writeln!(
                    out,
                    "{} changed, but its text is the same line by line",
                    diff.path()
                )?;
            }
        }
        DiffBody::NotText(reason) => {
            let what = match reason {
                NotText::Binary => "is binary".to_owned(),
                NotText::TooLarge => format!(
                    "is too large to diff (over {} MiB or {MAX_DIFF_LINES} lines)",
                    MAX_DIFF_LEN / MIB
                ),
            };
            let change = match diff.kind() {
                Some(ChangeKind::Added) => "was added",
                Some(ChangeKind::Removed) => "was removed",
                Some(ChangeKind::Changed) => "changed",
                None => "is unchanged",
            };
            writeln!(out, "{} {what}; it {change}", diff.path())?;
        }
    }
    out.flush()
}

fn write_unified(
    out: &mut impl Write,
    diff: &LineDiff<'_>,
    text: &TextDiff,
    old: &Archive,
    new: &Archive,
) -> io::Result<()> {
    write_header(
        out,
        "---",
        diff.old_entry().map(|_| diff.path()),
        old,
        diff.old_encoding(),
    )?;
    write_header(
        out,
        "+++",
        diff.new_entry().map(|_| diff.path()),
        new,
        diff.new_encoding(),
    )?;
    for hunk in text.hunks() {
        write!(out, "@@ -")?;
        write_range(out, hunk.old_start(), hunk.old_lines().len())?;
        write!(out, " +")?;
        write_range(out, hunk.new_start(), hunk.new_lines().len())?;
        writeln!(out, " @@")?;
        for line in hunk.lines() {
            let marker: &[u8] = match line.kind() {
                LineKind::Context => b" ",
                LineKind::Removed => b"-",
                LineKind::Added => b"+",
            };
            out.write_all(marker)?;
            out.write_all(line.bytes())?;
            if !line.bytes().ends_with(b"\n") {
                out.write_all(b"\n\\ No newline at end of file\n")?;
            }
        }
    }
    Ok(())
}

fn write_header(
    out: &mut impl Write,
    marker: &str,
    path: Option<&str>,
    archive: &Archive,
    encoding: Option<Encoding>,
) -> io::Result<()> {
    let archive = json::path(archive.path());
    match (path, encoding) {
        (None, _) => writeln!(out, "{marker} /dev/null\t{archive}"),
        (Some(path), Some(Encoding::CryXml(signature))) => {
            writeln!(
                out,
                "{marker} {path}\t{archive} ({signature} converted to XML)"
            )
        }
        (Some(path), Some(encoding @ (Encoding::Utf16Le | Encoding::Utf16Be))) => writeln!(
            out,
            "{marker} {path}\t{archive} ({} decoded)",
            encoding.as_str()
        ),
        (Some(path), _) => writeln!(out, "{marker} {path}\t{archive}"),
    }
}

fn write_range(out: &mut impl Write, start: usize, len: usize) -> io::Result<()> {
    if len == 1 {
        write!(out, "{start}")
    } else {
        write!(out, "{start},{len}")
    }
}
