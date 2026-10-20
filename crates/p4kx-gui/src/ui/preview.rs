use egui::{Label, Role, ScrollArea, Sense, Ui, WidgetInfo, vec2};
use fluent_bundle::FluentValue;
use p4kx_extraction::{Entry, Texture};

use super::audio_preview;
use super::preview_tab::{Asked, Typed, typed_tab};
use super::record_preview;
use super::soundbank_preview;
use super::{dataforge_summary, hex_view, image_preview, preview_bar, record_incoming};
use crate::theme::desktop::{HEX_ROW_HEIGHT, PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, Weight};
use crate::widgets::frames;
use crate::widgets::scroll_region;
use crate::widgets::text::{hairline_below, line, paint_left, plain};
use crate::{
    Browser, Model, PREVIEW_SIZE_LIMIT, Preview, PreviewContents, PreviewStatus, PreviewTab,
    ScrollText, Text, Texts, format, is_bank_path, is_wem_path,
};

/// Shows the Image tab after selecting a texture header, the Audio tab after selecting a WEM or a
/// soundbank, and leaves the XML, Image, Audio, DataForge, Record and Referenced-by tabs for the
/// Text tab after selecting another Entry; the XML tab opens once the Entry turns out to be
/// `CryXml`.
pub(super) fn select(model: &mut Model, id: u32) {
    let path = model
        .browser()
        .and_then(|browser| browser.index().entry(id))
        .map(Entry::path);
    if path.is_some_and(Texture::is_header_path) {
        model.set_preview_tab(PreviewTab::Image);
    } else if path.is_some_and(|path| is_wem_path(path) || is_bank_path(path)) {
        model.set_preview_tab(PreviewTab::Audio);
    } else if matches!(
        model.preview_tab(),
        PreviewTab::Xml
            | PreviewTab::Image
            | PreviewTab::Audio
            | PreviewTab::DataForge
            | PreviewTab::Record
            | PreviewTab::Incoming
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
    let incoming = model.preview_tab() == PreviewTab::Incoming;
    if let Some(browser) = model.browser_mut() {
        let ctx = ui.ctx().clone();
        browser.load_record_preview({
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        });
        if incoming {
            browser.load_incoming(move || ctx.request_repaint());
        }
    }
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let bar = preview_bar::tab_bar(ui, texts, browser, model.preview_tab());
    let tab = bar.tab;
    let asked = tab_body(ui, texts, browser, tab, model.audio_volume());
    if let Some(text) = bar.copy {
        ui.ctx().copy_text(text);
    }
    model.set_preview_tab(tab);
    match bar.step.map(Asked::Record).or(asked) {
        Some(Asked::Mip(level)) => {
            if let Some(image) = model.browser_mut().and_then(Browser::image_mut) {
                image.request(level, image_preview::max_side(ui));
            }
        }
        Some(Asked::Audio(action)) => audio_preview::apply(model, action),
        Some(Asked::Sound(id)) => {
            let ctx = ui.ctx().clone();
            if let Some(browser) = model.browser_mut() {
                browser.select_sound(id, move || ctx.request_repaint());
            }
        }
        Some(Asked::Record(step)) => record_preview::apply(model, &step),
        None => {}
    }
    if tab == PreviewTab::DataForge
        && let Some(browser) = model.browser_mut()
    {
        let ctx = ui.ctx().clone();
        browser.load_records(move || ctx.request_repaint());
    }
}

/// Shows the body of `tab`, the Audio tab at `volume`; returns what the Image, Audio or Record
/// tab asks for.
fn tab_body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tab: PreviewTab,
    volume: f32,
) -> Option<Asked> {
    let record = browser.selected_record().is_some();
    match (tab, browser.records()) {
        (PreviewTab::DataForge, Some(tree)) => {
            let size = browser
                .archive_index()
                .dataforge_entry()
                .map_or(0, Entry::uncompressed_size);
            path_line(ui, tree.entry_path(), &format::size(size, texts.language()));
            dataforge_summary::show(ui, texts, tree);
            None
        }
        (PreviewTab::Record, Some(_)) if record => {
            record_preview::show(ui, texts, browser).map(Asked::Record)
        }
        (PreviewTab::Incoming, Some(_)) if record => {
            record_incoming::show(ui, texts, browser).map(Asked::Record)
        }
        (PreviewTab::Xml, Some(_)) if record => {
            record_preview::show_xml(ui, texts, browser);
            None
        }
        _ => body(ui, texts, browser, tab, volume),
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
    let size_line = preview
        .soundbank()
        .and_then(|bank| soundbank_preview::summary(texts, bank, size))
        .unwrap_or_else(|| format::size(size, texts.language()));
    path_line(ui, path, &size_line);
    if let Typed::Shown(asked) = typed_tab(ui, texts, preview, (path, size), (tab, volume)) {
        return asked;
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
        status @ (PreviewStatus::ArchiveChanged
        | PreviewStatus::KeyRequired
        | PreviewStatus::KeyRejected) => note(ui, texts, status, path),
        PreviewStatus::Ready(contents) => ready(ui, texts, contents, tab),
    });
    None
}

/// Shows the note that stands for the preview of an Entry that cannot be read until the Archive is
/// reloaded or has the right key.
fn note(ui: &mut Ui, texts: &Texts, status: PreviewStatus<'_>, path: &str) {
    let path = [("path", FluentValue::from(path.to_owned()))];
    let (colour, text) = match status {
        PreviewStatus::KeyRequired => (
            palette::WARNING,
            texts.format(Text::PreviewKeyRequired, &path),
        ),
        PreviewStatus::KeyRejected => (
            palette::DANGER_TEXT,
            texts.format(Text::PreviewKeyRejected, &path),
        ),
        _ => (palette::WARNING, texts.get(Text::PreviewArchiveChanged)),
    };
    ui.colored_label(colour, text);
}

/// Shows the path line of the preview: `path`, then `size` muted.
pub(super) fn path_line(ui: &mut Ui, path: &str, size: &str) {
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
        PreviewTab::Hex => hex_view::hex_rows(ui, texts, contents),
        _ => text_rows(ui, texts, contents),
    }
}

fn text_rows(ui: &mut Ui, texts: &Texts, contents: &PreviewContents) {
    let area = ScrollArea::both()
        .id_salt("preview-text")
        .auto_shrink(false);
    let id = ui.scope_id().with("preview-text");
    scroll_region::show(
        ui,
        (id, || texts.get(ScrollText::Text)),
        HEX_ROW_HEIGHT + ui.spacing().item_spacing.y,
        area,
        |area, ui| {
            area.show_rows(ui, HEX_ROW_HEIGHT, contents.line_count(), |ui, rows| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in rows {
                    ui.add(Label::new(contents.line(index)).extend());
                }
            })
        },
    );
}
