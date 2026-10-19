use p4kx_audio::{HircKind, Soundbank};

use super::AudioFileError;

/// The folder below which the game keeps its soundbanks; a soundbank in a subfolder of it is
/// localised for the language the subfolder names.
const SOUNDBANK_FOLDER: &str = "wwise";

/// A soundbank Entry of the audio index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSoundbank {
    path: Box<str>,
    id: u32,
    language_id: u32,
    event_count: u32,
}

impl IndexedSoundbank {
    /// Returns the path of the soundbank Entry.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the file name of the soundbank Entry, e.g. `SSAM_ORIG_890J.bnk`.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.path.rsplit(['\\', '/']).next().unwrap_or_default()
    }

    /// Returns the language folder the soundbank lies in, e.g. `English(US)`, or `None` for a
    /// soundbank that is not localised.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        let mut folders = self.path.rsplit(['\\', '/']).skip(1);
        let folder = folders.next()?;
        let parent = folders.next()?;
        (parent.eq_ignore_ascii_case(SOUNDBANK_FOLDER) && !folder.is_empty()).then_some(folder)
    }

    /// Returns the soundbank ID its `BKHD` section states.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the language ID its `BKHD` section states.
    #[must_use]
    pub const fn language_id(&self) -> u32 {
        self.language_id
    }

    /// Returns how many Event objects its `HIRC` section holds.
    #[must_use]
    pub const fn event_count(&self) -> u32 {
        self.event_count
    }
}

/// Parses a soundbank and returns it with the IDs of its Event objects.
pub(crate) fn read(
    path: &str,
    bytes: &[u8],
) -> Result<(IndexedSoundbank, Vec<u32>), AudioFileError> {
    let bank = Soundbank::parse(bytes)?;
    let events: Vec<u32> = bank
        .objects()
        .iter()
        .filter(|object| object.kind == HircKind::Event)
        .map(|object| object.id)
        .collect();
    let header = bank.header();
    Ok((
        IndexedSoundbank {
            path: path.into(),
            id: header.id,
            language_id: header.language_id,
            event_count: u32::try_from(events.len()).unwrap_or(u32::MAX),
        },
        events,
    ))
}
