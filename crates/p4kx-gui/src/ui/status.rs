use egui::Ui;
use fluent_bundle::FluentValue;

use crate::{Browser, Model, Text, Texts, format};

pub(super) fn show(ui: &mut Ui, model: &Model) {
    let texts = model.texts();
    ui.horizontal(|ui| {
        if let Some(path) = model.opening_path() {
            ui.spinner();
            let path = path.display().to_string();
            ui.label(texts.format(Text::StatusOpening, &[("path", path.into())]));
        } else if let Some(failure) = model.failure() {
            let path = failure.path.display().to_string();
            let text = match &failure.reason {
                Some(reason) => texts.format(
                    Text::StatusFailed,
                    &[("path", path.into()), ("reason", reason.clone().into())],
                ),
                None => texts.format(Text::StatusOpeningStopped, &[("path", path.into())]),
            };
            ui.colored_label(ui.visuals().error_fg_color, text);
        } else if model.browser().is_none() {
            ui.label(texts.get(Text::StatusEmpty));
        }
        if let Some(browser) = model.browser() {
            for part in browser_parts(texts, browser) {
                ui.separator();
                ui.label(part);
            }
        }
    });
}

fn browser_parts(texts: &Texts, browser: &Browser) -> [String; 5] {
    let language = texts.language();
    let archive = browser.index().archive();
    let count = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    let number =
        |value: usize| -> FluentValue<'static> { format::number(count(value), language).into() };
    let build = archive.build().map_or_else(
        || texts.get(Text::StatusBuildUnknown),
        |build| {
            texts.format(
                Text::StatusBuild,
                &[("build", build.version().to_owned().into())],
            )
        },
    );
    [
        archive.path().display().to_string(),
        build,
        texts.format(
            Text::StatusEntries,
            &[
                ("count", count(browser.index().len()).into()),
                ("number", number(browser.index().len())),
            ],
        ),
        texts.format(
            Text::StatusShown,
            &[("number", number(browser.shown().len()))],
        ),
        texts.format(
            Text::StatusSelected,
            &[("number", number(browser.checked_count()))],
        ),
    ]
}
