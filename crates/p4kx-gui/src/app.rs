use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use p4kx_settings::{Settings, WindowGeometry};

use crate::{Language, Model, Text, ui, window};

/// The eframe application: a [`Model`] drawn by [`ui::show`], with the open dialog and the
/// settings file around it.
#[derive(Debug)]
pub struct App {
    model: Model,
    settings_file: Option<PathBuf>,
    window: Option<WindowGeometry>,
    language_chosen: bool,
    dialog: Option<Receiver<Option<PathBuf>>>,
}

impl App {
    /// Wraps `model`, remembering language and window geometry in `settings_file`.
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

    fn start_dialog(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let filter = self.model.texts().get(Text::DialogArchiveFilter);
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("open-dialog".to_owned())
            .spawn(move || {
                let path = rfd::FileDialog::new()
                    .add_filter(filter, &["p4k"])
                    .pick_file();
                if sender.send(path).is_ok() {
                    ctx.request_repaint();
                }
            });
        if spawned.is_ok() {
            self.dialog = Some(receiver);
        }
    }

    fn take_dialog_result(&mut self) -> Option<PathBuf> {
        let picked = self.dialog.as_ref()?.try_recv();
        match picked {
            Ok(path) => {
                self.dialog = None;
                path
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.dialog = None;
                None
            }
        }
    }

    fn save_settings(&self) {
        let Some(file) = &self.settings_file else {
            return;
        };
        let Ok(mut settings) = Settings::load(file) else {
            return;
        };
        remember(
            &mut settings,
            self.model.texts().language(),
            self.language_chosen,
            self.window,
        );
        let _ignored = settings.save(file);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(path) = self.take_dialog_result() {
            ui::open(ui, &mut self.model, path);
        }
        let requests = ui::show(ui, &mut self.model);
        if requests.open_dialog {
            self.start_dialog(ui.ctx());
        }
        if requests.language_changed {
            self.language_chosen = true;
            self.save_settings();
        }
        self.window = ui.input(|input| window::geometry(self.window, input.viewport()));
    }

    fn on_exit(&mut self) {
        self.save_settings();
    }
}

/// Stores the window geometry and the language, which stays unset to follow the system until the
/// user chooses one.
fn remember(
    settings: &mut Settings,
    language: Language,
    chosen: bool,
    window: Option<WindowGeometry>,
) {
    if chosen || settings.language.is_some() {
        settings.language = Some(language.tag().to_owned());
    }
    settings.window = window;
}

#[cfg(test)]
mod tests {
    use p4kx_settings::Settings;

    use super::remember;
    use crate::Language;

    #[test]
    fn the_language_stays_unset_until_the_user_chooses_one() {
        let mut settings = Settings::default();

        remember(&mut settings, Language::German, false, None);
        let unchosen = settings.language.clone();
        remember(&mut settings, Language::BritishEnglish, true, None);

        assert_eq!(unchosen, None, "follows the system");
        assert_eq!(settings.language.as_deref(), Some("en-GB"), "chosen");
    }
}
