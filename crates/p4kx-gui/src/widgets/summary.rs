use egui::{CornerRadius, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, pos2, vec2};

use super::chip::{self, Tone};
use super::rows::chevron;
use super::text::{caps, line, paint_left, plain};
use crate::theme::desktop::{
    ACCENT_BAR, CHEVRON, CHIP_PADDING_Y, FACT_KEY_GAP, FACT_KEY_SIZE, FACT_KEY_TRACKING,
    FACT_VALUE_SIZE, FACTS_GAP, FACTS_PADDING_X, FACTS_PADDING_Y, KPI_PADDING_X, KPI_PADDING_Y,
    KPI_VALUE_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_1, Weight};

/// One fact of a `.facts-bar`: an upper-case key and a value.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fact<'a> {
    pub(crate) key: &'a str,
    pub(crate) value: &'a str,
}

/// Shows a `.facts-bar` with `old` and `new`, an orange chevron between them for the direction.
pub(crate) fn facts_bar(ui: &mut Ui, old: Fact<'_>, new: Fact<'_>) {
    let key = |text: &str| caps(text, FACT_KEY_SIZE, Weight::Bold, FACT_KEY_TRACKING);
    let value = |text: &str| plain(text, FACT_VALUE_SIZE, Weight::Bold);
    let galleys = [
        line(ui, key(old.key), f32::INFINITY),
        line(ui, value(old.value), f32::INFINITY),
        line(ui, key(new.key), f32::INFINITY),
        line(ui, value(new.value), f32::INFINITY),
    ];
    let tallest = galleys
        .iter()
        .map(|galley| galley.size().y)
        .fold(CHEVRON, f32::max);
    let height = 2.0f32.mul_add(FACTS_PADDING_Y, tallest);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    response.widget_info(|| {
        let label = format!("{} {} {} {}", old.key, old.value, new.key, new.value);
        WidgetInfo::labeled(Role::Label, true, label)
    });
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, palette::GRAY_3),
    );
    let mut left = rect.left() + FACTS_PADDING_X;
    let [old_key, old_value, new_key, new_value] = galleys;
    for (index, (galley, colour)) in [
        (old_key, palette::GRAY_2_TEXT),
        (old_value, palette::WHITE),
        (new_key, palette::GRAY_2_TEXT),
        (new_value, palette::WHITE),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 2 {
            let centre = pos2(left + CHEVRON / 2.0, rect.center().y);
            chevron(ui.painter(), centre, CHEVRON, false, palette::PRIMARY);
            left += CHEVRON + FACTS_GAP;
        }
        let width = galley.size().x;
        paint_left(ui, rect.with_min_x(left), galley, colour);
        left += width
            + if index % 2 == 0 {
                FACT_KEY_GAP
            } else {
                FACTS_GAP
            };
    }
}

/// Shows a `.kpi-total` tile of `width` with the chip `label` in `tone` over the `value`.
pub(crate) fn kpi_tile(ui: &mut Ui, label: &str, value: &str, tone: Tone, width: f32) {
    let label_galley = chip::galley(ui, label);
    let value_galley = line(ui, plain(value, KPI_VALUE_SIZE, Weight::Bold), width);
    let chip_height = 2.0f32.mul_add(CHIP_PADDING_Y, label_galley.size().y);
    let height = 2.0f32.mul_add(
        KPI_PADDING_Y,
        chip_height + value_galley.size().y + SPACE_1 / 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, format!("{label} {value}")));
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::ZERO,
        palette::BG_DARK_GRAY,
        Stroke::new(1.0, palette::GRAY_3),
        StrokeKind::Inside,
    );
    let bar = Rect::from_min_size(rect.min, vec2(ACCENT_BAR, rect.height()));
    painter.rect_filled(bar, CornerRadius::ZERO, palette::PRIMARY);
    let inner = rect.shrink2(vec2(KPI_PADDING_X, KPI_PADDING_Y));
    chip::paint(
        ui,
        pos2(inner.left(), inner.top() + chip_height / 2.0),
        label_galley,
        tone,
    );
    ui.painter().galley(
        pos2(inner.left(), inner.top() + chip_height + SPACE_1 / 2.0),
        value_galley,
        palette::WHITE,
    );
}
