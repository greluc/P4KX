use egui::{Align, Label, Layout, Role, ScrollArea, Sense, Ui, WidgetInfo, vec2};
use fluent_bundle::FluentValue;
use p4kx_extraction::{Entry, Texture};

use super::audio_preview::{self, AudioAction};
use super::{dataforge_summary, hex_view, image_preview, xml_preview};
use crate::theme::desktop::{HEX_ROW_HEIGHT, PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, TOUCH_TARGET, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::tab;
use crate::widgets::text::{hairline_below, line, paint_left, plain};
use crate::{
    AudioText, Browser, DataForgeText, ImageText, Model, PREVIEW_SIZE_LIMIT, Preview,
    PreviewContents, PreviewStatus, PreviewTab, Text, Texts, XmlPreview, XmlText, format,
    is_wem_path,
};

const TABS: [(PreviewTab, Text); 6] = [
    (PreviewTab::Text, Text::PreviewTabText),
    (PreviewTab::Hex, Text::PreviewTabHex),
    (PreviewTab::Xml, Text::Xml(XmlText::Tab)),
    (PreviewTab::Image, Text::Image(ImageText::Tab)),
    (PreviewTab::Audio, Text::Audio(AudioText::Tab)),
    (PreviewTab::DataForge, Text::DataForge(DataForgeText::Tab)),
];

/// What the body of a tab asks for.
enum Asked {
    Mip(u32),
    Audio(AudioAction),
}

/// Shows the Image tab after selecting a texture header, the Audio tab after selecting a WEM,
/// and leaves the XML, Image, Audio and DataForge tabs for the Text tab after selecting another
/// Entry; the XML tab opens once the Entry turns out to be `CryXml`.
pub(super) fn select(model: &mut Model, id: u32) {
    let path = model
        .browser()
        .and_then(|browser| browser.index().entry(id))
        .map(Entry::path);
    if path.is_some_and(Texture::is_header_path) {
        model.set_preview_tab(PreviewTab::Image);
    } else if path.is_some_and(is_wem_path) {
        model.set_preview_tab(PreviewTab::Audio);
    } else if matches!(
        model.preview_tab(),
        PreviewTab::Xml | PreviewTab::Image | PreviewTab::Audio | PreviewTab::DataForge
    ) {
        model.set_preview_tab(PreviewTab::Text);
    }
}

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if let Some(image) = model.browser_mut().and_then(Browser::image_mut) {
        image.upload(ui.ctx());
    }
    if model
        .browser_mut()
        .and_then(Browser::preview_mut)
        .is_some_and(Preview::take_xml_arrival)
    {
        model.set_preview_tab(PreviewTab::Xml);
    }
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let (tab, copy) = tab_bar(ui, texts, browser, model.preview_tab());
    let asked = tab_body(ui, texts, browser, tab, model.audio_volume());
    if let Some(text) = copy {
        ui.ctx().copy_text(text);
    }
    model.set_preview_tab(tab);
    match asked {
        Some(Asked::Mip(level)) => {
            if let Some(image) = model.browser_mut().and_then(Browser::image_mut) {
                image.request(level, image_preview::max_side(ui));
            }
        }
        Some(Asked::Audio(action)) => audio_preview::apply(model, action),
        None => {}
    }
    if tab == PreviewTab::DataForge
        && let Some(browser) = model.browser_mut()
    {
        let ctx = ui.ctx().clone();
        browser.load_records(move || ctx.request_repaint());
    }
}

/// Shows the tabs `browser` offers with the copy action; returns the tab to show, `tab` unless
/// another was clicked or it is not offered, and the text to copy when copy was clicked.
fn tab_bar(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    mut tab: PreviewTab,
) -> (PreviewTab, Option<String>) {
    let image = browser.preview().and_then(Preview::image);
    let audio = browser.preview().and_then(Preview::audio);
    let xml = browser.preview().and_then(Preview::xml);
    let records = browser.records();
    let offered = |candidate: PreviewTab| match candidate {
        PreviewTab::Xml => xml.is_some(),
        PreviewTab::Image => image.is_some(),
        PreviewTab::Audio => audio.is_some(),
        PreviewTab::DataForge => records.is_some(),
        PreviewTab::Text | PreviewTab::Hex => true,
    };
    let tabs: Vec<(PreviewTab, Text)> = TABS
        .into_iter()
        .filter(|&(candidate, _)| offered(candidate))
        .collect();
    if !offered(tab) {
        tab = PreviewTab::Text;
    }
    let labels: Vec<String> = tabs.iter().map(|&(_, text)| texts.get(text)).collect();
    let current = tabs
        .iter()
        .position(|(candidate, _)| *candidate == tab)
        .unwrap_or(0);
    let bar = ui.max_rect().with_max_y(ui.max_rect().top() + TOUCH_TARGET);
    if let Some(index) = tab::tab_bar(ui, &labels, current)
        && let Some((candidate, _)) = tabs.get(index)
    {
        tab = *candidate;
    }
    let contents = browser
        .preview()
        .and_then(|preview| preview.contents())
        .filter(|_| matches!(tab, PreviewTab::Text | PreviewTab::Hex));
    let document = xml
        .and_then(XmlPreview::text)
        .filter(|_| tab == PreviewTab::Xml);
    let copy_label = texts.get(Text::PreviewCopy);
    let mut copy = None;
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(bar.shrink2(vec2(SPACE_2, 0.0))),
        |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let enabled = contents.is_some() || document.is_some();
                if button::button(ui, Kind::Ghost, Size::Compact, &copy_label, enabled).clicked() {
                    copy = contents.map(|contents| contents.copied(tab)).or_else(|| {
                        document
                            .map(|document| String::from_utf8_lossy(document.bytes()).into_owned())
                    });
                }
            });
        },
    );
    (tab, copy)
}

