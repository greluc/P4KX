use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use egui::{Color32, Galley, Id, Response, Role, Sense, TextStyle, TextWrapMode, Ui, WidgetInfo};
use fluent_bundle::FluentValue;
use p4kx_search::Source;

use super::search::eyebrow;
use crate::theme::desktop::{PREVIEW_PADDING_X, TOOLBAR_PADDING};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::button::{self, Size};
use crate::widgets::frames::alert_text_width;
use crate::widgets::tab::SegmentLabel;
use crate::widgets::text::{line, plain};
use crate::{
    ContentResult, ContentSearchStatus, ContentSearchTab, ContentSearchText, Texts, format,
};

pub(super) const SOURCES: [(Source, ContentSearchText); 3] = [
    (Source::Records, ContentSearchText::SourceRecords),
    (Source::Localization, ContentSearchText::SourceLocalization),
    (Source::Entries, ContentSearchText::SourceEntries),
];

const PANEL_BORDER: f32 = 2.0;

/// A button label with its accessible name.
pub(super) type ButtonText = (Arc<str>, Arc<Galley>);

/// What the body of the tab shows, laid out.
pub(super) enum Body {
    Message(Arc<Galley>),
    Searching(Arc<Galley>),
    Danger(Arc<Galley>),
    Warning(Arc<Galley>),
    Finished(Box<Finished>),
}

/// The laid-out texts of a finished search.
pub(super) struct Finished {
    pub(super) hits: Arc<Galley>,
    pub(super) notes: Vec<Arc<Galley>>,
    pub(super) key: Option<(Arc<Galley>, ButtonText)>,
    pub(super) segments: Option<[SegmentLabel; 3]>,
    pub(super) message: Option<Arc<Galley>>,
}

/// The laid-out texts of the Search tab, made again only when the language, the search, the
/// shown source, the width or the scale change.
pub(super) struct Notes {
    key: u64,
    pub(super) label: (Arc<str>, Arc<Galley>),
    pub(super) sources_label: Arc<Galley>,
    pub(super) hint: Arc<str>,
    pub(super) run: ButtonText,
    pub(super) cancel: ButtonText,
    pub(super) names: [Arc<str>; 3],
    pub(super) body: Body,
}

impl Notes {
    /// Returns the texts for `tab`, laid out again when anything they show changed.
    pub(super) fn get(ui: &Ui, texts: &Texts, tab: &ContentSearchTab, width: f32) -> Arc<Self> {
        let mut hasher = DefaultHasher::new();
        texts.language().hash(&mut hasher);
        tab.serial().hash(&mut hasher);
        width.to_bits().hash(&mut hasher);
        ui.ctx().pixels_per_point().to_bits().hash(&mut hasher);
        let key = hasher.finish();
        let id = Id::unique("p4kx-content-search-notes");
        if let Some(notes) = ui.data(|data| data.get_temp::<Arc<Self>>(id))
            && notes.key == key
        {
            return notes;
        }
        let notes = Arc::new(make(ui, texts, tab, key, width));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&notes)));
        notes
    }
}

fn make(ui: &Ui, texts: &Texts, tab: &ContentSearchTab, key: u64, width: f32) -> Notes {
    let label = texts.get(ContentSearchText::Label);
    let label_galley = line(ui, eyebrow(&label), f32::INFINITY);
    let sources = texts.get(ContentSearchText::SourcesLabel);
    let button = |text: ContentSearchText| button_label(ui, texts, text, Size::Normal);
    Notes {
        key,
        label: (label.into(), label_galley),
        sources_label: line(ui, eyebrow(&sources), f32::INFINITY),
        hint: texts.get(ContentSearchText::Hint).into(),
        run: button(ContentSearchText::Run),
        cancel: button(ContentSearchText::Cancel),
        names: SOURCES.map(|(_, text)| texts.get(text).into()),
        body: body(ui, texts, tab, width),
    }
}

fn button_label(ui: &Ui, texts: &Texts, text: ContentSearchText, size: Size) -> ButtonText {
    let label = texts.get(text);
    let galley = button::laid_label(ui, size, &label);
    (label.into(), galley)
}

fn body(ui: &Ui, texts: &Texts, tab: &ContentSearchTab, width: f32) -> Body {
    let message_width = PREVIEW_PADDING_X.mul_add(-2.0, width - PANEL_BORDER);
    let alert_width = alert_text_width(TOOLBAR_PADDING.mul_add(-2.0, width - PANEL_BORDER));
    let message =
        |text: String| Body::Message(wrapped(ui, text, palette::GRAY_2_TEXT, message_width));
    match tab.status() {
        ContentSearchStatus::Idle => message(texts.get(ContentSearchText::Prompt)),
        ContentSearchStatus::Running => Body::Searching(line(
            ui,
            plain(
                texts.get(ContentSearchText::Searching),
                FS_SM,
                Weight::Regular,
            )
            .color(palette::GRAY_1),
            f32::INFINITY,
        )),
        ContentSearchStatus::Cancelled => message(texts.get(ContentSearchText::Cancelled)),
        ContentSearchStatus::Failed(reason) => {
            let text = reason.map_or_else(
                || texts.get(ContentSearchText::Stopped),
                |reason| {
                    texts.format(
                        ContentSearchText::Failed,
                        &[("reason", reason.to_owned().into())],
                    )
                },
            );
            Body::Danger(wrapped(ui, text, palette::GRAY_1, alert_width))
        }
        ContentSearchStatus::ArchiveChanged => Body::Warning(wrapped(
            ui,
            texts.get(ContentSearchText::ArchiveChanged),
            palette::GRAY_1,
            alert_width,
        )),
        ContentSearchStatus::Finished(result) => Body::Finished(Box::new(finished(
            ui,
            (texts, tab, result),
            (message_width, alert_width),
        ))),
    }
}

