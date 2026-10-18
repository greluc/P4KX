use egui::{
    Color32, Context, CornerRadius, Frame, Id, LayerId, Margin, Modal, Painter, Rect, Shape,
    Stroke, Ui, Window, pos2, vec2,
};

use super::text::caps;
use crate::theme::desktop::{
    ACCENT_BAR, BRACKET, BRACKET_STROKE, BUTTON_XS_HEIGHT, HEADER_PADDING_X, MODAL_TOP_EDGE,
    MODAL_WIDE_WIDTH, MODAL_WIDTH, NOTICE_PADDING_Y, PREVIEW_PADDING_X, SCRIM, TITLE_SIZE,
    TITLE_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::{GLOW_DANGER_LG, GLOW_PRIMARY_LG, SPACE_3, SPACE_4, Weight};

/// The width of the body of a modal.
pub(crate) const MODAL_BODY_WIDTH: f32 = MODAL_WIDTH - 2.0 * SPACE_4;
/// The width of the body of a wide modal.
pub(crate) const MODAL_WIDE_BODY_WIDTH: f32 = MODAL_WIDE_WIDTH - 2.0 * SPACE_4;

/// The tone of a modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    Normal,
    Danger,
}

impl Tone {
    const fn accent(self) -> Color32 {
        match self {
            Self::Normal => palette::PRIMARY,
            Self::Danger => palette::DANGER,
        }
    }
}

const fn margin(x: f32, y: f32) -> Margin {
    Margin::symmetric(px(x), px(y))
}

/// Returns a margin of `value` points as egui stores it.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the design system's margins are small whole points"
)]
pub(crate) const fn px(value: f32) -> i8 {
    value as i8
}

/// The frame of the header: the panel surface, its padding and room for the 2 point orange rule.
pub(crate) fn header() -> Frame {
    Frame::NONE
        .fill(palette::SURFACE_PANEL)
        .inner_margin(Margin {
            left: px(HEADER_PADDING_X),
            right: px(HEADER_PADDING_X),
            top: 1,
            bottom: 3,
        })
}

/// Paints the 2 point orange rule under the header at `rect`.
pub(crate) fn header_rule(ui: &Ui, rect: Rect) {
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    ui.painter()
        .rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
}

/// The frame of the status bar.
pub(crate) fn footer() -> Frame {
    Frame::NONE
        .fill(palette::SURFACE_PANEL)
        .inner_margin(margin(SPACE_4, 0.0))
}

/// The frame of the sidebar.
pub(crate) fn sidebar() -> Frame {
    Frame::NONE.fill(palette::SURFACE_PANEL)
}

/// The frame of the canvas around the work surface.
pub(crate) fn canvas() -> Frame {
    Frame::NONE.fill(palette::SURFACE_CANVAS)
}

/// The frame of a `.list-panel` or the preview: the panel surface with a hairline.
pub(crate) fn panel() -> Frame {
    Frame::NONE
        .fill(palette::SURFACE_PANEL)
        .stroke(Stroke::new(1.0, palette::GRAY_3))
}

/// The frame of the preview body.
pub(crate) fn preview_body() -> Frame {
    Frame::NONE.inner_margin(margin(PREVIEW_PADDING_X, 8.0))
}

/// Shows an `.alert.alert-warning` with `body` inside.
pub(crate) fn warning_alert<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    let fill = crate::theme::desktop::ALERT_WARNING_FILL;
    let accent = palette::WARNING;
    let response = Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(1.0, accent))
        .inner_margin(Margin {
            left: px(SPACE_4 + ACCENT_BAR),
            right: px(SPACE_4),
            top: px(NOTICE_PADDING_Y),
            bottom: px(NOTICE_PADDING_Y),
        })
        .show(ui, body);
    let rect = response.response.rect;
    let bar = Rect::from_min_size(rect.min, vec2(ACCENT_BAR, rect.height()));
    ui.painter().rect_filled(bar, CornerRadius::ZERO, accent);
    response.inner
}

fn modal_frame(tone: Tone) -> Frame {
    let glow = if tone == Tone::Danger {
        GLOW_DANGER_LG
    } else {
        GLOW_PRIMARY_LG
    };
    Frame::NONE
        .fill(palette::BG_DARK_GRAY)
        .stroke(Stroke::new(1.0, palette::GRAY_3))
        .shadow(glow)
        .inner_margin(margin(SPACE_4, SPACE_3))
}

