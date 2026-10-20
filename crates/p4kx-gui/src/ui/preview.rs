use egui::{Align, Layout, Role, Sense, Ui, WidgetInfo, vec2};
use p4kx_extraction::{Entry, Texture};

use super::audio_preview::{self, AudioAction};
use super::entry_status::status_body;
use super::record_preview::{self, RecordStep};
use super::{dataforge_summary, image_preview, model_preview, xml_preview};
use crate::theme::desktop::{PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, TOUCH_TARGET, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::tab;
use crate::widgets::text::{hairline_below, line, paint_left, plain};
use crate::{
    AudioText, Browser, Camera, DataForgeText, ImageText, Model, ModelText, Preview, PreviewTab,
    RecordText, Text, Texts, ViewerSupport, XmlDocument, XmlPreview, XmlText, format,
    is_model_path, is_wem_path,
};

const TABS: [(PreviewTab, Text); 8] = [
    (PreviewTab::Text, Text::PreviewTabText),
    (PreviewTab::Hex, Text::PreviewTabHex),
    (PreviewTab::Xml, Text::Xml(XmlText::Tab)),
    (PreviewTab::Image, Text::Image(ImageText::Tab)),
    (PreviewTab::Audio, Text::Audio(AudioText::Tab)),
    (PreviewTab::Model, Text::Model(ModelText::Tab)),
    (PreviewTab::DataForge, Text::DataForge(DataForgeText::Tab)),
    (PreviewTab::Record, Text::Record(RecordText::Tab)),
];

/// What the body of a tab asks for.
enum Asked {
    Mip(u32),
    Audio(AudioAction),
    Record(RecordStep),
    Camera(Camera),
}

/// Shows the Image tab after selecting a texture header, the Audio tab after selecting a WEM,
/// the Model tab after selecting a model, and leaves the XML, Image, Audio, Model, DataForge and
/// Record tabs for the Text tab after selecting another Entry; the XML tab opens once the Entry
/// turns out to be `CryXml`.
pub(super) fn select(model: &mut Model, id: u32) {
    let path = model
        .browser()
        .and_then(|browser| browser.index().entry(id))
        .map(Entry::path);
    if path.is_some_and(Texture::is_header_path) {
        model.set_preview_tab(PreviewTab::Image);
    } else if path.is_some_and(is_wem_path) {
        model.set_preview_tab(PreviewTab::Audio);
    } else if path.is_some_and(is_model_path) {
        model.set_preview_tab(PreviewTab::Model);
    } else if matches!(
        model.preview_tab(),
        PreviewTab::Xml
            | PreviewTab::Image
            | PreviewTab::Audio
            | PreviewTab::Model
            | PreviewTab::DataForge
            | PreviewTab::Record
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
    if let Some(browser) = model.browser_mut() {
        let ctx = ui.ctx().clone();
        browser.load_record_preview(move || ctx.request_repaint());
    }
    let Some(browser) = model.browser() else {
        return;
    };
    let texts = model.texts();
    let bar = tab_bar(ui, texts, browser, model.preview_tab());
    let tab = bar.tab;
    let asked = tab_body(
        ui,
        texts,
        browser,
        tab,
        (model.audio_volume(), model.viewer_support()),
    );
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
        Some(Asked::Camera(camera)) => {
            if let Some(slot) = model.viewer_camera_mut() {
                *slot = camera;
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

/// What the tab bar asks for: the tab to show, the text to copy and a step through the Records.
struct Bar {
    tab: PreviewTab,
    copy: Option<String>,
    step: Option<RecordStep>,
}

/// Shows the tabs `browser` offers with back and forward through the Records and the copy
/// action; the tab to show is `tab` unless another was clicked or it is not offered.
fn tab_bar(ui: &mut Ui, texts: &Texts, browser: &Browser, mut tab: PreviewTab) -> Bar {
    let image = browser.preview().and_then(Preview::image);
    let audio = browser.preview().and_then(Preview::audio);
    let modelled = browser.preview().and_then(Preview::model).is_some();
    let record = browser.selected_record().is_some();
    let xml = browser.preview().and_then(Preview::xml).filter(|_| !record);
    let records = browser.records();
    let offered = |candidate: PreviewTab| match candidate {
        PreviewTab::Xml => xml.is_some() || record,
        PreviewTab::Image => image.is_some(),
        PreviewTab::Audio => audio.is_some(),
        PreviewTab::Model => modelled,
        PreviewTab::DataForge => records.is_some(),
        PreviewTab::Record => record,
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
    let document = shown_xml(browser, xml).filter(|_| tab == PreviewTab::Xml);
    let copy_label = texts.get(Text::PreviewCopy);
    let mut copy = None;
    let mut step = None;
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(bar.shrink2(vec2(SPACE_2, 0.0))),
        |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let enabled = contents.is_some() || document.is_some();
                if button::button(ui, Kind::Ghost, Size::Compact, &copy_label, enabled).clicked() {
                    copy = contents.map(|contents| contents.copied(tab)).or_else(|| {
                        document.map(|document| String::from_utf8_lossy(document).into_owned())
                    });
                }
                if browser.shows_records() {
                    step = steps(ui, texts, browser);
                }
            });
        },
    );
    Bar { tab, copy, step }
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

/// Shows the body of `tab`, the Audio tab at `volume` and the Model tab on `support`; returns what
/// the Image, Audio, Model or Record tab asks for.
fn tab_body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tab: PreviewTab,
    (volume, support): (f32, &ViewerSupport),
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
        (PreviewTab::Xml, Some(_)) if record => {
            record_preview::show_xml(ui, texts, browser);
            None
        }
        _ => body(ui, texts, browser, tab, (volume, support)),
    }
}

fn body(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    tab: PreviewTab,
    (volume, support): (f32, &ViewerSupport),
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
    if let Some(model) = preview.model().filter(|_| tab == PreviewTab::Model) {
        return model_preview::show(ui, texts, path, (model, support)).map(Asked::Camera);
    }
    if let Some(xml) = preview.xml().filter(|_| tab == PreviewTab::Xml) {
        xml_preview::show(ui, texts, (path, size), xml);
        return None;
    }
    status_body(ui, texts, preview, (path, size), tab);
    None
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
