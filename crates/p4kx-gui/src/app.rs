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
mod tests;
