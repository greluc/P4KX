// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use egui::{Frame, Margin, Role, Sense, Ui, WidgetInfo, vec2};
use p4kx_channels::Channel;

use crate::theme::desktop::{
    CHANNEL_GAP, ROW_HEIGHT, SECTION_TITLE_PADDING_BOTTOM, SECTION_TITLE_PADDING_TOP,
    SIDEBAR_PADDING_X,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_2XS, FS_XS, TRACKING_DISPLAY, TRACKING_OVERLINE, Weight};
use crate::widgets::frames::px;
use crate::widgets::rows::{RowState, focus_inside, paint_divider, paint_row};
use crate::widgets::text::{caps, line, paint_left, paint_right, plain};
use crate::widgets::tooltip;
use crate::{Model, Text, Texts};

pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let current = current_archive(model);
    let channels = model.channels().detected();
    let mut picked = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    Frame::NONE
        .inner_margin(Margin {
            left: px(SIDEBAR_PADDING_X),
            right: px(SIDEBAR_PADDING_X),
            top: px(SECTION_TITLE_PADDING_TOP),
            bottom: px(SECTION_TITLE_PADDING_BOTTOM),
        })
        .show(ui, |ui| {
            let title = caps(
                &texts.get(Text::ChannelsHeading),
                FS_2XS,
                Weight::Bold,
                TRACKING_OVERLINE,
            );
            ui.label(title.color(palette::GRAY_2_TEXT));
            if channels.is_empty() {
                ui.label(
                    plain(texts.get(Text::ChannelsNone), FS_XS, Weight::Regular)
                        .color(palette::GRAY_2_TEXT),
                );
            }
        });
    for channel in channels {
        if show_row(ui, texts, channel, current) && current != Some(channel.archive()) {
            picked = Some(channel.archive().to_path_buf());
        }
    }
    if let Some(path) = picked {
        super::open(ui, model, path);
    }
}

fn show_row(ui: &mut Ui, texts: &Texts, channel: &Channel, current: Option<&Path>) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = current == Some(channel.archive());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, active, channel.name()));
    let state = RowState::of_list(active, response.hovered());
    paint_row(ui.painter(), rect, state);
    paint_divider(ui.painter(), rect);
    focus_inside(ui.painter(), rect, &response);
    let (_, colour) = state.colours();
    let inner = rect.shrink2(vec2(SIDEBAR_PADDING_X, 0.0));
    let name = line(
        ui,
        caps(channel.name(), FS_XS, Weight::Bold, TRACKING_DISPLAY),
        inner.width(),
    );
    let name_width = name.size().x;
    paint_left(ui, inner, name, colour);
    let build = super::status::build_label(texts, channel.build());
    let build_rect = inner.with_min_x(inner.left() + name_width + CHANNEL_GAP);
    let galley = line(
        ui,
        plain(build.as_str(), FS_XS, Weight::Regular),
        build_rect.width(),
    );
    paint_right(ui, build_rect, galley, palette::GRAY_2_TEXT);
    ui.interact(build_rect, response.id.with("build"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &build));
    tooltip::hover(response, |ui| {
        tooltip::line(ui, channel.archive().display().to_string());
    })
    .clicked()
}

fn current_archive(model: &Model) -> Option<&Path> {
    model.opening_path().or_else(|| {
        model
            .browser()
            .map(|browser| browser.archive_index().path())
    })
}
