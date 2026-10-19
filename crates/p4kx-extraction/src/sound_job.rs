use crate::AudioConversion;
use crate::texture_job::Item;

/// What an Entry is to audio Conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundRole {
    /// A WEM sound, `.wem`.
    Wem,
    /// A soundbank, `.bnk`.
    Bank,
}

/// Returns the role of the Entry at `path` with the path without its extension, or `None` when it
/// is no sound or names nothing before the extension.
pub(crate) fn sound_role(path: &str) -> Option<(SoundRole, &str)> {
    let (stem, extension) = path.rsplit_once('.')?;
    if stem.is_empty() || stem.ends_with(['\\', '/']) {
        return None;
    }
    let role = if extension.eq_ignore_ascii_case("wem") {
        SoundRole::Wem
    } else if extension.eq_ignore_ascii_case("bnk") {
        SoundRole::Bank
    } else {
        return None;
    };
    Some((role, stem))
}

/// Turns each Entry of `items` that is a WEM sound into a sound item unless `audio` keeps sounds
/// unchanged, and each soundbank into a bank item.
pub(crate) fn mark_sounds(items: &mut [Item<'_>], audio: AudioConversion) {
    for item in items {
        let Item::Entry(indexed) = *item else {
            continue;
        };
        match sound_role(indexed.1.path()) {
            Some((SoundRole::Wem, _)) if audio != AudioConversion::Raw => {
                *item = Item::Sound(indexed);
            }
            Some((SoundRole::Bank, _)) => *item = Item::Bank(indexed),
            _ => {}
        }
    }
}

impl Item<'_> {
    /// Returns whether the item is a sound or a soundbank, which run on the sound workers.
    pub(crate) const fn is_sound(&self) -> bool {
        matches!(self, Self::Sound(_) | Self::Bank(_))
    }

    /// Returns the uncompressed size of the item's first Entry, the bytes a sound worker holds.
    pub(crate) const fn first_size(&self) -> u64 {
        match self {
            Self::Entry((_, entry)) | Self::Sound((_, entry)) | Self::Bank((_, entry)) => {
                entry.uncompressed_size()
            }
            Self::Texture(job) => job.header.1.uncompressed_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundRole, sound_role};

    #[test]
    fn sounds_and_soundbanks_are_recognised_ignoring_case() {
        for (path, expected) in [
            ("Data\\a.wem", Some((SoundRole::Wem, "Data\\a"))),
            ("Data/A.WEM", Some((SoundRole::Wem, "Data/A"))),
            ("Data\\b.bnk", Some((SoundRole::Bank, "Data\\b"))),
            ("Data\\b.Bnk", Some((SoundRole::Bank, "Data\\b"))),
            ("b.bnk", Some((SoundRole::Bank, "b"))),
        ] {
            assert_eq!(sound_role(path), expected, "{path}");
        }
        for other in [
            "Data\\.wem",
            "Data/.bnk",
            ".wem",
            "Data\\a.wem.1",
            "Data\\a.wav",
            "Data\\wem",
            "Data.wem\\a",
        ] {
            assert_eq!(sound_role(other), None, "{other} is no sound");
        }
    }
}
