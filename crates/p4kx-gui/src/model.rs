use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::{Archive, ArchiveError};
use p4kx_settings::{ExtractionTextures, RecentArchives};

mod archive_change;
mod audio_control;
mod catalog_control;
mod extraction_control;
mod record_export_control;
mod record_text;
mod update;
mod work_tab;

use catalog_control::CatalogState;
use record_export_control::RecordExportState;
use update::UpdateState;
pub use work_tab::WorkTab;

use crate::{
    AudioOutput, Browser, ChannelList, ComparisonTab, DeviceOutput, EntryIndex, Extraction,
    ExtractionSetup, Language, LocalizationTab, PreviewTab, QueryTab, RecordTextChoice, Texts,
};

/// Why an Archive could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The Archive that was opened.
    pub path: PathBuf,
    /// The error and its causes, joined by colons, or `None` when the opening stopped without
    /// a result.
    pub reason: Option<String>,
}

struct Opening {
    path: PathBuf,
    result: Receiver<Result<EntryIndex, ArchiveError>>,
}

/// The whole state of the window, independent of how it is drawn.
pub struct Model {
    texts: Texts,
    channels: ChannelList,
    recent: RecentArchives,
    browser: Option<Browser>,
    opening: Option<Opening>,
    failure: Option<Failure>,
    licence_shown: bool,
    preview_tab: PreviewTab,
    extraction_folder: Option<PathBuf>,
    extraction_conversion: bool,
    extraction_textures: ExtractionTextures,
    extraction_setup: Option<ExtractionSetup>,
    extraction: Option<Extraction>,
    update: UpdateState,
    work_tab: WorkTab,
    comparison: ComparisonTab,
    stopped_by_change: bool,
    audio_output: Box<dyn AudioOutput>,
    audio_volume: f32,
    query: QueryTab,
    localization: LocalizationTab,
    record_export: RecordExportState,
    catalog: CatalogState,
    record_text_choice: Option<RecordTextChoice>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("texts", &self.texts)
            .field("channels", &self.channels)
            .field("recent", &self.recent)
            .field("browser", &self.browser)
            .field("opening", &self.opening_path())
            .field("failure", &self.failure)
            .field("licence_shown", &self.licence_shown)
            .field("preview_tab", &self.preview_tab)
            .field("extraction_folder", &self.extraction_folder)
            .field("extraction_conversion", &self.extraction_conversion)
            .field("extraction_textures", &self.extraction_textures)
            .field("extraction_setup", &self.extraction_setup)
            .field("extraction", &self.extraction)
            .field("update", &self.update)
            .field("work_tab", &self.work_tab)
            .field("comparison", &self.comparison)
            .field("stopped_by_change", &self.stopped_by_change)
            .field("query", &self.query)
            .field("localization", &self.localization)
            .field("record_export", &self.record_export)
            .field("catalog", &self.catalog)
            .field("record_text_choice", &self.record_text_choice)
            .field("audio_volume", &self.audio_volume)
            .finish_non_exhaustive()
    }
}

