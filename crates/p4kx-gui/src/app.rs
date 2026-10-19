use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use p4kx_settings::{Settings, WindowGeometry};

use crate::ui::Dialog;
use crate::{ComparisonText, Model, QueryExportFormat, QueryText, Text, ui, window};

/// The eframe application: a [`Model`] drawn by [`ui::show`], with the open dialog and the
/// settings file around it.
#[derive(Debug)]
pub struct App {
    model: Model,
    settings_file: Option<PathBuf>,
    window: Option<WindowGeometry>,
    language_chosen: bool,
    dialog: Option<(Dialog, Receiver<Option<PathBuf>>)>,
}

impl App {
    /// Wraps `model`, remembering language, window geometry, recent Archives and manual entries
    /// in `settings_file`.
    #[must_use]
    pub const fn new(
        model: Model,
        settings_file: Option<PathBuf>,
        window: Option<WindowGeometry>,
    ) -> Self {
        Self {
            model,
            settings_file,
            window,
            language_chosen: false,
            dialog: None,
        }
    }

    /// Returns the model.
    #[must_use]
    pub const fn model(&self) -> &Model {
        &self.model
    }

    fn start_dialog(&mut self, ctx: &egui::Context, dialog: Dialog) {
        if self.dialog.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let filter = self.model.texts().get(Text::DialogArchiveFilter);
        let report_filter = self.model.texts().get(ComparisonText::ReportFilter);
        let export_filter = match dialog {
            Dialog::QueryExport(QueryExportFormat::Json) => QueryText::FilterJson,
            _ => QueryText::FilterCsv,
        };
        let export_filter = self.model.texts().get(export_filter);
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("file-dialog".to_owned())
            .spawn(move || {
                let picker = rfd::FileDialog::new();
                let path = match dialog {
                    Dialog::AddFolder | Dialog::ExtractionFolder | Dialog::RecordExportFolder => {
                        picker.pick_folder()
                    }
                    Dialog::OpenArchive | Dialog::AddArchive | Dialog::ComparisonArchive(_) => {
                        picker.add_filter(filter, &["p4k"]).pick_file()
                    }
                    Dialog::ComparisonReport => picker
                        .add_filter(report_filter, &["json"])
                        .set_file_name("comparison.json")
                        .save_file(),
                    Dialog::QueryExport(format) => {
                        let extension = format.extension();
                        picker
                            .add_filter(export_filter, &[extension])
                            .set_file_name(format!("query.{extension}"))
                            .save_file()
                    }
                };
                if sender.send(path).is_ok() {
                    ctx.request_repaint();
                }
            });
        if spawned.is_ok() {
            self.dialog = Some((dialog, receiver));
        }
    }

    fn take_dialog_result(&mut self) -> Option<(Dialog, PathBuf)> {
        let (dialog, receiver) = self.dialog.as_ref()?;
        let dialog = *dialog;
        let picked = receiver.try_recv();
        match picked {
            Ok(path) => {
                self.dialog = None;
                path.map(|path| (dialog, path))
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.dialog = None;
                None
            }
        }
    }

