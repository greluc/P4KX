use std::process::ExitCode;

use p4kx_extraction::{ArchiveError, UnknownEntries};

use crate::channels::InvalidPath;
use crate::extract::NoMatch;
use crate::texture;

/// Every requested Entry was extracted or already existed.
pub(crate) const SUCCESS: u8 = 0;
/// Some Entries failed or were cancelled, the target folder could not be created, or a texture
/// cannot be read, merged, decoded or written.
pub(crate) const PARTIAL_FAILURE: u8 = 1;
/// The command line is invalid, names an Entry the Archive does not contain, a file that does
/// not exist or a mip level the texture does not have, or has a Filter that matches no Entry.
pub(crate) const INVALID_INPUT: u8 = 2;
/// The Archive cannot be opened or read.
pub(crate) const ARCHIVE_ERROR: u8 = 3;
/// The Archive changed on disk after it was opened, so the Extraction stopped.
pub(crate) const ARCHIVE_CHANGED: u8 = 4;
/// A second Ctrl+C ended an Extraction without waiting for it to stop.
pub(crate) const INTERRUPTED: u8 = 130;

/// The exit-code section of the help text.
pub(crate) const EXIT_CODES_HELP: &str = "Exit codes:
  0  success: every requested Entry was extracted or already existed
  1  partial failure: some Entries failed or were cancelled, the target or the settings
     cannot be written, or a texture cannot be read, merged, decoded or written, e.g. because
     its format is not supported
  2  invalid input: bad arguments, an Entry path the Archive does not contain, a Filter that
     matches no Entry, a Channel path that does not exist or was never added, a texture file
     that does not exist, a mip level the texture does not have, or a merge or PNG output that
     is one of its inputs
  3  the Archive cannot be read: missing, not an Archive or corrupt
  4  the Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the
     Extraction stopped, and Entries written before may be stale
  130  interrupted: a second Ctrl+C ended an Extraction at once; temporary .part files may remain";

/// Returns the exit code for an error that ended a command.
pub(crate) fn for_error(error: &anyhow::Error) -> ExitCode {
    if error.downcast_ref::<ArchiveError>().is_some() || texture::is_archive_error(error) {
        ExitCode::from(ARCHIVE_ERROR)
    } else if error.downcast_ref::<UnknownEntries>().is_some()
        || error.downcast_ref::<InvalidPath>().is_some()
        || error.downcast_ref::<NoMatch>().is_some()
        || texture::is_invalid_input(error)
    {
        ExitCode::from(INVALID_INPUT)
    } else {
        ExitCode::from(PARTIAL_FAILURE)
    }
}
