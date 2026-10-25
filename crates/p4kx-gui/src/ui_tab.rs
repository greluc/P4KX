// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod contents;
mod export;
mod files;
mod job;
mod pick;
mod render;
mod report;
mod settings;
mod ships;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use p4kx_assembly::EntityCatalogue;
use p4kx_screens::ScreenTexts;

pub use contents::{ContentRow, MAX_CONTENT_ROWS, UiContents};
pub use export::{UiExport, UiExportStatus, UiExportSummary};
pub use files::FileList;
pub use pick::{ShipPick, UiScreensStatus};
pub use render::{UiRendered, UiSource};
pub use report::{ReportLine, UiReport};
pub use settings::{UiBackground, UiFormat, UiSettings, UiSize};
pub use ships::ShipList;
pub use view::{UiDetail, UiFilesStatus, UiMode, UiRenderFailure};

use crate::{EntryIndex, TextLanguage};
use job::{Job, Poll};
use render::{Built, Fail, RenderKey};

type Drawing = Result<Built, Fail>;

#[derive(Debug, Default)]
struct Rendering {
    requested: Option<RenderKey>,
    job: Option<Job<Drawing>>,
    shown: Option<UiRendered>,
    failure: Option<UiRenderFailure>,
}

#[derive(Debug, Default)]
enum Files {
    #[default]
    Idle,
    Loading(Job<files::Outcome>),
    Failed(Option<String>),
    ArchiveChanged,
    Ready(Box<FileList>),
}

/// The UI tab: the GFx movies of the open Archive, and the screens of its ships, each drawn as
/// `p4kx ui render` draws them, with the choices of size, background and format, and the export
/// of the picture.
#[derive(Debug, Default)]
pub struct UiTab {
    mode: UiMode,
    detail: UiDetail,
    settings: UiSettings,
    files_query: String,
    ships_query: String,
    files: Files,
    file: Option<String>,
    ships: Option<ShipList>,
    ship: Option<ShipPick>,
    rendering: Rendering,
    export: Option<UiExport>,
    texts: Option<(Option<TextLanguage>, Arc<ScreenTexts>)>,
    serial: u64,
}

impl UiTab {
    /// Returns what the tab lists: the files or the ships.
    #[must_use]
    pub const fn mode(&self) -> UiMode {
        self.mode
    }

    /// Lists the files or the ships.
    pub const fn set_mode(&mut self, mode: UiMode) {
        self.mode = mode;
    }

    /// Returns the part of the details that is shown.
    #[must_use]
    pub const fn detail(&self) -> UiDetail {
        self.detail
    }

    /// Shows the contents or the report below the picture.
    pub const fn set_detail(&mut self, detail: UiDetail) {
        self.detail = detail;
    }

    /// Returns the choices of how the tab draws and exports.
    #[must_use]
    pub const fn settings(&self) -> &UiSettings {
        &self.settings
    }

    /// Draws in `size`.
    pub const fn set_size(&mut self, size: UiSize) {
        self.settings.size = size;
    }

    /// Draws on `background`.
    pub const fn set_background(&mut self, background: UiBackground) {
        self.settings.background = background;
    }

    /// Exports in `format`.
    pub const fn set_format(&mut self, format: UiFormat) {
        self.settings.format = format;
    }

    /// Returns the search text of the list the mode shows.
    #[must_use]
    pub fn query(&self) -> &str {
        match self.mode {
            UiMode::Files => &self.files_query,
            UiMode::Ships => &self.ships_query,
        }
    }

    /// Searches the list the mode shows for `text`, ignoring case; every word has to occur.
    pub fn set_query(&mut self, text: &str) {
        match self.mode {
            UiMode::Files => {
                text.clone_into(&mut self.files_query);
                if let Files::Ready(list) = &mut self.files {
                    list.refilter(text);
                }
            }
            UiMode::Ships => {
                text.clone_into(&mut self.ships_query);
                if let Some(ships) = &mut self.ships {
                    ships.refilter(text);
                }
            }
        }
    }

