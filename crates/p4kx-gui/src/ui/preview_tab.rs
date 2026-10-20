use egui::Ui;

use super::audio_preview::{self, AudioAction, Named};
use super::record_preview::RecordStep;
use super::soundbank_preview::{self, BankAction};
use super::{image_preview, model_preview, xml_preview};
use crate::{Camera, Preview, PreviewTab, Texts, ViewerSupport};

/// What the body of a tab asks for.
pub(super) enum Asked {
    Mip(u32),
    Audio(AudioAction),
    Sound(u32),
    Record(RecordStep),
    Camera(Camera),
}

/// Whether a tab of its own showed the Entry.
pub(super) enum Typed {
    Absent,
    Shown(Option<Asked>),
}

/// Shows the Image, Audio, Model or XML tab of `preview` at `path` of `size` bytes when `tab` is one of
/// them and the Entry has it; returns [`Typed::Absent`] when it has not, and else what the tab
/// asks for.
pub(super) fn typed_tab(
    ui: &mut Ui,
    texts: &Texts,
    preview: &Preview,
    (path, size): (&str, u64),
    (tab, volume, support): (PreviewTab, f32, &ViewerSupport),
) -> Typed {
    if let Some(image) = preview.image().filter(|_| tab == PreviewTab::Image) {
        return Typed::Shown(image_preview::show(ui, texts, path, image).map(Asked::Mip));
    }
    if let Some(bank) = preview.soundbank().filter(|_| tab == PreviewTab::Audio) {
        let action = soundbank_preview::show(ui, texts, (path, size), bank, volume);
        return Typed::Shown(action.map(|action| match action {
            BankAction::Select(id) => Asked::Sound(id),
            BankAction::Audio(action) => Asked::Audio(action),
        }));
    }
    if let Some(audio) = preview.audio().filter(|_| tab == PreviewTab::Audio) {
        let named = Named {
            path,
            size,
            embedded: false,
        };
        return Typed::Shown(
            audio_preview::show(ui, texts, named, audio, volume).map(Asked::Audio),
        );
    }
    if let Some(model) = preview.model().filter(|_| tab == PreviewTab::Model) {
        return Typed::Shown(
            model_preview::show(ui, texts, path, (model, support)).map(Asked::Camera),
        );
    }
    if let Some(xml) = preview.xml().filter(|_| tab == PreviewTab::Xml) {
        xml_preview::show(ui, texts, (path, size), xml);
        return Typed::Shown(None);
    }
    Typed::Absent
}
