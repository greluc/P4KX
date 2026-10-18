use std::time::SystemTime;

use egui::{Button, Id, RadioButton, Ui, Window};
use fluent_bundle::FluentValue;
use p4kx_extraction::{ConflictPolicy, default_target_name};

use super::{Dialog, Requests};
use crate::extraction::target_naming;
use crate::{
    Extraction, ExtractionScope, ExtractionSetup, ExtractionText as T, Model, TargetKind, Texts,
    format,
};

const ACTIONS: [(ExtractionScope, T); 3] = [
    (ExtractionScope::Selected, T::ActionSelected),
    (ExtractionScope::Shown, T::ActionShown),
    (ExtractionScope::All, T::ActionAll),
];

const SCOPES: [(ExtractionScope, T); 3] = [
    (ExtractionScope::Selected, T::ScopeSelected),
    (ExtractionScope::Shown, T::ScopeShown),
    (ExtractionScope::All, T::ScopeAll),
];

const POLICIES: [(ConflictPolicy, T); 3] = [
    (ConflictPolicy::Skip, T::PolicySkip),
    (ConflictPolicy::Overwrite, T::PolicyOverwrite),
    (ConflictPolicy::IfChanged, T::PolicyIfChanged),
];

enum Action {
    Submit,
    Close,
    ChooseFolder,
}

pub(super) fn actions(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let running = model.extraction().is_some_and(Extraction::is_running);
    let mut begin = None;
    ui.horizontal(|ui| {
        for (scope, text) in ACTIONS {
            let enabled = !running && model.scope_len(scope) > 0;
            if ui
                .add_enabled(enabled, Button::new(texts.get(text)))
                .clicked()
            {
                begin = Some(scope);
            }
        }
    });
    if let Some(scope) = begin {
        model.begin_extraction(scope);
    }
}

pub(super) fn setup(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let Some(current) = model.extraction_setup() else {
        return;
    };
    let mut setup = current.clone();
    let mut action = None;
    let mut open = true;
    Window::new(model.texts().get(T::SetupTitle))
        .id(Id::unique("extraction-setup"))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            action = if setup.is_confirming() {
                confirm(ui, model, &mut setup)
            } else {
                choices(ui, model, &mut setup)
            };
        });
    if let Some(slot) = model.extraction_setup_mut() {
        *slot = setup;
    }
    if !open {
        model.close_extraction_setup();
        return;
    }
    match action {
        Some(Action::Submit) => {
            let ctx = ui.ctx().clone();
            requests.settings_changed |= model.submit_extraction(move || ctx.request_repaint());
        }
        Some(Action::Close) => model.close_extraction_setup(),
        Some(Action::ChooseFolder) => requests.dialog = Some(Dialog::ExtractionFolder),
        None => {}
    }
}

fn choices(ui: &mut Ui, model: &Model, setup: &mut ExtractionSetup) -> Option<Action> {
    let texts = model.texts();
    let mut action = None;
    ui.strong(texts.get(T::ScopeHeading));
    for (scope, text) in SCOPES {
        let len = model.scope_len(scope);
        let label = texts.format(text, &[("number", number(texts, len))]);
        let radio = RadioButton::new(setup.scope() == scope, label);
        if ui.add_enabled(len > 0, radio).clicked() {
            setup.set_scope(scope);
        }
    }
    ui.separator();
    ui.strong(texts.get(T::TargetHeading));
    for (kind, text) in [
        (TargetKind::NewFolder, T::TargetNew),
        (TargetKind::ExistingFolder, T::TargetExisting),
    ] {
        if ui.radio(setup.kind() == kind, texts.get(text)).clicked() {
            setup.set_kind(kind);
        }
    }
    ui.horizontal(|ui| {
        match setup.folder() {
            Some(folder) => ui.label(folder.display().to_string()),
            None => ui.weak(texts.get(T::FolderNone)),
        };
        if ui.button(texts.get(T::ChooseFolder)).clicked() {
            action = Some(Action::ChooseFolder);
        }
    });
    writing(ui, model, setup);
    ui.separator();
    ui.horizontal(|ui| {
        let ready = setup.folder().is_some() && model.scope_len(setup.scope()) > 0;
        if ui
            .add_enabled(ready, Button::new(texts.get(T::Start)))
            .clicked()
        {
            action = Some(Action::Submit);
        }
        if ui.button(texts.get(T::Close)).clicked() {
            action = Some(Action::Close);
        }
    });
    action
}

/// Shows the name of a new folder or the Conflict policy for an existing one, then the
/// Conversion switch.
fn writing(ui: &mut Ui, model: &Model, setup: &mut ExtractionSetup) {
    let texts = model.texts();
    if setup.kind() == TargetKind::NewFolder {
        let name = new_folder_name(model);
        ui.weak(texts.format(T::TargetNewName, &[("name", name.into())]));
    } else {
        ui.separator();
        ui.strong(texts.get(T::PolicyHeading));
        for (policy, text) in POLICIES {
            if ui
                .radio(setup.policy() == policy, texts.get(text))
                .clicked()
            {
                setup.set_policy(policy);
            }
        }
    }
    ui.separator();
    let mut convert = setup.convert();
    if ui
        .checkbox(&mut convert, texts.get(T::Conversion))
        .changed()
    {
        setup.set_convert(convert);
    }
}

fn confirm(ui: &mut Ui, model: &Model, setup: &mut ExtractionSetup) -> Option<Action> {
    let texts = model.texts();
    let asked = setup.confirmation()?;
    ui.label(texts.format(
        T::ConfirmAll,
        &[
            (
                "count",
                u64::try_from(asked.entries).unwrap_or(u64::MAX).into(),
            ),
            ("number", number(texts, asked.entries)),
            ("size", format::size(asked.bytes, texts.language()).into()),
        ],
    ));
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button(texts.get(T::ConfirmAllAction)).clicked() {
            action = Some(Action::Submit);
        }
        if ui.button(texts.get(T::Back)).clicked() {
            setup.withdraw_confirmation();
        }
    });
    action
}

fn new_folder_name(model: &Model) -> String {
    let (channel, build) = model
        .browser()
        .map(|browser| target_naming(browser.index().archive()))
        .unwrap_or_default();
    default_target_name(channel.as_deref(), build, SystemTime::now())
}

pub(super) fn number(texts: &Texts, value: usize) -> FluentValue<'static> {
    format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language()).into()
}
