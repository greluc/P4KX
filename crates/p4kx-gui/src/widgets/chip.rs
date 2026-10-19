use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::text::{caps, line};
use crate::theme::desktop::{
    CHIP_DANGER_FILL, CHIP_INFO_FILL, CHIP_PADDING_X, CHIP_PADDING_Y, CHIP_SUCCESS_FILL,
    CHIP_TRACKING, CHIP_WARNING_FILL,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_2XS, Weight};

/// The tone of a `.chip`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    /// `.chip--success`.
    Success,
    /// `.chip--danger`.
    Danger,
    /// `.chip--warning`.
    Warning,
    /// `.chip--info`.
    Info,
    /// `.chip--muted`.
    Muted,
}

/// The colours of a chip: its border, its text tint and its faint fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChipColours {
    pub(crate) border: Color32,
    pub(crate) text: Color32,
    pub(crate) fill: Color32,
}

/// Returns the colours of a chip in `tone`; text takes the accessible tint of the hue.
pub(crate) const fn colours(tone: Tone) -> ChipColours {
    match tone {
        Tone::Success => ChipColours {
            border: palette::SUCCESS,
            text: palette::SUCCESS_TEXT,
            fill: CHIP_SUCCESS_FILL,
        },
        Tone::Danger => ChipColours {
            border: palette::DANGER,
            text: palette::DANGER_TEXT,
            fill: CHIP_DANGER_FILL,
        },
        Tone::Warning => ChipColours {
            border: palette::WARNING,
            text: palette::WARNING,
            fill: CHIP_WARNING_FILL,
        },
        Tone::Info => ChipColours {
            border: palette::INFO,
            text: palette::INFO_TEXT,
            fill: CHIP_INFO_FILL,
        },
        Tone::Muted => ChipColours {
            border: palette::GRAY_3,
            text: palette::GRAY_2_TEXT,
            fill: Color32::TRANSPARENT,
        },
    }
}

/// Lays out the upper-case `label` of a chip.
pub(crate) fn galley(ui: &Ui, label: &str) -> Arc<Galley> {
    line(
        ui,
        caps(label, FS_2XS, Weight::Bold, CHIP_TRACKING),
        f32::INFINITY,
    )
}

/// Paints a chip in `tone` with `galley` inside, its left edge at `left_centre`; returns its
/// rectangle.
pub(crate) fn paint(ui: &Ui, left_centre: Pos2, galley: Arc<Galley>, tone: Tone) -> Rect {
    paint_with(ui.painter(), left_centre, galley, tone)
}

/// Paints a chip like [`paint`], cut off at the edges of `clip`.
pub(crate) fn paint_clipped(
    ui: &Ui,
    left_centre: Pos2,
    galley: Arc<Galley>,
    tone: Tone,
    clip: Rect,
) -> Rect {
    let painter = ui.painter().with_clip_rect(clip.intersect(ui.clip_rect()));
    paint_with(&painter, left_centre, galley, tone)
}

fn paint_with(painter: &Painter, left_centre: Pos2, galley: Arc<Galley>, tone: Tone) -> Rect {
    let colours = colours(tone);
    let size = galley.size() + vec2(2.0 * CHIP_PADDING_X, 2.0 * CHIP_PADDING_Y);
    let rect = Rect::from_min_size(pos2(left_centre.x, left_centre.y - size.y / 2.0), size);
    painter.rect(
        rect,
        CornerRadius::ZERO,
        colours.fill,
        Stroke::new(1.0, colours.border),
        StrokeKind::Inside,
    );
    painter.galley(
        rect.min + vec2(CHIP_PADDING_X, CHIP_PADDING_Y),
        galley,
        colours.text,
    );
    rect
}

#[cfg(test)]
mod tests {
    use super::{Tone, colours};
    use crate::theme::palette;

    #[test]
    fn chip_text_takes_the_accessible_tints() {
        assert_eq!(
            colours(Tone::Success).text,
            palette::SUCCESS_TEXT,
            "success"
        );
        assert_eq!(colours(Tone::Danger).text, palette::DANGER_TEXT, "danger");
        assert_eq!(colours(Tone::Warning).text, palette::WARNING, "warning");
        assert_eq!(colours(Tone::Info).text, palette::INFO_TEXT, "info");
        assert_eq!(colours(Tone::Muted).text, palette::GRAY_2_TEXT, "muted");
        assert_eq!(colours(Tone::Muted).border, palette::GRAY_3, "muted border");
        assert_eq!(
            colours(Tone::Danger).border,
            palette::DANGER,
            "hue as border"
        );
    }
}
