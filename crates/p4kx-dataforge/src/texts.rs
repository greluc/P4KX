use std::borrow::Cow;

/// How a [`TextResolver`] answers for one localisation key.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resolution<'a> {
    /// The key has this text.
    Text(Cow<'a, str>),
    /// The key has no text in the chosen language; the output marks it.
    Missing,
    /// The key stands for no text, e.g. a placeholder key; the output keeps the key alone.
    Keep,
}

/// Resolves the localisation keys of Record fields into the texts of one chosen language, for the
/// writers of Record XML and Record JSON.
pub trait TextResolver: Sync {
    /// Resolves `key` as a Record field stores it, with its leading `@`.
    fn resolve<'a>(&'a self, key: &str) -> Resolution<'a>;
}

/// The texts of the writers: none for default output.
pub(crate) type Texts<'t> = Option<&'t dyn TextResolver>;

/// Returns the annotation of the localisation key `key`; an empty key is kept alone.
pub(crate) fn annotation<'t>(texts: Texts<'t>, key: &str) -> Option<Resolution<'t>> {
    if key.is_empty() {
        return None;
    }
    match texts?.resolve(key) {
        Resolution::Keep => None,
        resolution => Some(resolution),
    }
}
