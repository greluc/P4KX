use egui::{TextEdit, Ui};

use crate::{Model, Text};

/// The Filters the quick-filter chips apply.
const QUICK_FILTERS: [&str; 5] = ["*.xml", "*.dds", "*.dcb", "Game2.dcb", "aurora"];

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let label = texts.get(Text::SearchLabel);
    let hint = texts.get(Text::SearchHint);
    let Some(browser) = model.browser() else {
        return;
    };
    let mut text = browser.search_text().to_owned();
    let mut changed = false;
    ui.horizontal(|ui| {
        let label = ui.label(label);
        let field = TextEdit::singleline(&mut text)
            .hint_text(hint)
            .desired_width(360.0);
        changed = ui.add(field).labelled_by(label.id).changed();
        for chip in QUICK_FILTERS {
            let active = text == chip;
            if ui.selectable_label(active, chip).clicked() {
                text = if active {
                    String::new()
                } else {
                    chip.to_owned()
                };
                changed = true;
            }
        }
    });
    if !changed {
        return;
    }
    let ctx = ui.ctx().clone();
    if let Some(browser) = model.browser_mut() {
        browser.search(&text, move || ctx.request_repaint());
    }
}