impl Model {
    /// Creates the state of a window without an Archive, in `language`.
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self {
            texts: Texts::new(language),
            channels: ChannelList::default(),
            recent: RecentArchives::default(),
            browser: None,
            opening: None,
            failure: None,
            licence_shown: false,
            preview_tab: PreviewTab::Text,
            extraction_folder: None,
            extraction_conversion: false,
            extraction_textures: ExtractionTextures::default(),
            extraction_setup: None,
            extraction: None,
            update: UpdateState::default(),
            work_tab: WorkTab::default(),
            comparison: ComparisonTab::default(),
            stopped_by_change: false,
            audio_output: Box::new(DeviceOutput),
            audio_volume: 1.0,
            query: QueryTab::default(),
            localization: LocalizationTab::default(),
            record_export: RecordExportState::default(),
            catalog: CatalogState::default(),
            record_text_choice: None,
        }
    }

    /// Returns the interface texts in the current language.
    #[must_use]
    pub const fn texts(&self) -> &Texts {
        &self.texts
    }

    /// Switches the interface language.
    pub fn set_language(&mut self, language: Language) {
        self.texts.set_language(language);
        self.apply_record_language();
    }

    /// Returns the Channels the picker offers.
    #[must_use]
    pub const fn channels(&self) -> &ChannelList {
        &self.channels
    }

    /// Replaces the Channels the picker offers.
    pub fn set_channels(&mut self, channels: ChannelList) {
        self.channels = channels;
    }

    /// Adds `path` as a manual entry; returns whether it was not listed yet.
    pub fn add_manual(&mut self, path: PathBuf) -> bool {
        self.channels.add(path)
    }

    /// Returns the recently opened Archives.
    #[must_use]
    pub const fn recent(&self) -> &RecentArchives {
        &self.recent
    }

    /// Replaces the recently opened Archives.
    pub fn set_recent(&mut self, recent: RecentArchives) {
        self.recent = recent;
    }

    /// Starts opening the Archive at `path` on a background thread, replacing an opening that
    /// is still running; `wake` is called from that thread once the result is ready.
    pub fn open(&mut self, path: PathBuf, wake: impl Fn() + Send + 'static) {
        let (sender, result) = mpsc::channel();
        let thread_path = path.clone();
        let spawned = thread::Builder::new()
            .name("open-archive".to_owned())
            .spawn(move || {
                let index = Archive::open(&thread_path).map(EntryIndex::build);
                if sender.send(index).is_ok() {
                    wake();
                }
            });
        self.opening = None;
        match spawned {
            Ok(_handle) => {
                self.failure = None;
                self.opening = Some(Opening { path, result });
            }
            Err(error) => self.fail(path, &error),
        }
    }

    /// Takes the result of a finished opening; returns whether one finished.
    pub fn poll(&mut self) -> bool {
        let Some(opening) = &self.opening else {
            return false;
        };
        let received = match opening.result.try_recv() {
            Ok(received) => received,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                let path = opening.path.clone();
                self.opening = None;
                self.failure = Some(Failure { path, reason: None });
                return true;
            }
        };
        let Some(opening) = self.opening.take() else {
            return false;
        };
        match received {
            Ok(index) => {
                let path = opening.path;
                self.recent.push(std::path::absolute(&path).unwrap_or(path));
                self.browser = Some(Browser::new(index));
                self.apply_record_language();
                self.stopped_by_change = false;
                self.query.archive_opened();
                self.localization.archive_opened();
            }
            Err(error) => self.fail(opening.path, &error),
        }
        true
    }

    /// Returns the Archive being opened, or `None` when nothing is opening.
    #[must_use]
    pub fn opening_path(&self) -> Option<&Path> {
        self.opening.as_ref().map(|opening| opening.path.as_path())
    }

    /// Returns why the last opening failed, or `None` when it did not.
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// Returns the browsing state of the open Archive, or `None` when none is open.
    #[must_use]
    pub const fn browser(&self) -> Option<&Browser> {
        self.browser.as_ref()
    }

    /// Returns the browsing state of the open Archive for changes.
    pub const fn browser_mut(&mut self) -> Option<&mut Browser> {
        self.browser.as_mut()
    }

    /// Returns whether the font licence is shown.
    #[must_use]
    pub const fn licence_shown(&self) -> bool {
        self.licence_shown
    }

    /// Shows or hides the font licence.
    pub const fn set_licence_shown(&mut self, shown: bool) {
        self.licence_shown = shown;
    }

    /// Returns the tab the preview is shown in.
    #[must_use]
    pub const fn preview_tab(&self) -> PreviewTab {
        self.preview_tab
    }

    /// Shows the preview in `tab`.
    pub fn set_preview_tab(&mut self, tab: PreviewTab) {
        self.preview_tab = tab;
        if tab != PreviewTab::Audio {
            self.pause_playback();
        }
    }

    fn fail(&mut self, path: PathBuf, error: &dyn Error) {
        self.failure = Some(Failure {
            path,
            reason: Some(describe(error)),
        });
    }
}

/// Returns `error` and its causes, joined by colons.
pub(crate) fn describe(error: &dyn Error) -> String {
    let mut reason = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        reason.push_str(": ");
        reason.push_str(&cause.to_string());
        source = cause.source();
    }
    reason
}