/// Shows the body of `tab`, the Audio tab at `volume`; returns what the Image or Audio tab asks
/// for.
fn tab_body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tab: PreviewTab,
    volume: f32,
) -> Option<Asked> {
    match browser.records().filter(|_| tab == PreviewTab::DataForge) {
        Some(tree) => {
            let size = browser
                .archive_index()
                .dataforge_entry()
                .map_or(0, Entry::uncompressed_size);
            path_line(ui, tree.entry_path(), &format::size(size, texts.language()));
            dataforge_summary::show(ui, texts, tree);
            None
        }
        None => body(ui, texts, browser, tab, volume),
    }
}

fn body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tab: PreviewTab,
    volume: f32,
) -> Option<Asked> {
    let Some(preview) = browser.preview() else {
        frames::preview_body().show(ui, |ui| {
            ui.label(
                plain(texts.get(Text::PreviewNone), FS_XS, Weight::Regular)
                    .color(palette::GRAY_2_TEXT),
            );
        });
        return None;
    };
    let entry = browser.index().entry(preview.entry());
    let path = entry.map_or("", Entry::path);
    let size = entry.map_or(0, Entry::uncompressed_size);
    path_line(ui, path, &format::size(size, texts.language()));
    if let Some(image) = preview.image().filter(|_| tab == PreviewTab::Image) {
        return image_preview::show(ui, texts, path, image).map(Asked::Mip);
    }
    if let Some(audio) = preview.audio().filter(|_| tab == PreviewTab::Audio) {
        return audio_preview::show(ui, texts, (path, size), audio, volume).map(Asked::Audio);
    }
    if let Some(xml) = preview.xml().filter(|_| tab == PreviewTab::Xml) {
        xml_preview::show(ui, texts, (path, size), xml);
        return None;
    }
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    frames::preview_body().show(ui, |ui| match preview.status() {
        PreviewStatus::Loading => {
            ui.horizontal(|ui| {
                crate::widgets::indicator::spinner(ui, crate::theme::desktop::SPINNER_INLINE);
                ui.label(texts.format(Text::PreviewLoading, &[path_arg()]));
            });
        }
        PreviewStatus::TooLarge => {
            ui.label(texts.format(
                Text::PreviewTooLarge,
                &[
                    path_arg(),
                    ("size", format::size(size, language).into()),
                    ("limit", format::size(PREVIEW_SIZE_LIMIT, language).into()),
                ],
            ));
        }
        PreviewStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.format(Text::PreviewStopped, &[path_arg()]),
                |reason| {
                    texts.format(
                        Text::PreviewFailed,
                        &[path_arg(), ("reason", reason.to_owned().into())],
                    )
                },
            );
            ui.colored_label(palette::DANGER_TEXT, text);
        }
        PreviewStatus::ArchiveChanged => {
            ui.colored_label(palette::WARNING, texts.get(Text::PreviewArchiveChanged));
        }
        PreviewStatus::Ready(contents) => ready(ui, texts, contents, tab),
    });
    None
}

fn path_line(ui: &mut Ui, path: &str, size: &str) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), PREVIEW_PATH_HEIGHT),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, path));
    let inner = rect.shrink2(vec2(PREVIEW_PADDING_X, 0.0));
    let size = line(ui, plain(size, FS_XS, Weight::Regular), f32::INFINITY);
    let galley = line(
        ui,
        plain(path, FS_XS, Weight::Regular),
        inner.width() - size.size().x - SPACE_2,
    );
    let size_rect = inner.with_min_x(inner.left() + galley.size().x + SPACE_2);
    paint_left(ui, inner, galley, palette::GRAY_1);
    paint_left(ui, size_rect, size, palette::GRAY_2_TEXT);
    hairline_below(ui, rect);
}

fn ready(ui: &mut Ui, texts: &Texts, contents: &PreviewContents, tab: PreviewTab) {
    if contents.is_truncated() {
        let language = texts.language();
        let shown = u64::try_from(contents.bytes().len()).unwrap_or(u64::MAX);
        let note = texts.format(
            Text::PreviewTruncated,
            &[
                ("shown", format::number(shown, language).into()),
                ("total", format::number(contents.size(), language).into()),
            ],
        );
        ui.colored_label(palette::WARNING, note);
    }
    match tab {
        PreviewTab::Hex => hex_view::hex_rows(ui, contents),
        _ => text_rows(ui, contents),
    }
}

fn text_rows(ui: &mut Ui, contents: &PreviewContents) {
    ScrollArea::both()
        .id_salt("preview-text")
        .auto_shrink(false)
        .show_rows(ui, HEX_ROW_HEIGHT, contents.line_count(), |ui, rows| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for index in rows {
                ui.add(Label::new(contents.line(index)).extend());
            }
        });
}
