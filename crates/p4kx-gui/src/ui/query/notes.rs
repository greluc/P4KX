use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use egui::{Color32, Galley, Id, Response, Ui};
use fluent_bundle::FluentValue;

use super::{labelled, note, page_status, status_key, wrapped};
use crate::theme::palette;
use crate::theme::tokens::FS_SM;
use crate::ui::search::eyebrow;
use crate::widgets::pagination::PageNames;
use crate::widgets::text::line;
use crate::{
    QueryExportStatus, QueryResult, QueryStage, QueryStatus, QueryTab, QueryText, Texts, format,
};

/// The laid-out texts of the Query tab, made again only when the status, the result, the
/// export, the language or the width change.
pub(super) struct Notes {
    key: u64,
    label: (Arc<str>, Arc<Galley>),
    pub(super) hint: Arc<str>,
    pub(super) run: Arc<str>,
    pub(super) cancel: Arc<str>,
    pub(super) export_csv: Arc<str>,
    pub(super) export_json: Arc<str>,
    pub(super) status: Option<Arc<Galley>>,
    pub(super) fraction: Option<f32>,
    pub(super) alert: Option<Arc<Galley>>,
    pub(super) prompt: Option<Arc<Galley>>,
    pub(super) count: Option<Arc<Galley>>,
    pub(super) export: Option<Arc<Galley>>,
    pub(super) page_status: Option<Arc<Galley>>,
    page_names: [Arc<str>; 5],
}

impl Notes {
    /// Returns the texts for `query`, laid out again when anything they show changed.
    pub(super) fn get(ui: &Ui, texts: &Texts, query: &QueryTab) -> Arc<Self> {
        let mut hasher = DefaultHasher::new();
        texts.language().hash(&mut hasher);
        status_key(query, &mut hasher);
        ui.available_width().to_bits().hash(&mut hasher);
        ui.ctx().pixels_per_point().to_bits().hash(&mut hasher);
        let key = hasher.finish();
        let id = Id::unique("p4kx-query-notes");
        if let Some(notes) = ui.data(|data| data.get_temp::<Arc<Self>>(id))
            && notes.key == key
        {
            return notes;
        }
        let notes = Arc::new(make(ui, texts, query, key));
        ui.data_mut(|data| data.insert_temp(id, Arc::clone(&notes)));
        notes
    }

    /// Shows the label of the editor.
    pub(super) fn show_label(&self, ui: &mut Ui) -> Response {
        labelled(ui, &self.label)
    }

    /// Returns the accessible names of the pagination.
    pub(super) fn page_names(&self) -> PageNames<'_> {
        let [first, previous, next, last, status] = &self.page_names;
        PageNames {
            first,
            previous,
            next,
            last,
            status,
        }
    }
}

fn make(ui: &Ui, texts: &Texts, query: &QueryTab, key: u64) -> Notes {
    let shared = |text: QueryText| Arc::<str>::from(texts.get(text));
    let label = texts.get(QueryText::Label);
    let label_galley = line(ui, eyebrow(&label), f32::INFINITY);
    let width = ui.available_width();
    let (status, fraction) = status(texts, query.status())
        .map_or((None, None), |(text, colour, fraction)| {
            (Some(note(ui, text, colour)), fraction)
        });
    let alert = alert(ui, texts, query.status(), width);
    let result = query.result();
    let prompt = match (query.status(), result) {
        (QueryStatus::Idle, _) => Some(texts.get(QueryText::Prompt)),
        (_, Some(result)) if result.is_empty() => Some(texts.get(QueryText::NoRows)),
        _ => None,
    }
    .map(|text| wrapped(ui, text, FS_SM, palette::GRAY_2_TEXT, width));
    let page = result.map(|result| (result.page(), result.page_count()));
    Notes {
        key,
        label: (label.into(), label_galley),
        hint: shared(QueryText::Hint),
        run: shared(QueryText::Run),
        cancel: shared(QueryText::Cancel),
        export_csv: shared(QueryText::ExportCsv),
        export_json: shared(QueryText::ExportJson),
        status,
        fraction,
        alert,
        prompt,
        count: result.map(|result| note(ui, count(texts, result), palette::GRAY_2_TEXT)),
        export: query
            .export_status()
            .map(|export| export_note(texts, export))
            .map(|(text, colour)| note(ui, text, colour)),
        page_status: page.map(|(page, pages)| page_status(ui, page, pages)),
        page_names: page_names(texts, page),
    }
}