fn finished(
    ui: &Ui,
    (texts, tab, result): (&Texts, &ContentSearchTab, &ContentResult),
    (message_width, alert_width): (f32, f32),
) -> Finished {
    let number = |value: u64| format::number(value, texts.language());
    let hits = [Source::Records, Source::Localization, Source::Entries]
        .into_iter()
        .map(|source| u64::try_from(result.len(source)).unwrap_or(u64::MAX))
        .fold(0_u64, u64::saturating_add);
    let hits = texts.format(
        ContentSearchText::Hits,
        &[
            ("count", FluentValue::from(hits)),
            ("number", number(hits).into()),
            (
                "seconds",
                format!("{:.2}", result.elapsed().as_secs_f64()).into(),
            ),
        ],
    );
    let hits = line(
        ui,
        plain(hits, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
        f32::INFINITY,
    );
    let notes = notes(texts, result)
        .into_iter()
        .map(|text| wrapped(ui, text, palette::GRAY_1, alert_width))
        .collect();
    let stats = result.stats();
    let key = (stats.entries_key_required > 0).then(|| {
        let text = texts.format(
            ContentSearchText::KeyRequired,
            &[
                ("count", FluentValue::from(stats.entries_key_required)),
                ("number", number(stats.entries_key_required).into()),
            ],
        );
        (
            wrapped(ui, text, palette::GRAY_1, alert_width),
            button_label(ui, texts, ContentSearchText::KeyButton, Size::Compact),
        )
    });
    let (segments, message) = if result.is_empty() {
        let text = texts.format(
            ContentSearchText::NoMatch,
            &[("text", result.text().to_owned().into())],
        );
        (None, Some(text))
    } else {
        let none = (result.len(tab.shown()) == 0).then(|| texts.get(ContentSearchText::NoneHere));
        (Some(segments(ui, texts, result)), none)
    };
    Finished {
        hits,
        notes,
        key,
        segments,
        message: message.map(|text| wrapped(ui, text, palette::GRAY_2_TEXT, message_width)),
    }
}

fn segments(ui: &Ui, texts: &Texts, result: &ContentResult) -> [SegmentLabel; 3] {
    SOURCES.map(|(source, name)| {
        let count = u64::try_from(result.len(source)).unwrap_or(u64::MAX);
        let text = if result.has_more(source) {
            ContentSearchText::SegmentMore
        } else {
            ContentSearchText::Segment
        };
        let count = texts.format(
            text,
            &[("number", format::number(count, texts.language()).into())],
        );
        SegmentLabel::new(ui, &texts.get(name), Some(&count))
    })
}

fn notes(texts: &Texts, result: &ContentResult) -> Vec<String> {
    let stats = result.stats();
    let number = |value: u64| format::number(value, texts.language());
    let mut lines = Vec::new();
    if Source::ALL
        .into_iter()
        .any(|source| result.has_more(source))
    {
        let more = u64::try_from(p4kx_search::DEFAULT_LIMIT).unwrap_or(u64::MAX);
        lines.push(texts.format(ContentSearchText::More, &[("number", number(more).into())]));
    }
    if result
        .unavailable()
        .iter()
        .any(|missing| missing.source == Source::Records)
    {
        lines.push(texts.get(ContentSearchText::NoDataForge));
    }
    let skipped = stats
        .entries_skipped()
        .saturating_sub(stats.entries_key_required);
    if skipped > 0 {
        lines.push(texts.format(
            ContentSearchText::Skipped,
            &[
                ("count", FluentValue::from(skipped)),
                ("number", number(skipped).into()),
                ("binary", number(stats.entries_binary).into()),
                ("large", number(stats.entries_too_large).into()),
                ("unreadable", number(stats.entries_unreadable).into()),
            ],
        ));
    }
    lines
}

fn wrapped(ui: &Ui, text: String, colour: Color32, width: f32) -> Arc<Galley> {
    egui::WidgetText::from(plain(text, FS_SM, Weight::Regular).color(colour)).into_galley(
        ui,
        Some(TextWrapMode::Wrap),
        width,
        TextStyle::Body,
    )
}

/// Shows an eyebrow label named by its Fluent text.
pub(super) fn labelled(ui: &mut Ui, (name, galley): &(Arc<str>, Arc<Galley>)) -> Response {
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &**name));
    ui.painter()
        .galley(rect.min, Arc::clone(galley), palette::GRAY_2_TEXT);
    response
}
