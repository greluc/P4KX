use std::io;
use std::path::Path;

use p4kx_extraction::{WriteRefused, ensure_writable};

/// Returns why nothing may be written into `folder`, or `None` when it lies outside every Star
/// Citizen installation or none is chosen.
pub(crate) fn refusal_of(folder: Option<&Path>) -> Option<WriteRefused> {
    ensure_writable(folder?).err()
}

/// Refuses `target` as an I/O error when it lies inside a Star Citizen installation.
pub(crate) fn refuse(target: &Path) -> io::Result<()> {
    ensure_writable(target)
        .map_err(|refused| io::Error::new(io::ErrorKind::PermissionDenied, refused))
}
