use p4kx_audio::SoundSource;

use super::{AudioIndex, IndexedSoundbank, LanguageChoice, SourceKind, WemLocation};

/// Where the WEM of one Sound object is, and in which language.
#[derive(Debug, Default)]
pub(crate) struct Located<'i> {
    pub(crate) location: Option<WemLocation<'i>>,
    pub(crate) localised: bool,
    pub(crate) language: Option<&'i str>,
    pub(crate) fallback: bool,
    pub(crate) missing_localisation: bool,
}

/// Locates the WEM of `source`, a Sound object of `soundbank`: embedded in a soundbank, or for a
/// streamed or prefetched source the loose WEM Entry named by its ID, in the language folder the
/// choice and the sound's localisation select.
pub(crate) fn locate<'i>(
    index: &'i AudioIndex,
    soundbank: &'i IndexedSoundbank,
    choice: &LanguageChoice,
    source: &SoundSource,
) -> Located<'i> {
    let kind = SourceKind::of(source);
    if matches!(kind, SourceKind::Plugin | SourceKind::Other(_)) {
        return Located::default();
    }
    let localised = source.is_language_specific() || soundbank.language().is_some();
    let id = source.source_id;
    let streamed = matches!(kind, SourceKind::Streamed | SourceKind::PrefetchStreamed);
    let find_loose = || loose(index, soundbank, choice, id, localised);
    let location = if streamed {
        find_loose().or_else(|| match kind {
            SourceKind::Streamed => None,
            _ => embedded(index, soundbank, id),
        })
    } else {
        embedded(index, soundbank, id).or_else(find_loose)
    };
    let missing_localisation =
        localised && streamed && !matches!(location, Some(WemLocation::Entry { .. }));
    let language = location.as_ref().and_then(WemLocation::language);
    Located {
        fallback: localised && choice.is_fallback(language),
        language: language.filter(|_| localised),
        location,
        localised,
        missing_localisation,
    }
}

const fn same(language: Option<&str>, wanted: Option<&str>) -> bool {
    match (language, wanted) {
        (None, None) => true,
        (Some(language), Some(wanted)) => language.eq_ignore_ascii_case(wanted),
        _ => false,
    }
}

fn loose<'i>(
    index: &'i AudioIndex,
    soundbank: &IndexedSoundbank,
    choice: &LanguageChoice,
    id: u32,
    localised: bool,
) -> Option<WemLocation<'i>> {
    let pick = |wanted: Option<&str>| {
        index
            .loose()
            .find(id)
            .find(|(language, _)| same(*language, wanted))
            .map(|(_, location)| location)
    };
    if !localised {
        return pick(None);
    }
    if let Some(language) = soundbank.language() {
        return pick(Some(language)).or_else(|| pick(None));
    }
    if choice.is_any() {
        return index
            .loose()
            .find(id)
            .find(|(language, _)| language.is_some())
            .map(|(_, location)| location)
            .or_else(|| pick(None));
    }
    choice
        .candidates()
        .find_map(|language| pick(Some(language)))
        .or_else(|| pick(None))
}

fn embedded<'i>(
    index: &'i AudioIndex,
    soundbank: &'i IndexedSoundbank,
    wem: u32,
) -> Option<WemLocation<'i>> {
    let holder = |location: &WemLocation<'i>| match location {
        WemLocation::Soundbank { soundbank, .. } => Some(*soundbank),
        _ => None,
    };
    let find = |accepts: &dyn Fn(&'i IndexedSoundbank) -> bool| {
        index
            .embedded_wem(wem)
            .find(|location| holder(location).is_some_and(accepts))
    };
    let ours = soundbank.language();
    find(&|bank| std::ptr::eq(bank, soundbank))
        .or_else(|| {
            find(&|bank| match (bank.language(), ours) {
                (Some(theirs), Some(ours)) => theirs.eq_ignore_ascii_case(ours),
                _ => false,
            })
        })
        .or_else(|| find(&|bank| bank.language().is_none()))
        .or_else(|| index.embedded_wem(wem).next())
}