    /// Returns where the list of files stands.
    #[must_use]
    pub fn files_status(&self) -> UiFilesStatus<'_> {
        match &self.files {
            Files::Idle => UiFilesStatus::Idle,
            Files::Loading(_) => UiFilesStatus::Loading,
            Files::Failed(reason) => UiFilesStatus::Failed(reason.as_deref()),
            Files::ArchiveChanged => UiFilesStatus::ArchiveChanged,
            Files::Ready(list) if list.total() == 0 => UiFilesStatus::NoFiles,
            Files::Ready(list) => UiFilesStatus::Ready(list),
        }
    }

    /// Returns whether the files are listed or being listed.
    #[must_use]
    pub const fn is_files_started(&self) -> bool {
        !matches!(self.files, Files::Idle)
    }

    /// Starts listing the GFx and SWF Entries of `index` off the UI thread, calling `wake` from
    /// it once it ended; returns whether it started. Nothing starts when they are listed or
    /// being listed.
    pub(crate) fn load_files(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        if self.is_files_started() {
            return false;
        }
        self.files = match files::start(index, wake) {
            Ok(job) => Files::Loading(job),
            Err(reason) => Files::Failed(Some(reason)),
        };
        true
    }

    /// Returns the ships and vehicles the picker lists, once the Entities are listed.
    #[must_use]
    pub const fn ships(&self) -> Option<&ShipList> {
        self.ships.as_ref()
    }

    /// Takes the catalogue of the Entities tab as the list of ships, or forgets the list when
    /// there is none.
    pub(crate) fn sync_ships(&mut self, catalogue: Option<&Arc<EntityCatalogue>>) {
        match catalogue {
            Some(catalogue) if !self.ships.as_ref().is_some_and(|s| s.is_of(catalogue)) => {
                let mut list = ShipList::new(Arc::clone(catalogue));
                list.refilter(&self.ships_query);
                self.ships = Some(list);
            }
            Some(_) => {}
            None => self.ships = None,
        }
    }

    /// Returns the path of the picked file.
    #[must_use]
    pub fn selected_file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    /// Picks the file at `position` of the shown ones.
    pub fn select_file(&mut self, position: usize) {
        if let Files::Ready(list) = &self.files
            && let Some(path) = list.get(position)
        {
            self.file = Some(path.to_owned());
        }
    }

    /// Returns the picked ship or vehicle with its screens.
    #[must_use]
    pub const fn ship(&self) -> Option<&ShipPick> {
        self.ship.as_ref()
    }

    /// Picks the ship or vehicle at `position` of the shown ones and lists its screens off the
    /// UI thread, calling `wake` from it as it ends. Picking the one that is picked keeps it.
    pub(crate) fn select_ship(
        &mut self,
        position: usize,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        let Some(entity) = self.ships.as_ref().and_then(|ships| ships.get(position)) else {
            return;
        };
        let (guid, class_name) = (entity.guid(), entity.class_name().to_owned());
        if self.ship.as_ref().is_some_and(|ship| ship.guid() == guid) {
            return;
        }
        let job = ships::list_screens(index, guid, wake);
        self.ship = Some(ShipPick::new(guid, &class_name, job));
    }

    /// Picks the screen at `position` of the listing of the picked ship.
    pub fn pick_screen(&mut self, position: usize) {
        if let Some(ship) = &mut self.ship {
            ship.pick(position);
        }
    }

    /// Returns the picture that was drawn last for what is picked.
    #[must_use]
    pub const fn rendered(&self) -> Option<&UiRendered> {
        self.rendering.shown.as_ref()
    }

    /// Returns whether a picture is being drawn.
    #[must_use]
    pub const fn is_rendering(&self) -> bool {
        self.rendering.job.is_some()
    }

    /// Returns why the last picture could not be drawn.
    #[must_use]
    pub const fn render_failure(&self) -> Option<&UiRenderFailure> {
        self.rendering.failure.as_ref()
    }

    /// Returns what the picked file or screen is made of.
    #[must_use]
    pub fn contents(&self) -> Option<&UiContents> {
        match self.mode {
            UiMode::Files => self
                .rendering
                .shown
                .as_ref()
                .filter(|shown| self.file.as_deref().is_some_and(|f| shown_is(shown, f)))
                .and_then(UiRendered::contents),
            UiMode::Ships => self.ship.as_ref().and_then(ShipPick::contents),
        }
    }

    fn wanted(&self, language: Option<TextLanguage>) -> Option<RenderKey> {
        let source = match self.mode {
            UiMode::Files => UiSource::Movie(self.file.clone()?),
            UiMode::Ships => {
                let ship = self.ship.as_ref()?;
                UiSource::Screen {
                    canvas: ship.screen()?.canvas.clone(),
                    brand: ship.brand().map(str::to_owned),
                }
            }
        };
        Some(RenderKey {
            source,
            size: self.settings.size,
            background: self.settings.background,
            language,
        })
    }

    /// Draws the picture of what is picked off the UI thread, unless it is drawn or being drawn
    /// for these choices and `language`; calls `wake` from the thread once it ended.
    pub(crate) fn sync_render(
        &mut self,
        index: &Arc<EntryIndex>,
        language: Option<TextLanguage>,
        wake: impl Fn() + Send + 'static,
    ) {
        let Some(key) = self.wanted(language) else {
            self.rendering = Rendering::default();
            return;
        };
        if self.rendering.requested.as_ref() == Some(&key) {
            return;
        }
        if self
            .rendering
            .shown
            .as_ref()
            .is_some_and(|shown| shown.source != key.source)
        {
            self.rendering.shown = None;
        }
        let cached = self
            .texts
            .as_ref()
            .filter(|(cached, _)| *cached == key.language)
            .map(|(_, texts)| Arc::clone(texts));
        let (task_index, task_key) = (Arc::clone(index), key.clone());
        let spawned = Job::spawn("ui-render", wake, move || {
            render::draw(&task_index, &task_key, cached)
        });
        match spawned {
            Ok(job) => {
                self.rendering.job = Some(job);
                self.rendering.failure = None;
            }
            Err(reason) => {
                self.rendering.job = None;
                self.rendering.failure = Some(UiRenderFailure::Failed(Some(reason)));
            }
        }
        self.rendering.requested = Some(key);
    }

    /// Takes the screens, the file list and the picture once they are ready, and the outcome of
    /// the export once it ended; returns whether something changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = self.ship.as_mut().is_some_and(ShipPick::poll);
        changed |= self.export.as_mut().is_some_and(UiExport::poll);
        changed |= self.poll_files();
        changed |= self.poll_render();
        changed
    }

    fn poll_files(&mut self) -> bool {
        let Files::Loading(job) = &self.files else {
            return false;
        };
        self.files = match job.poll() {
            Poll::Pending => return false,
            Poll::Ready(files::Outcome::Ready(paths)) => {
                let mut list = FileList::new(paths);
                list.refilter(&self.files_query);
                Files::Ready(Box::new(list))
            }
            Poll::Ready(files::Outcome::ArchiveChanged) => Files::ArchiveChanged,
            Poll::Lost => Files::Failed(None),
        };
        true
    }

    fn poll_render(&mut self) -> bool {
        let Some(job) = &self.rendering.job else {
            return false;
        };
        let outcome = match job.poll() {
            Poll::Pending => return false,
            Poll::Ready(outcome) => outcome,
            Poll::Lost => Err(Fail::Reason(String::new())),
        };
        self.rendering.job = None;
        let Some(key) = self.rendering.requested.clone() else {
            return true;
        };
        match outcome {
            Ok(built) => {
                self.serial += 1;
                if let Some(texts) = built.texts {
                    self.texts = Some((key.language, texts));
                }
                self.rendering.shown = Some(UiRendered {
                    serial: self.serial,
                    source: key.source,
                    image: Arc::new(built.image),
                    report: built.report,
                    contents: built.contents,
                });
                self.rendering.failure = None;
            }
            Err(failure) => {
                self.rendering.shown = None;
                self.rendering.failure = Some(match failure {
                    Fail::ArchiveChanged => UiRenderFailure::ArchiveChanged,
                    Fail::Reason(reason) if reason.is_empty() => UiRenderFailure::Failed(None),
                    Fail::Reason(reason) => UiRenderFailure::Failed(Some(reason)),
                });
            }
        }
        true
    }

    /// Returns whether what is picked can be exported: it is picked and no export runs.
    #[must_use]
    pub fn export_available(&self) -> bool {
        self.wanted(None).is_some() && !self.export.as_ref().is_some_and(UiExport::is_running)
    }

    /// Returns the file name the save dialog proposes for the export of what is picked.
    #[must_use]
    pub fn proposed_file_name(&self) -> String {
        let extension = self.settings.format.extension();
        let stem = match (self.mode, &self.ship) {
            (UiMode::Files, _) => self
                .file
                .as_deref()
                .and_then(|path| path.rsplit(['/', '\\']).next())
                .and_then(|name| name.rsplit_once('.').map(|(stem, _)| stem))
                .unwrap_or("ui")
                .to_owned(),
            (UiMode::Ships, Some(ship)) => {
                let screen = ship
                    .screen()
                    .and_then(|screen| screen.canvas.rsplit('/').next())
                    .unwrap_or("screen");
                format!("{}_{screen}", ship.class_name())
            }
            (UiMode::Ships, None) => "screen".to_owned(),
        };
        format!("{stem}.{extension}")
    }

    /// Draws what is picked again, as the choices say, and writes it to `file` off the UI
    /// thread, calling `wake` from it once it ended; returns whether it started.
    ///
    /// Nothing starts while nothing is picked or an export runs. The file is written whole or
    /// not at all, and never over the open Archive.
    pub(crate) fn start_export(
        &mut self,
        index: &Arc<EntryIndex>,
        (file, language): (PathBuf, Option<TextLanguage>),
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        if !self.export_available() {
            return false;
        }
        let Some(key) = self.wanted(language) else {
            return false;
        };
        let texts = self
            .texts
            .as_ref()
            .filter(|(cached, _)| *cached == key.language)
            .map(|(_, texts)| Arc::clone(texts));
        let request = export::Request {
            key,
            format: self.settings.format,
            file,
            texts,
        };
        self.export = Some(export::start(index, request, wake));
        true
    }

    /// Returns the running or finished export.
    #[must_use]
    pub const fn export(&self) -> Option<&UiExport> {
        self.export.as_ref()
    }

    /// Dismisses the outcome of an export that ended.
    pub fn dismiss_export(&mut self) {
        if self.export.as_ref().is_some_and(|export| !export.is_running()) {
            self.export = None;
        }
    }

    /// Forgets the lists, the picks and the picture, because another Archive was opened.
    pub(crate) fn archive_opened(&mut self) {
        let (mode, detail, settings) = (self.mode, self.detail, self.settings);
        *self = Self {
            mode,
            detail,
            settings,
            ..Self::default()
        };
    }

    /// Stops what the tab shows of the Archive, because it changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        if self.is_files_started() {
            self.files = Files::ArchiveChanged;
        }
        self.rendering = Rendering::default();
        self.export = None;
        self.texts = None;
    }
}

fn shown_is(shown: &UiRendered, path: &str) -> bool {
    matches!(&shown.source, UiSource::Movie(movie) if movie == path)
}
