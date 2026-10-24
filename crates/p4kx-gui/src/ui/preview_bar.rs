// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Id, Layout, Rect, Ui, vec2};

use super::record_preview::{self, RecordStep};
use crate::theme::tokens::{SPACE_2, TOUCH_TARGET};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::tab;
use crate::{
    AssemblyExportText, AudioText, Browser, DataForgeText, ImageText, IncomingText,
    ModelExportText, ModelInfoText, ModelText, Preview, PreviewTab, RecordText, Text, Texts,
    XmlDocument, XmlPreview, XmlText,
};

pub(super) const TABS: [(PreviewTab, Text); 10] = [
    (PreviewTab::Text, Text::PreviewTabText),
    (PreviewTab::Hex, Text::PreviewTabHex),
    (PreviewTab::Xml, Text::Xml(XmlText::Tab)),
    (PreviewTab::Image, Text::Image(ImageText::Tab)),
    (PreviewTab::Audio, Text::Audio(AudioText::Tab)),
    (PreviewTab::Model, Text::Model(ModelText::Tab)),
    (PreviewTab::ModelInfo, Text::ModelInfo(ModelInfoText::Tab)),
    (PreviewTab::DataForge, Text::DataForge(DataForgeText::Tab)),
    (PreviewTab::Record, Text::Record(RecordText::Tab)),
    (PreviewTab::Incoming, Text::Incoming(IncomingText::Tab)),
];

/// The export action the tab bar offers for the selected Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Export {
    /// Export as GLB, of a model.
    Model,
    /// Export Interior as GLB, of a socpak container.
    Interior,
}

/// What the tab bar asks for: the tab to show, the text to copy, a step through the Records and
/// the export that was clicked.
pub(super) struct Bar {
    pub(super) tab: PreviewTab,
    pub(super) copy: Option<String>,
    pub(super) step: Option<RecordStep>,
    pub(super) export: Option<Export>,
}

/// Shows the tabs `browser` offers with back and forward through the Records, the export of a
/// model or of an Interior and the copy action; the tab to show is the first of `shown` unless
/// another was clicked or it is not offered, and its second says which export action is shown,
/// and if so whether it is enabled.
pub(super) fn tab_bar(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    shown: (PreviewTab, Option<(Export, bool)>),
) -> Bar {
    let (wanted, export) = shown;
    let (tabs, mut tab) = offered_tabs(browser, wanted);
    let record = browser.selected_record().is_some();
    let xml = browser.preview().and_then(Preview::xml).filter(|_| !record);
    let labels: Vec<String> = tabs.iter().map(|&(_, text)| texts.get(text)).collect();
    let current = tabs
        .iter()
        .position(|(candidate, _)| *candidate == tab)
        .unwrap_or(0);
    let bar = ui.max_rect().with_max_y(ui.max_rect().top() + TOUCH_TARGET);
    let controls_id = Id::unique("p4kx-preview-controls");
    let reserved = ui
        .data(|data| data.get_temp::<f32>(controls_id))
        .unwrap_or(0.0);
    if let Some(index) = tab::tab_bar_reserving(ui, &labels, current, reserved + SPACE_2)
        && let Some((candidate, _)) = tabs.get(index)
    {
        tab = *candidate;
    }
    let contents = browser
        .preview()
        .and_then(|preview| preview.contents())
        .filter(|_| matches!(tab, PreviewTab::Text | PreviewTab::Hex));
    let document = shown_xml(browser, xml).filter(|_| tab == PreviewTab::Xml);
    let enabled = contents.is_some() || document.is_some();
    let copy_text = || {
        contents
            .map(|contents| contents.copied(tab))
            .or_else(|| document.map(|document| String::from_utf8_lossy(document).into_owned()))
    };
    let shown = controls(ui, (texts, browser), bar, (enabled, copy_text, export));
    let width = bar.right() - shown.left;
    if (width - reserved).abs() > 0.5 {
        ui.data_mut(|data| data.insert_temp(controls_id, width));
        ui.ctx().request_repaint();
    }
    Bar {
        tab,
        copy: shown.copy,
        step: shown.step,
        export: shown.export,
    }
}