    /// Stores the settings into the file freshly read, so that what another window stored
    /// meanwhile stays; the saved Queries get this window's edits replayed by name. A file that
    /// cannot be read is never written over.
    fn save_settings(&mut self) {
        let Some(file) = &self.settings_file else {
            return;
        };
        let Ok(mut settings) = Settings::load(file) else {
            return;
        };
        remember(
            &mut settings,
            &self.model,
            self.language_chosen,
            self.window,
        );
        let edits = self.model.query().saved_edits();
        for edit in edits {
            settings.queries.apply(edit);
        }
        let edited = !edits.is_empty();
        if settings.save(file).is_ok() && edited {
            self.model.query_mut().edits_stored(settings.queries);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut settings_changed = false;
        match self.take_dialog_result() {
            Some((Dialog::OpenArchive, path)) => ui::open(ui, &mut self.model, path),
            Some((Dialog::AddFolder | Dialog::AddArchive, path)) => {
                settings_changed = self.model.add_manual(path);
            }
            Some((Dialog::ExtractionFolder, path)) => {
                if let Some(setup) = self.model.extraction_setup_mut() {
                    setup.set_folder(path);
                }
            }
            Some((Dialog::RecordExportFolder, path)) => {
                if let Some(setup) = self.model.record_export_setup_mut() {
                    setup.set_folder(path);
                }
            }
            Some((Dialog::ComparisonArchive(side), path)) => {
                self.model.comparison_mut().set_archive(side, path);
            }
            Some((Dialog::QueryExport(format), path)) => {
                let ctx = ui.ctx().clone();
                self.model
                    .query_mut()
                    .export(path, format, move || ctx.request_repaint());
            }
            Some((Dialog::ComparisonReport, path)) => {
                let ctx = ui.ctx().clone();
                if let Some(view) = self.model.comparison_mut().view_mut() {
                    view.save_report(path, move || ctx.request_repaint());
                }
            }
            None => {}
        }
        let requests = ui::show(ui, &mut self.model);
        if let Some(dialog) = requests.dialog {
            self.start_dialog(ui.ctx(), dialog);
        }
        if requests.language_changed {
            self.language_chosen = true;
        }
        if settings_changed || requests.settings_changed || requests.language_changed {
            self.save_settings();
        }
        self.window = ui.input(|input| window::geometry(self.window, input.viewport()));
    }

    fn on_exit(&mut self) {
        self.model.stop_extraction();
        self.model.stop_record_export();
        self.model.comparison_mut().cancel();
        self.model.query_mut().cancel();
        self.save_settings();
    }
}

/// Stores the window geometry, the recent Archives, the manual entries added in the window next
/// to those already in `settings`, and the language, which stays unset to follow the system until the user chooses
/// one.
fn remember(settings: &mut Settings, model: &Model, chosen: bool, window: Option<WindowGeometry>) {
    if chosen || settings.language.is_some() {
        settings.language = Some(model.texts().language().tag().to_owned());
    }
    settings.window = window;
    settings.recent = model.recent().clone();
    settings.extraction_folder = model.extraction_folder().map(Path::to_path_buf);
    settings.extraction_conversion = model.extraction_conversion();
    settings.extraction_textures = model.extraction_textures();
    settings.update_check = model.update_check();
    settings.channels.merge(model.channels().added());
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use p4kx_channels::SearchPaths;
    use p4kx_settings::{
        ExtractionTextures, ManualChannels, RecentArchives, SavedQueries, Settings,
    };

    use super::{App, remember};
    use crate::{ChannelList, Language, Model};

    #[test]
    fn the_language_stays_unset_until_the_user_chooses_one() {
        let mut settings = Settings::default();

        remember(&mut settings, &Model::new(Language::German), false, None);
        let unchosen = settings.language.clone();
        remember(
            &mut settings,
            &Model::new(Language::BritishEnglish),
            true,
            None,
        );

        assert_eq!(unchosen, None, "follows the system");
        assert_eq!(settings.language.as_deref(), Some("en-GB"), "chosen");
    }

    #[test]
    fn manual_entries_join_those_already_in_the_settings() -> Result<(), std::io::Error> {
        let folder = tempfile::tempdir()?;
        let mut settings = Settings::default();
        settings.channels.add_folder(PathBuf::from(r"D:\Elsewhere"));
        let mut model = Model::new(Language::German);
        model.add_manual(folder.path().to_path_buf());

        remember(&mut settings, &model, false, None);

        assert_eq!(
            settings.channels.folders,
            [PathBuf::from(r"D:\Elsewhere"), folder.path().to_path_buf()],
            "both folders are kept"
        );
        Ok(())
    }

    #[test]
    fn the_extraction_folder_is_stored() {
        let mut model = Model::new(Language::German);
        model.set_extraction_folder(Some(PathBuf::from(r"D:\Exports")));
        let mut settings = Settings::default();

        remember(&mut settings, &model, false, None);

        assert_eq!(
            settings.extraction_folder,
            Some(PathBuf::from(r"D:\Exports")),
            "the folder chosen last"
        );
    }

    #[test]
    fn the_update_check_is_stored() {
        let mut model = Model::new(Language::German);
        model.set_update_check(true);
        let mut settings = Settings::default();

        remember(&mut settings, &model, false, None);

        assert!(settings.update_check, "switched on");
    }

    #[test]
    fn the_conversion_switch_is_stored() {
        let mut model = Model::new(Language::German);
        model.set_extraction_conversion(true);
        let mut settings = Settings::default();

        remember(&mut settings, &model, false, None);

        assert!(settings.extraction_conversion, "switched on");
    }

    #[test]
    fn the_texture_outputs_are_stored() {
        let mut model = Model::new(Language::German);
        let textures = ExtractionTextures {
            merge: false,
            png: true,
            mip: 3,
            keep_raw: true,
        };
        model.set_extraction_textures(textures);
        let mut settings = Settings::default();

        remember(&mut settings, &model, false, None);

        assert_eq!(settings.extraction_textures, textures, "stored");
    }

    #[test]
    fn an_entry_removed_elsewhere_is_not_written_back() {
        let mut model = Model::new(Language::German);
        let mut loaded = ManualChannels::default();
        loaded.add_folder(PathBuf::from(r"D:\Removed"));
        model.set_channels(ChannelList::new(SearchPaths::default(), loaded));
        let mut recent = RecentArchives::default();
        recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));
        model.set_recent(recent.clone());
        let mut settings = Settings::default();