/// Returns the title of a modal head in `tone`.
pub(crate) fn modal_title(title: &str, tone: Tone) -> egui::RichText {
    let colour = if tone == Tone::Danger {
        palette::DANGER_TEXT
    } else {
        palette::WHITE
    };
    caps(title, TITLE_SIZE, Weight::Bold, TITLE_TRACKING).color(colour)
}

/// Returns a non-modal `.krt-modal` window titled `title`; its head is drawn by [`head`].
pub(crate) fn window(title: &str, tone: Tone) -> Window<'static> {
    Window::new(title.to_owned())
        .title_bar(false)
        .frame(modal_frame(tone))
}

/// Shows the head of a modal: the upper-case `title` over a hairline, with the close cross named
/// `close` when given. A `labelled` head carries `title` as its accessible name; a window's head
/// leaves that to the window. Returns whether the cross was clicked.
pub(crate) fn head(
    ui: &mut Ui,
    (title, tone): (&str, Tone),
    labelled: bool,
    close: Option<&str>,
) -> bool {
    let galley = super::text::line(ui, modal_title(title, tone), f32::INFINITY);
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), BUTTON_XS_HEIGHT),
        egui::Sense::hover(),
    );
    if labelled {
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, title));
    }
    super::text::paint_left(ui, rect, galley, palette::WHITE);
    let closed = close.is_some_and(|label| {
        let cross = Rect::from_min_size(
            pos2(rect.right() - BUTTON_XS_HEIGHT, rect.top()),
            vec2(BUTTON_XS_HEIGHT, BUTTON_XS_HEIGHT),
        );
        super::button::close_icon(ui, cross, label).clicked()
    });
    let y = rect.bottom() + SPACE_3;
    ui.painter().hline(
        rect.expand2(vec2(SPACE_4, 0.0)).x_range(),
        y,
        Stroke::new(1.0, palette::GRAY_3),
    );
    ui.add_space(SPACE_3 + SPACE_4);
    closed
}

/// Starts the foot of a modal: its gap and the hairline across the whole modal above it.
pub(crate) fn foot_rule(ui: &mut Ui) {
    ui.add_space(SPACE_4);
    let top = ui.cursor().top();
    ui.painter().hline(
        ui.max_rect().expand2(vec2(SPACE_4, 0.0)).x_range(),
        top,
        Stroke::new(1.0, palette::GRAY_3),
    );
    ui.add_space(SPACE_3);
}

/// Returns a `.krt-modal` on the flat scrim.
pub(crate) fn modal(id: Id, tone: Tone) -> Modal {
    Modal::new(id)
        .frame(modal_frame(tone))
        .backdrop_color(SCRIM)
}

/// Paints the top edge and the corner brackets of a modal at `rect` on its `layer`.
pub(crate) fn modal_decoration(ctx: &Context, layer: LayerId, rect: Rect, tone: Tone) {
    let painter = ctx.layer_painter(layer);
    let accent = tone.accent();
    let edge = Rect::from_min_size(rect.min, vec2(rect.width(), MODAL_TOP_EDGE));
    painter.rect_filled(edge, CornerRadius::ZERO, accent);
    brackets(&painter, rect.expand(BRACKET_STROKE / 2.0), accent);
}

fn brackets(painter: &Painter, rect: Rect, colour: Color32) {
    let stroke = Stroke::new(BRACKET_STROKE, colour);
    let (min, max) = (rect.min, rect.max);
    painter.line_segment([min, pos2(min.x + BRACKET, min.y)], stroke);
    painter.line_segment([min, pos2(min.x, min.y + BRACKET)], stroke);
    painter.line_segment([max, pos2(max.x - BRACKET, max.y)], stroke);
    painter.line_segment([max, pos2(max.x, max.y - BRACKET)], stroke);
}

/// Paints a dashed hairline around `rect` in `colour`.
pub(crate) fn dashed(ui: &Ui, rect: Rect, colour: Color32) {
    let stroke = Stroke::new(1.0, colour);
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    let mut shapes = Vec::new();
    for pair in corners.windows(2) {
        shapes.extend(Shape::dashed_line(pair, stroke, 4.0, 4.0));
    }
    ui.painter().extend(shapes);
}

/// The id of a modal from a fixed `name`.
pub(crate) fn modal_id(name: &'static str) -> Id {
    Id::unique(name)
}