fn alert(ui: &Ui, texts: &Texts, status: QueryStatus<'_>, width: f32) -> Option<Arc<Galley>> {
    let QueryStatus::Invalid(invalid) = status else {
        return None;
    };
    let text = texts.format(
        QueryText::Invalid,
        &[
            ("line", invalid.line().into()),
            ("column", invalid.column().into()),
            ("reason", invalid.message().to_owned().into()),
        ],
    );
    Some(wrapped(ui, text, FS_SM, palette::GRAY_1, width))
}

fn page_names(texts: &Texts, page: Option<(usize, usize)>) -> [Arc<str>; 5] {
    let shared = |text: QueryText| Arc::<str>::from(texts.get(text));
    let status = page.map_or_else(String::new, |(page, pages)| {
        texts.format(
            QueryText::Page,
            &[("page", (page + 1).into()), ("pages", pages.into())],
        )
    });
    [
        shared(QueryText::PageFirst),
        shared(QueryText::PagePrevious),
        shared(QueryText::PageNext),
        shared(QueryText::PageLast),
        status.into(),
    ]
}

fn number(texts: &Texts, value: usize) -> FluentValue<'static> {
    format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language()).into()
}

fn status(texts: &Texts, status: QueryStatus<'_>) -> Option<(String, Color32, Option<f32>)> {
    Some(match status {
        QueryStatus::Running(QueryStage::Loading) => {
            (texts.get(QueryText::Loading), palette::GRAY_1, None)
        }
        QueryStatus::Running(QueryStage::Selecting) => {
            (texts.get(QueryText::Selecting), palette::GRAY_1, None)
        }
        QueryStatus::Running(QueryStage::Reading { done, total }) => {
            let percent = done
                .saturating_mul(100)
                .checked_div(total)
                .unwrap_or(100)
                .min(100);
            let text = texts.format(
                QueryText::Progress,
                &[
                    ("done", number(texts, done)),
                    ("total", number(texts, total)),
                    ("percent", percent.to_string().into()),
                ],
            );
            let fraction = f32::from(u8::try_from(percent).unwrap_or(100)) / 100.0;
            (text, palette::GRAY_1, Some(fraction))
        }
        QueryStatus::Cancelled => (texts.get(QueryText::Cancelled), palette::GRAY_2_TEXT, None),
        QueryStatus::Failed(Some(reason)) => (
            texts.format(QueryText::Failed, &[("reason", reason.to_owned().into())]),
            palette::DANGER_TEXT,
            None,
        ),
        QueryStatus::Failed(None) => (texts.get(QueryText::Stopped), palette::DANGER_TEXT, None),
        QueryStatus::ArchiveChanged => {
            (texts.get(QueryText::ArchiveChanged), palette::WARNING, None)
        }
        QueryStatus::Idle | QueryStatus::Invalid(_) | QueryStatus::Finished(_) => return None,
    })
}

fn count(texts: &Texts, result: &QueryResult) -> String {
    let rows = result.len();
    let count = (
        "count",
        FluentValue::from(u64::try_from(rows).unwrap_or(u64::MAX)),
    );
    let shown = ("number", number(texts, rows));
    if result.failed() == 0 {
        texts.format(QueryText::Rows, &[count, shown])
    } else {
        let failed = format::number(result.failed(), texts.language());
        texts.format(
            QueryText::RowsFailed,
            &[count, shown, ("failed", failed.into())],
        )
    }
}

fn export_note(texts: &Texts, export: &QueryExportStatus) -> (String, Color32) {
    let path = |path: &std::path::Path| ("path", path.display().to_string().into());
    match export {
        QueryExportStatus::Exporting(at) => (
            texts.format(QueryText::Exporting, &[path(at)]),
            palette::PRIMARY,
        ),
        QueryExportStatus::Exported {
            path: at, records, ..
        } => (
            texts.format(
                QueryText::Exported,
                &[
                    ("count", FluentValue::from(*records)),
                    ("number", format::number(*records, texts.language()).into()),
                    path(at),
                ],
            ),
            palette::GRAY_2_TEXT,
        ),
        QueryExportStatus::Failed(at, reason) => (
            texts.format(
                QueryText::ExportFailed,
                &[path(at), ("reason", reason.clone().into())],
            ),
            palette::DANGER_TEXT,
        ),
    }
}
