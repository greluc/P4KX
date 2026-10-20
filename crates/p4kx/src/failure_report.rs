use std::error::Error;
use std::io::{self, Write};

use p4kx_extraction::EntryFailure;
use serde::Serialize;

/// The JSON shape of an Entry that failed.
#[derive(Serialize)]
pub(crate) struct FailureJson<'a> {
    pub(crate) path: &'a str,
    pub(crate) error: String,
}

impl<'a> FailureJson<'a> {
    /// Returns the JSON of `failure`.
    pub(crate) fn of(failure: &'a EntryFailure) -> Self {
        Self {
            path: failure.path(),
            error: reason(failure),
        }
    }
}

/// The JSON shape of an Entry whose data does not match its checksum.
#[derive(Serialize)]
pub(crate) struct MismatchJson<'a> {
    path: &'a str,
    kind: &'static str,
    expected: String,
    actual: String,
}

impl<'a> MismatchJson<'a> {
    /// Returns the JSON of `failure` when it is a checksum mismatch.
    pub(crate) fn of(failure: &'a EntryFailure) -> Option<Self> {
        let (expected, actual) = failure.checksum_mismatch()?;
        Some(Self {
            path: failure.path(),
            kind: expected.name(),
            expected: expected.hex(),
            actual: actual.hex(),
        })
    }
}

/// Returns the error of `failure` followed by its causes, separated by colons.
pub(crate) fn reason(failure: &EntryFailure) -> String {
    let mut reason = failure.error().to_string();
    let mut source = failure.error().source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}

/// Returns the failures that are no checksum mismatch.
pub(crate) fn unreadable(failures: &[EntryFailure]) -> impl Iterator<Item = &EntryFailure> {
    failures
        .iter()
        .filter(|failure| failure.checksum_mismatch().is_none())
}

/// Writes the line of a checksum mismatch to `out`.
pub(crate) fn write_mismatch(out: &mut impl Write, failure: &EntryFailure) -> io::Result<()> {
    match failure.checksum_mismatch() {
        Some((expected, actual)) => writeln!(
            out,
            "mismatch {}: {actual} computed, {expected} stored",
            failure.path()
        ),
        None => Ok(()),
    }
}
