use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use p4kx_assembly::character::{CHF_SIZE, Chf, ChfError, Versions};
use serde::Serialize;
use thiserror::Error;

use crate::cli::{ChfAction, ChfArgs};
use crate::cryxml::JsonNeedsOut;
use crate::json;
use crate::query_file::write_replacing;

/// The largest JSON file `p4kx chf from-json` reads.
const MAX_CHF_JSON_LEN: u64 = 1024 * 1024;

/// The input file of `p4kx chf` cannot be read or is larger than any valid input.
#[derive(Debug, Error)]
pub(crate) enum UnreadableChfInput {
    /// Opening or reading failed.
    #[error("cannot read {}", path.display())]
    Io {
        /// The input file.
        path: PathBuf,
        /// What failed.
        #[source]
        source: io::Error,
    },
    /// The file is larger than any input of its kind.
    #[error("cannot read {}: it is larger than {limit} bytes", path.display())]
    TooLarge {
        /// The input file.
        path: PathBuf,
        /// The largest accepted input.
        limit: u64,
    },
}

/// The input of `p4kx chf to-json` is not a valid CHF file.
#[derive(Debug, Error)]
#[error("cannot read {} as CHF", path.display())]
pub(crate) struct InvalidChf {
    path: PathBuf,
    #[source]
    source: ChfError,
}

/// The input of `p4kx chf from-json` is not valid CHF JSON or cannot be written as CHF.
#[derive(Debug, Error)]
#[error("cannot convert {} to CHF", path.display())]
pub(crate) struct InvalidChfJson {
    path: PathBuf,
    #[source]
    source: ChfError,
}

#[derive(Serialize)]
struct Summary {
    input: String,
    output: String,
    versions: Versions,
    size: u64,
}

/// Runs `p4kx chf to-json` or `p4kx chf from-json`.
pub(crate) fn run(args: &ChfArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ChfAction::ToJson { file, out } => to_json(file, out.as_deref(), as_json),
        ChfAction::FromJson { file, out } => from_json(file, out, as_json),
    }
}

fn to_json(file: &Path, out: Option<&Path>, as_json: bool) -> Result<()> {
    if as_json && out.is_none() {
        return Err(JsonNeedsOut.into());
    }
    let bytes = read_capped(file, CHF_SIZE as u64)?;
    let chf = Chf::read(&bytes).map_err(|source| InvalidChf {
        path: file.to_owned(),
        source,
    })?;
    let mut text = chf.to_json().map_err(|source| InvalidChf {
        path: file.to_owned(),
        source,
    })?;
    text.push('\n');
    let Some(out) = out else {
        return Ok(io::stdout().lock().write_all(text.as_bytes())?);
    };
    write_replacing(out, |writer| writer.write_all(text.as_bytes()))
        .with_context(|| format!("cannot write {}", out.display()))?;
    report(
        file,
        out,
        chf.character.versions,
        text.len() as u64,
        as_json,
    )
}

fn from_json(file: &Path, out: &Path, as_json: bool) -> Result<()> {
    let text = read_capped(file, MAX_CHF_JSON_LEN)?;
    let invalid = |source| InvalidChfJson {
        path: file.to_owned(),
        source,
    };
    let chf = Chf::from_json(&text).map_err(invalid)?;
    let bytes = chf.write().map_err(invalid)?;
    write_replacing(out, |writer| writer.write_all(&bytes))
        .with_context(|| format!("cannot write {}", out.display()))?;
    report(
        file,
        out,
        chf.character.versions,
        bytes.len() as u64,
        as_json,
    )
}

fn report(input: &Path, output: &Path, versions: Versions, size: u64, as_json: bool) -> Result<()> {
    let summary = Summary {
        input: json::path(input),
        output: json::path(output),
        versions,
        size,
    };
    if as_json {
        json::print(&summary)?;
    } else {
        writeln!(
            io::stdout().lock(),
            "converted {} (versions {}/{}) to {}, {} bytes",
            summary.input,
            versions.female,
            versions.male,
            summary.output,
            size
        )?;
    }
    Ok(())
}

/// Reads a whole file of at most `limit` bytes.
fn read_capped(path: &Path, limit: u64) -> Result<Vec<u8>, UnreadableChfInput> {
    let io_error = |source| UnreadableChfInput::Io {
        path: path.to_owned(),
        source,
    };
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
        .map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err(UnreadableChfInput::TooLarge {
            path: path.to_owned(),
            limit,
        });
    }
    Ok(bytes)
}
