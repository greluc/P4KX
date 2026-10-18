use crate::format_error::FormatError;
use crate::source::Source;
use crate::{Entry, central_directory, directory_v2, end_records};

const LAYOUT_MARKER_LEN: usize = 6;
const LAYOUT_MAGIC: [u8; 4] = *b"Jiji";
const LAYOUT_V2: u16 = 2;

/// Reads the Entries of the Archive in whichever layout it is written.
///
/// An Archive whose last bytes are a layout version followed by `Jiji` is read in that layout;
/// any other file is read as a v1 ZIP64 Archive.
pub(crate) fn read_entries(source: &Source) -> Result<Vec<Entry>, FormatError> {
    match layout_version(source)? {
        None => read_v1_entries(source),
        Some(LAYOUT_V2) => directory_v2::read_entries(source),
        Some(version) => Err(FormatError::UnsupportedLayout { version }),
    }
}

fn layout_version(source: &Source) -> Result<Option<u16>, FormatError> {
    let Some(offset) = source.len().checked_sub(LAYOUT_MARKER_LEN as u64) else {
        return Ok(None);
    };
    let marker = source.read_at(offset, LAYOUT_MARKER_LEN)?;
    match marker.split_first_chunk::<2>() {
        Some((version, magic)) if magic == LAYOUT_MAGIC => Ok(Some(u16::from_le_bytes(*version))),
        _ => Ok(None),
    }
}

fn read_v1_entries(source: &Source) -> Result<Vec<Entry>, FormatError> {
    let location = end_records::locate_directory(source)?;
    let size = usize::try_from(location.size).map_err(|_| {
        FormatError::corrupt(
            location.offset,
            "central directory does not fit into memory",
        )
    })?;
    let directory = source.read_at(location.offset, size)?;
    central_directory::parse_entries(&directory, location.offset, location.entry_count)
}
