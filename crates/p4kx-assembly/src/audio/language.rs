/// The folder below which the game keeps its soundbanks and loose WEMs; a file in a language
/// subfolder of it is localised for that language.
pub(crate) const SOUND_FOLDER: &str = "Data\\Sounds\\wwise\\";

/// The folder below the sound folder, and below a language folder, that holds the loose WEMs.
const MEDIA_FOLDER: &str = "Media";

/// The language the audio of an Archive is read in: the one asked for and the one used when the
/// first has no localisation of a sound.
///
/// Language names are the folder names the Archive holds below `Data\Sounds\wwise\`, e.g.
/// `English(US)`, compared ignoring ASCII case. A choice without a name is *any*: no language is
/// preferred, every localised soundbank is followed and a loose localised WEM is taken from the
/// first language folder in path order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageChoice {
    preferred: Option<Box<str>>,
    fallback: Option<Box<str>>,
}

impl LanguageChoice {
    /// Returns the choice that prefers no language.
    #[must_use]
    pub const fn any() -> Self {
        Self {
            preferred: None,
            fallback: None,
        }
    }

    /// Returns the choice of the language `preferred`, falling back to `fallback` where a sound
    /// has no localisation in the first; either may be `None`.
    #[must_use]
    pub fn new(preferred: Option<&str>, fallback: Option<&str>) -> Self {
        Self {
            preferred: preferred.map(Into::into),
            fallback: fallback.map(Into::into),
        }
    }

    /// Returns the language asked for.
    #[must_use]
    pub fn preferred(&self) -> Option<&str> {
        self.preferred.as_deref()
    }

    /// Returns the language used where the preferred one has no localisation.
    #[must_use]
    pub fn fallback(&self) -> Option<&str> {
        self.fallback.as_deref()
    }

    /// Returns whether the choice names no language at all.
    #[must_use]
    pub const fn is_any(&self) -> bool {
        self.preferred.is_none() && self.fallback.is_none()
    }

    pub(crate) fn candidates(&self) -> impl Iterator<Item = &str> {
        self.preferred().into_iter().chain(self.fallback())
    }

    pub(crate) fn no_localisation(&self) -> super::BreakReason {
        super::BreakReason::NoLocalisation {
            language: self.preferred().unwrap_or_default().into(),
            fallback: self.fallback().unwrap_or_default().into(),
        }
    }

    pub(crate) fn is_fallback(&self, language: Option<&str>) -> bool {
        match (self.preferred(), language) {
            (Some(preferred), Some(language)) => !preferred.eq_ignore_ascii_case(language),
            _ => false,
        }
    }
}

/// Returns the language folder of a soundbank Entry: `Data\Sounds\wwise\<language>\x.bnk`.
pub(crate) fn bank_language(path: &str) -> Option<&str> {
    let mut folders = path.rsplit(['\\', '/']).skip(1);
    let folder = folders.next()?;
    let parent = folders.next()?;
    (parent.eq_ignore_ascii_case("wwise") && !folder.is_empty()).then_some(folder)
}

/// Where a loose WEM Entry lies and which sound it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LooseName<'p> {
    pub(crate) id: u32,
    pub(crate) language: Option<&'p str>,
}

/// Reads the name of a loose WEM Entry `Data\Sounds\wwise\[<language>\][Media\][<language>\]<id>.wem`,
/// or `None` for any other path.
pub(crate) fn loose_name(path: &str) -> Option<LooseName<'_>> {
    let rest = path.get(SOUND_FOLDER.len()..)?;
    let (folders, file) = rest.rsplit_once(['\\', '/']).unwrap_or(("", rest));
    let stem = file
        .len()
        .checked_sub(".wem".len())
        .and_then(|end| Some((file.get(..end)?, file.get(end..)?)))
        .filter(|(_, extension)| extension.eq_ignore_ascii_case(".wem"))
        .map(|(stem, _)| stem)?;
    if stem.is_empty() || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = stem.parse().ok()?;
    let mut language = None;
    if !folders.is_empty() {
        for folder in folders.split(['\\', '/']) {
            if folder.eq_ignore_ascii_case(MEDIA_FOLDER) {
                continue;
            }
            if folder.is_empty() || language.replace(folder).is_some() {
                return None;
            }
        }
    }
    Some(LooseName { id, language })
}