/// Returns the tabs `browser` offers and the tab to show: `wanted` when it is offered, else Text.
fn offered_tabs(browser: &Browser, wanted: PreviewTab) -> (Vec<(PreviewTab, Text)>, PreviewTab) {
    let image = browser.preview().and_then(Preview::image);
    let audio = browser.preview().and_then(Preview::audio);
    let sounds = browser.soundbank().is_some();
    let modelled = browser.preview().and_then(Preview::model).is_some();
    let record = browser.selected_record().is_some();
    let xml = browser.preview().and_then(Preview::xml).filter(|_| !record);
    let records = browser.records();
    let offered = |candidate: PreviewTab| match candidate {
        PreviewTab::Xml => xml.is_some() || record,
        PreviewTab::Image => image.is_some(),
        PreviewTab::Audio => audio.is_some() || sounds,
        PreviewTab::Model | PreviewTab::ModelInfo => modelled,
        PreviewTab::DataForge => records.is_some(),
        PreviewTab::Record | PreviewTab::Incoming => record,
        PreviewTab::Text | PreviewTab::Hex => true,
    };
    let tabs = TABS
        .into_iter()
        .filter(|&(candidate, _)| offered(candidate))
        .collect();
    let tab = if offered(wanted) {
        wanted
    } else {
        PreviewTab::Text
    };
    (tabs, tab)
}

/// What the controls at the right end of the tab bar asked for, and where they start.
struct Controls {
    copy: Option<String>,
    step: Option<RecordStep>,
    export: Option<Export>,
    left: f32,
}

/// Shows Copy, the Export as GLB action of a model or an Interior and, over Records, back and
/// forward at the right end of `bar`; `copy` gives the text when `enabled` and Copy is clicked,
/// and `export` says which export action is shown and whether it is enabled.
fn controls(
    ui: &mut Ui,
    (texts, browser): (&Texts, &Browser),
    bar: Rect,
    (enabled, copy, export): (
        bool,
        impl FnOnce() -> Option<String>,
        Option<(Export, bool)>,
    ),
) -> Controls {
    let label = texts.get(Text::PreviewCopy);
    let mut shown = Controls {
        copy: None,
        step: None,
        export: None,
        left: bar.right(),
    };
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(bar.shrink2(vec2(SPACE_2, 0.0))),
        |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button::button(ui, Kind::Ghost, Size::Compact, &label, enabled).clicked() {
                    shown.copy = copy();
                }
                if let Some((kind, available)) = export {
                    let label = match kind {
                        Export::Model => texts.get(ModelExportText::Action),
                        Export::Interior => texts.get(AssemblyExportText::ActionInterior),
                    };
                    if button::button(ui, Kind::Ghost, Size::Compact, &label, available).clicked() {
                        shown.export = Some(kind);
                    }
                }
                if browser.shows_records() {
                    shown.step = steps(ui, texts, browser);
                }
                shown.left = ui.cursor().right();
            });
        },
    );
    shown
}
/// Returns the XML the XML tab shows: that of the selected Record, else the converted Entry.
fn shown_xml<'b>(browser: &'b Browser, xml: Option<&'b XmlPreview>) -> Option<&'b [u8]> {
    if browser.selected_record().is_some() {
        record_preview::xml_bytes(browser)
    } else {
        xml.and_then(XmlPreview::text).map(XmlDocument::bytes)
    }
}

/// Shows forward and back, right to left; returns the step that was clicked.
fn steps(ui: &mut Ui, texts: &Texts, browser: &Browser) -> Option<RecordStep> {
    let forward = texts.get(RecordText::Forward);
    let back = texts.get(RecordText::Back);
    let forward = button::button(
        ui,
        Kind::Ghost,
        Size::Compact,
        &forward,
        browser.can_go_forward(),
    )
    .clicked();
    let back =
        button::button(ui, Kind::Ghost, Size::Compact, &back, browser.can_go_back()).clicked();
    if back {
        Some(RecordStep::Back)
    } else {
        forward.then_some(RecordStep::Forward)
    }
}
