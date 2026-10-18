use std::io::Read;

use p4kx_archive::{Archive, Entry};

use super::{AudioFileError, MAX_AUDIO_FILE_LEN};

/// Reads the whole data of `entry`, refusing Entries above [`MAX_AUDIO_FILE_LEN`].
pub(crate) fn entry_bytes(archive: &Archive, entry: &Entry) -> Result<Vec<u8>, AudioFileError> {
    let size = entry.uncompressed_size();
    if size > MAX_AUDIO_FILE_LEN {
        return Err(AudioFileError::TooLarge { size });
    }
    let unreadable = |reason: String| AudioFileError::Unreadable { reason };
    let reader = archive
        .open_entry(entry)
        .map_err(|error| unreadable(error.to_string()))?;
    let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    reader
        .take(MAX_AUDIO_FILE_LEN)
        .read_to_end(&mut bytes)
        .map_err(|error| unreadable(error.to_string()))?;
    Ok(bytes)
}

/// Returns whether `path` starts with `prefix`, ignoring ASCII case and the separator kind.
pub(crate) fn starts_with_folder(path: &str, prefix: &str) -> bool {
    path.len() >= prefix.len()
        && path
            .bytes()
            .zip(prefix.bytes())
            .all(|(a, b)| fold(a) == fold(b))
}

/// Returns whether `path` ends with `.extension`, ignoring ASCII case.
pub(crate) fn has_extension(path: &str, extension: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, found)| found.eq_ignore_ascii_case(extension))
}

/// Orders two paths ignoring ASCII case, with `/` and `\` alike.
pub(crate) fn folded_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.bytes().map(fold).cmp(b.bytes().map(fold))
}

const fn fold(byte: u8) -> u8 {
    match byte {
        b'/' => b'\\',
        other => other.to_ascii_lowercase(),
    }
}
