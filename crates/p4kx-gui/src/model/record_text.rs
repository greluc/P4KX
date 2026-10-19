use super::Model;
use crate::record_language::effective;
use crate::{RecordTextChoice, TextLanguage};

impl Model {
    /// Returns what the user chose for the texts next to localisation keys in Record previews,
    /// or `None` while the language of the interface decides.
    #[must_use]
    pub const fn record_text_choice(&self) -> Option<RecordTextChoice> {
        self.record_text_choice
    }

    /// Sets what Record previews show next to localisation keys; `None` follows the language of
    /// the interface. The open preview follows with the next frame without reading DataForge
    /// again.
    pub fn set_record_text_choice(&mut self, choice: Option<RecordTextChoice>) {
        self.record_text_choice = choice;
        self.apply_record_language();
    }

    /// Returns the languages the open Archive holds a `global.ini` for, or none while no
    /// Archive is open.
    #[must_use]
    pub fn record_text_languages(&self) -> &[TextLanguage] {
        self.browser
            .as_ref()
            .map_or(&[], |browser| browser.index().text_languages())
    }

    /// Returns the language Record previews resolve localisation keys in: the choice, or that of
    /// the interface when nothing was chosen, provided the open Archive holds its `global.ini`;
    /// `None` shows the keys alone.
    #[must_use]
    pub fn record_text_language(&self) -> Option<TextLanguage> {
        effective(
            self.record_text_choice,
            self.texts.language(),
            self.record_text_languages(),
        )
    }

    pub(crate) fn apply_record_language(&mut self) {
        let language = self.record_text_language();
        if let Some(browser) = &mut self.browser {
            browser.set_record_language(language);
        }
    }
}