        remember(&mut settings, &model, false, None);

        assert!(settings.channels.folders.is_empty(), "stays removed");
        assert_eq!(settings.recent, recent, "the recent Archives are stored");
    }

    fn app_over(file: &std::path::Path) -> App {
        App::new(Model::new(Language::German), Some(file.to_path_buf()), None)
    }

    fn save(app: &mut App, name: &str, text: &str) {
        let query = app.model.query_mut();
        query.set_text(text);
        query.begin_save();
        if let Some(dialog) = query.saved_dialog_mut()
            && let Some(typed) = dialog.name_mut()
        {
            name.clone_into(typed);
        }
        query.submit_saved_dialog();
        app.save_settings();
    }

    fn names(queries: &SavedQueries) -> Vec<&str> {
        queries
            .iter()
            .map(p4kx_settings::SavedQuery::name)
            .collect()
    }

    #[test]
    fn two_windows_over_one_file_keep_each_others_saved_queries()
    -> Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("settings.toml");
        let mut first = app_over(&file);
        let mut second = app_over(&file);

        save(&mut first, "A", "from A");
        save(&mut second, "B", "from B");
        assert_eq!(names(second.model.query().saved()), ["A", "B"], "merged");
        second.model.query_mut().begin_delete(1);
        second.model.query_mut().submit_saved_dialog();
        second.save_settings();
        first.save_settings();

        let stored = Settings::load(&file)?;
        assert_eq!(names(&stored.queries), ["A"], "B saved and deleted, A kept");
        assert_eq!(
            names(first.model.query().saved()),
            ["A"],
            "the window without new edits keeps its list"
        );
        assert!(first.model.query().saved_edits().is_empty(), "stored");
        Ok(())
    }

    #[test]
    fn a_window_without_edits_never_writes_the_saved_queries()
    -> Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("settings.toml");
        let mut settings = Settings::default();
        settings.queries.save("Theirs", "from A")?;
        let mut idle = app_over(&file);
        settings.save(&file)?;

        idle.save_settings();

        assert_eq!(names(&Settings::load(&file)?.queries), ["Theirs"], "kept");
        Ok(())
    }

    #[test]
    fn a_file_that_cannot_be_read_is_never_written_over() -> Result<(), Box<dyn std::error::Error>>
    {
        let folder = tempfile::tempdir()?;
        let file = folder.path().join("settings.toml");
        std::fs::write(&file, "queries = [ broken")?;
        let mut app = app_over(&file);

        save(&mut app, "A", "from A");

        assert_eq!(
            std::fs::read_to_string(&file)?,
            "queries = [ broken",
            "untouched"
        );
        assert_eq!(app.model.query().saved_edits().len(), 1, "kept for later");
        Ok(())
    }
}
