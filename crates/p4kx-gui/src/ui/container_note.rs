use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use egui::{Color32, Galley, Id, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, vec2};
use fluent_bundle::FluentValue;

use crate::theme::desktop::{ROW_FONT, ROW_HEIGHT, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, Weight};
use crate::widgets::indicator::spinner;
use crate::widgets::text::{line, paint_left, plain};
use crate::{
    ContainerFailure, ContainerId, ContainerStatus, ContainerText, Language, Texts, format,
};

const MAX_CACHED: usize = 4096;

type Note = Option<(Arc<Galley>, Color32)>;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Key {
    language: Language,
    pixels_per_point: f32,
}

#[derive(Default)]
struct Cache {
    key: Option<Key>,
    notes: HashMap<(ContainerId, u8, u32), Note>,
}

/// The laid-out notes of the container rows, kept across frames until the language or the scale
/// change.
#[derive(Clone)]
pub(super) struct Notes(Arc<Mutex<Cache>>);

impl Notes {
    pub(super) fn get(ui: &Ui, texts: &Texts) -> Self {
        let id = Id::unique("p4kx-container-notes");
        let notes = ui.data_mut(|data| {
            data.get_temp_mut_or_insert_with(id, || Self(Arc::new(Mutex::new(Cache::default()))))
                .clone()
        });
        let key = Key {
            language: texts.language(),
            pixels_per_point: ui.ctx().pixels_per_point(),
        };
        let mut cache = notes.0.lock().unwrap_or_else(PoisonError::into_inner);
        if cache.key != Some(key) || cache.notes.len() > MAX_CACHED {
            *cache = Cache {
                key: Some(key),
                ..Cache::default()
            };
        }
        drop(cache);
        notes
    }

    fn note(
        &self,
        ui: &Ui,
        texts: &Texts,
        (id, status): (ContainerId, ContainerStatus<'_>),
        width: f32,
    ) -> Note {
        let mut cache = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        cache
            .notes
            .entry((id, status_code(status), width.to_bits()))
            .or_insert_with(|| {
                let (text, colour) = note_text(texts, status)?;
                Some((
                    line(ui, plain(text, ROW_FONT, Weight::Regular), width),
                    colour,
                ))
            })
            .clone()
    }
}

const fn status_code(status: ContainerStatus<'_>) -> u8 {
    match status {
        ContainerStatus::Closed => 0,
        ContainerStatus::Opening => 1,
        ContainerStatus::Open => 2,
        ContainerStatus::Failed(ContainerFailure::Unreadable(_)) => 3,
        ContainerStatus::Failed(ContainerFailure::OverBudget) => 4,
        ContainerStatus::Failed(ContainerFailure::Stopped) => 5,
        ContainerStatus::ArchiveChanged => 6,
    }
}

fn note_text(texts: &Texts, status: ContainerStatus<'_>) -> Option<(String, Color32)> {
    let (text, colour) = match status {
        ContainerStatus::Closed | ContainerStatus::Open => return None,
        ContainerStatus::Opening => (ContainerText::Opening, palette::GRAY_2_TEXT),
        ContainerStatus::Failed(ContainerFailure::Unreadable(_)) => {
            (ContainerText::Unreadable, palette::DANGER_TEXT)
        }
        ContainerStatus::Failed(ContainerFailure::OverBudget) => {
            (ContainerText::OverBudget, palette::WARNING)
        }
        ContainerStatus::Failed(ContainerFailure::Stopped) => {
            (ContainerText::Stopped, palette::WARNING)
        }
        ContainerStatus::ArchiveChanged => (ContainerText::ArchiveChanged, palette::WARNING),
    };
    Some((texts.get(text), colour))
}

/// Returns the full message of the container called `name` that is not open, for its tooltip,
/// or `None` while nothing is wrong; `budget` is the memory the open containers may hold.
pub(super) fn detail(
    texts: &Texts,
    name: &str,
    status: ContainerStatus<'_>,
    budget: u64,
) -> Option<(String, Color32)> {
    let name_arg = || ("name", FluentValue::from(name.to_owned()));
    match status {
        ContainerStatus::Closed | ContainerStatus::Opening | ContainerStatus::Open => None,
        ContainerStatus::Failed(ContainerFailure::Unreadable(reason)) => Some((
            texts.format(
                ContainerText::UnreadableDetail,
                &[name_arg(), ("reason", reason.clone().into())],
            ),
            palette::DANGER_TEXT,
        )),
        ContainerStatus::Failed(ContainerFailure::OverBudget) => {
            let limit = format::size(budget, texts.language());
            Some((
                texts.format(
                    ContainerText::OverBudgetDetail,
                    &[name_arg(), ("limit", limit.into())],
                ),
                palette::WARNING,
            ))
        }
        ContainerStatus::Failed(ContainerFailure::Stopped) => Some((
            texts.format(ContainerText::StoppedDetail, &[name_arg()]),
            palette::WARNING,
        )),
        ContainerStatus::ArchiveChanged => Some((
            texts.format(ContainerText::ArchiveChangedDetail, &[name_arg()]),
            palette::WARNING,
        )),
    }
}

/// Shows the note of the container `id` in `rect`, after its name: a spinner while it opens,
/// and why it is not open when it failed.
pub(super) fn show(
    ui: &mut Ui,
    (texts, notes): (&Texts, &Notes),
    (id, status): (ContainerId, ContainerStatus<'_>),
    rect: Rect,
) {
    let opening = matches!(status, ContainerStatus::Opening);
    let text_rect = if opening {
        rect.with_min_x(rect.left() + SPINNER_INLINE + SPACE_2)
    } else {
        rect
    };
    let Some((galley, colour)) = notes.note(ui, texts, (id, status), text_rect.width().max(0.0))
    else {
        return;
    };
    if opening {
        let spot = Rect::from_min_size(rect.min, vec2(SPINNER_INLINE, rect.height()));
        ui.scope_builder(
            UiBuilder::new().max_rect(spot.shrink2(vec2(0.0, (ROW_HEIGHT - SPINNER_INLINE) / 2.0))),
            |ui| spinner(ui, SPINNER_INLINE),
        );
    }
    let response = ui.interact(
        text_rect,
        ui.scope_id().with(("container-note", id)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, galley.text()));
    paint_left(ui, text_rect, galley, colour);
}
