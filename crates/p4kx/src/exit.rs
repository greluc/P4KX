use std::process::ExitCode;

use p4kx_assembly::{EntityLookupError, LoadoutError};
use p4kx_comparison::LineDiffError;
use p4kx_extraction::cryxml::CryXmlError;
use p4kx_extraction::{ArchiveError, DataForgeError, DataForgeLoadError, UnknownEntries};

use crate::catalog::{InvalidExportTime, MissingCatalogArgs};
use crate::channels::InvalidPath;
use crate::chf::{InvalidChf, InvalidChfJson, UnreadableChfInput};
use crate::cryxml::{JsonNeedsOut, UnreadableInput};
use crate::dcb_export::NoRecordMatch;
use crate::dcb_query::{InvalidQuery, JsonNeedsJsonFormat, OutputIsArchive, UnreadableSchema};
use crate::extract::NoMatch;
use crate::texture;

/// Every requested Entry was extracted or already existed; a Comparison found no difference.
pub(crate) const SUCCESS: u8 = 0;
/// Some Entries or Records failed or were cancelled, the target folder could not be created, a
/// texture is invalid or cannot be merged, decoded or written, or a sound or soundbank is invalid or
/// cannot be converted or written.
pub(crate) const PARTIAL_FAILURE: u8 = 1;
/// A Comparison found differences.
pub(crate) const DIFFERENCES: u8 = 1;
/// The command line is invalid, names an Entry the Archive does not contain (for a line diff:
/// neither Archive contains), a file that does not exist or a mip level the texture does not have,
/// has a Filter that matches no Entry or Record, names an output that is one of its inputs, has
/// an invalid Query or a Query output that does not fit --json, or gives CHF JSON that is invalid
/// or cannot be written as CHF.
pub(crate) const INVALID_INPUT: u8 = 2;
/// The Archive, an Entry's data or the input file cannot be opened or read, or is not a valid CHF
/// file.
pub(crate) const ARCHIVE_ERROR: u8 = 3;
/// The Archive changed on disk after it was opened, so the Extraction stopped.
pub(crate) const ARCHIVE_CHANGED: u8 = 4;
/// The Archive holds no DataForge, or its DataForge has an unknown version or is corrupt.
pub(crate) const DATAFORGE_ERROR: u8 = 5;
/// A Comparison ran, but its report or its output cannot be written.
pub(crate) const COMPARISON_FAILED: u8 = 6;
/// A second Ctrl+C ended an Extraction without waiting for it to stop.
pub(crate) const INTERRUPTED: u8 = 130;

/// The exit-code section of the help text.
pub(crate) const EXIT_CODES_HELP: &str = "Exit codes:
  0  success: every requested Entry was extracted or already existed; compare: no differences,
     also for the --diff Entry
  1  partial failure: some Entries or Records failed or were cancelled, the target or the
     settings cannot be written, a texture is invalid or cannot be merged, decoded or written,
     e.g. because its format is not supported, or a sound or soundbank is invalid or cannot be
     converted or written, e.g. a corrupt WEM or BNK or a codec without a decoder; compare:
     the Archives or the --diff Entry differ
  2  invalid input: bad arguments, an Entry path the Archive does not contain (for --diff:
     neither Archive contains), a Filter that matches no Entry or Record, a Channel path that
     does not exist or was never added, a texture, sound or soundbank file that does not
     exist, a mip level the texture does not have, an output that is one of its inputs,
     --json without --out for cryxml or chf to-json, an invalid Query, --json with another --format and
     without --out for dcb query, an Entity that no Record or several Records match, or a
     CHF JSON file that is invalid, has an unknown version or holds values a CHF file cannot
     store
  3  the Archive or the input file cannot be read: missing, not an Archive, not CryXml or CHF,
     corrupt, with a CRC-32C mismatch, an unknown CHF version or unreadable Entry data, also a
     --diff Entry that cannot be read or holds corrupt CryXml; a missing texture, sound or
     soundbank file is 2
  4  the Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the
     Extraction stopped, and Entries written before may be stale
  5  the DataForge cannot be read: the Archive holds none, its version is not supported or it
     is corrupt, or the Record of the Entity whose Loadout is shown cannot be read
  6  compare: the report or the output cannot be written
  130  interrupted: a second Ctrl+C ended an Extraction at once; temporary .part files may remain";

/// Returns the exit code for an error that ended a command.
pub(crate) fn for_error(error: &anyhow::Error) -> ExitCode {
    ExitCode::from(code_for(error))
}

/// Returns the exit code for an error that ended `p4kx compare`, whose code 1 means differences.
pub(crate) fn for_compare_error(error: &anyhow::Error) -> ExitCode {
    match code_for(error) {
        PARTIAL_FAILURE => ExitCode::from(COMPARISON_FAILED),
        code => ExitCode::from(code),
    }
}

fn code_for(error: &anyhow::Error) -> u8 {
    if let Some(error) = error.downcast_ref::<LineDiffError>() {
        return match error {
            LineDiffError::UnknownEntry { .. } => INVALID_INPUT,
            _ => ARCHIVE_ERROR,
        };
    }
    let dataforge = error.downcast_ref::<DataForgeLoadError>();
    if error.downcast_ref::<ArchiveError>().is_some()
        || texture::is_archive_error(error)
        || error.downcast_ref::<CryXmlError>().is_some()
        || error.downcast_ref::<UnreadableInput>().is_some()
        || error.downcast_ref::<UnreadableChfInput>().is_some()
        || error.downcast_ref::<InvalidChf>().is_some()
        || matches!(
            dataforge,
            Some(DataForgeLoadError::Archive(_) | DataForgeLoadError::Read { .. })
        )
    {
        ARCHIVE_ERROR
    } else if dataforge.is_some()
        || error.downcast_ref::<DataForgeError>().is_some()
        || error.downcast_ref::<UnreadableSchema>().is_some()
        || error.downcast_ref::<LoadoutError>().is_some()
    {
        DATAFORGE_ERROR
    } else if error.downcast_ref::<UnknownEntries>().is_some()
        || error.downcast_ref::<InvalidPath>().is_some()
        || error.downcast_ref::<NoMatch>().is_some()
        || error.downcast_ref::<NoRecordMatch>().is_some()
        || texture::is_invalid_input(error)
        || error.downcast_ref::<JsonNeedsOut>().is_some()
        || error.downcast_ref::<InvalidQuery>().is_some()
        || error.downcast_ref::<JsonNeedsJsonFormat>().is_some()
        || error.downcast_ref::<OutputIsArchive>().is_some()
        || error.downcast_ref::<EntityLookupError>().is_some()
        || error.downcast_ref::<InvalidChfJson>().is_some()
        || error.downcast_ref::<InvalidExportTime>().is_some()
        || error.downcast_ref::<MissingCatalogArgs>().is_some()
    {
        INVALID_INPUT
    } else {
        PARTIAL_FAILURE
    }
}
