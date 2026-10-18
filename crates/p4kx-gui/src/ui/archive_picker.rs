use std::path::{Path, PathBuf};

use egui::{
    CornerRadius, Popup, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2,
};
use p4kx_channels::Channel;

use super::menu::{dropdown, entry};
use super::search::eyebrow;
use super::status::build_label;
use crate::theme::desktop::{
    BUTTON_HEIGHT, CHANNEL_GAP, CHEVRON, FOCUS_OFFSET, PICKER_MIN_WIDTH, PICKER_PADDING_X,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, TRACKING_DISPLAY, Weight};
use crate::widgets::button::focus_ring;
use crate::widgets::rows::chevron;
use crate::widgets::text::{caps, line, paint_left, paint_right, plain};
use crate::{ComparisonText, Model, Text, Texts};

/// What the user did with a picker.
pub(super) enum Picked {
    /// Picked this Archive from the list.
    Archive(PathBuf),
    /// Asked for a file dialog.
    ChooseFile,
}

/// Shows the labelled picker of one side, `width` wide, showing `current`; offers the detected
/// Channels, the Recent Archives and a file dialog.
pub(super) fn show(
    ui: &mut Ui,
    model: &Model,
    label: &str,
    current: Option<&Path>,
    width: f32,
) -> Option<Picked> {
    let texts = model.texts();
    let channels = model.channels().detected();
    let width = width.max(PICKER_MIN_WIDTH);
    let mut picked = None;
    ui.vertical(|ui| {
        ui.set_width(width);
        let caption = ui.label(eyebrow(label));
        let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON_HEIGHT), Sense::click());
        let response = response.labelled_by(caption.id);
        let display = describe(texts, channels, current);
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, true, label);
            info.current_text_value = Some(display.text());
            info
        });
        let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
        paint_field(ui, rect, &display, response.hovered() || open);
        if response.has_focus() {
            focus_ring(ui, rect, palette::PRIMARY, FOCUS_OFFSET);
        }
        Popup::menu(&response).width(width).show(|ui| {
            dropdown(ui);
            ui.set_min_width(width);
            picked = list(ui, texts, model, channels);
        });
    });
    picked
}

/// What a picker shows: the hint before a pick, a Channel with its Build, or another file.
enum Display {
    Hint(String),
    Channel(String, String),
    File(String),
}

impl Display {
    fn text(&self) -> String {
        match self {
            Self::Hint(text) | Self::File(text) => text.clone(),
            Self::Channel(name, build) => format!("{name} {build}"),
        }
    }
}

fn describe(texts: &Texts, channels: &[Channel], current: Option<&Path>) -> Display {
    let Some(path) = current else {
        return Display::Hint(texts.get(ComparisonText::PickArchive));
    };
    channels
        .iter()
        .find(|channel| channel.archive() == path)
        .map_or_else(
            || Display::File(path.display().to_string()),
            |channel| {
                Display::Channel(
                    channel.name().to_owned(),
                    build_label(texts, channel.build()),
                )
            },
        )
}

fn paint_field(ui: &Ui, rect: Rect, display: &Display, emphasised: bool) {
    let border = if emphasised {
        palette::PRIMARY
    } else {
        palette::CONTROL_BORDER
    };
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        palette::CONTROL_BG,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(vec2(PICKER_PADDING_X, 0.0));
    let text_area = inner.with_max_x(inner.right() - CHEVRON - CHANNEL_GAP);
    let centre = pos2(inner.right() - CHEVRON / 2.0, inner.center().y);
    chevron(ui.painter(), centre, CHEVRON, true, palette::PRIMARY);
    let (name, detail) = match display {
        Display::Hint(text) | Display::File(text) => {
            let colour = if matches!(display, Display::Hint(_)) {
                palette::GRAY_2_TEXT
            } else {
                palette::GRAY_1
            };
            let galley = line(
                ui,
                plain(text.as_str(), FS_XS, Weight::Regular),
                text_area.width(),
            );
            paint_left(ui, text_area, galley, colour);
            return;
        }
        Display::Channel(name, detail) => (name, detail),
    };
    let name = line(
        ui,
        caps(name, FS_XS, Weight::Bold, TRACKING_DISPLAY),
        text_area.width(),
    );
    let name_width = name.size().x;
    paint_left(ui, text_area, name, palette::GRAY_1);
    let detail_area = text_area.with_min_x(text_area.left() + name_width + CHANNEL_GAP);
    let galley = line(
        ui,
        plain(detail.as_str(), FS_XS, Weight::Regular),
        detail_area.width(),
    );
    paint_right(ui, detail_area, galley, palette::GRAY_2_TEXT);
}

fn list(ui: &mut Ui, texts: &Texts, model: &Model, channels: &[Channel]) -> Option<Picked> {
    let mut picked = None;
    let heading = |ui: &mut Ui, text: Text| {
        ui.horizontal(|ui| {
            ui.add_space(PICKER_PADDING_X);
            ui.label(eyebrow(&texts.get(text)));
        });
    };
    heading(ui, Text::ChannelsHeading);
    if channels.is_empty() {
        note(ui, &texts.get(Text::ChannelsNone));
    }
    for channel in channels {
        let text = texts.format(
            ComparisonText::Side,
            &[
                ("channel", channel.name().to_owned().into()),
                ("build", build_label(texts, channel.build()).into()),
            ],
        );
        if ui.button(entry(&text)).clicked() {
            picked = Some(Picked::Archive(channel.archive().to_path_buf()));
        }
    }
    ui.separator();
    heading(ui, Text::MenuRecent);
    if model.recent().is_empty() {
        note(ui, &texts.get(Text::RecentNone));
    }
    for path in model.recent() {
        if ui.button(entry(&path.display().to_string())).clicked() {
            picked = Some(Picked::Archive(path.clone()));
        }
    }
    ui.separator();
    if ui
        .button(entry(&texts.get(ComparisonText::ChooseFile)))
        .clicked()
    {
        picked = Some(Picked::ChooseFile);
    }
    picked
}

fn note(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(PICKER_PADDING_X - ui.spacing().item_spacing.x);
        ui.label(entry(text).color(palette::GRAY_2_TEXT));
    });
}
