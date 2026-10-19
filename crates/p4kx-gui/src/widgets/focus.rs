use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui};

use crate::theme::desktop::FOCUS_WIDTH;
use crate::theme::tokens::RADIUS_SM_POINTS;

/// Paints the 2 point focus ring `offset` outside `rect`, following the `radius-sm` corner of
/// the control; a negative offset draws it inside.
pub(crate) fn focus_ring(ui: &Ui, rect: Rect, colour: Color32, offset: f32) {
    focus_ring_rounded(ui, rect, colour, offset, RADIUS_SM_POINTS);
}

/// Paints the 2 point focus ring like [`focus_ring`] with square corners, for rows, tabs, links
/// and other parts without a radius.
pub(crate) fn focus_ring_square(ui: &Ui, rect: Rect, colour: Color32, offset: f32) {
    focus_ring_rounded(ui, rect, colour, offset, 0);
}

/// Paints the 2 point focus ring like [`focus_ring`] for a control with the corner `radius`, as
/// the checkbox's: the ring corner is `radius` plus `offset`, and square stays square.
pub(crate) fn focus_ring_rounded(ui: &Ui, rect: Rect, colour: Color32, offset: f32, radius: u8) {
    ui.painter().rect_stroke(
        rect.expand(offset),
        CornerRadius::same(ring_radius(radius, offset)),
        Stroke::new(FOCUS_WIDTH, colour),
        StrokeKind::Outside,
    );
}

/// Paints the 2 point focus ring `offset` outside the circle of `radius` around `centre`.
pub(crate) fn focus_ring_circle(ui: &Ui, centre: Pos2, radius: f32, colour: Color32, offset: f32) {
    ui.painter().circle_stroke(
        centre,
        radius + offset + FOCUS_WIDTH / 2.0,
        Stroke::new(FOCUS_WIDTH, colour),
    );
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the corner is clamped to the u8 range first"
)]
fn ring_radius(radius: u8, offset: f32) -> u8 {
    if radius == 0 {
        0
    } else {
        (f32::from(radius) + offset)
            .clamp(0.0, f32::from(u8::MAX))
            .round() as u8
    }
}

#[cfg(test)]
mod tests {
    use egui::{Shape, Vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::ring_radius;
    use crate::theme::desktop::{CHECKBOX_RADIUS, FOCUS_OFFSET, FOCUS_WIDTH, TOGGLE_SIZE};
    use crate::theme::palette;
    use crate::widgets::text::link;
    use crate::widgets::toggle::{checkbox, radio};

    #[test]
    fn a_ring_corner_grows_by_the_offset_and_a_square_corner_stays_square() {
        assert_eq!(ring_radius(CHECKBOX_RADIUS, FOCUS_OFFSET), 4, "checkbox");
        assert_eq!(ring_radius(4, FOCUS_OFFSET), 6, "radius-sm control");
        assert_eq!(ring_radius(0, FOCUS_OFFSET), 0, "square");
        assert_eq!(ring_radius(3, -3.0), 0, "inside ring cannot go negative");
    }

    fn is_ring(stroke: egui::Stroke) -> bool {
        stroke.color == palette::PRIMARY && (stroke.width - FOCUS_WIDTH).abs() < f32::EPSILON
    }

    fn rings(harness: &Harness<'_, ()>) -> Vec<Shape> {
        harness
            .output()
            .shapes
            .iter()
            .map(|clipped| clipped.shape.clone())
            .filter(|shape| match shape {
                Shape::Rect(rect) => is_ring(rect.stroke),
                Shape::Circle(circle) => is_ring(circle.stroke),
                _ => false,
            })
            .collect()
    }

    fn focused(label: &str, show: impl FnMut(&mut egui::Ui) + 'static) -> Vec<Shape> {
        let mut harness = Harness::builder()
            .with_context_setup(crate::theme::apply)
            .with_size(Vec2::new(200.0, 100.0))
            .build_ui(show);
        harness.run();
        harness.get_by_label(label).focus();
        harness.run();
        rings(&harness)
    }

    #[test]
    fn a_focused_checkbox_rings_with_its_own_corner() {
        let mut checked = false;
        let shapes = focused("Option", move |ui| {
            checkbox(ui, &mut checked, "Option");
        });

        let corner = shapes.iter().find_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.corner_radius),
            _ => None,
        });
        assert_eq!(
            corner.map(|corner| (corner.nw, corner.se)),
            Some((4, 4)),
            "corner"
        );
    }

    #[test]
    fn a_focused_radio_rings_as_a_circle() {
        let shapes = focused("Choice", |ui| {
            radio(ui, false, "Choice", true);
        });

        let radius = shapes.iter().find_map(|shape| match shape {
            Shape::Circle(circle) => Some(circle.radius),
            _ => None,
        });
        let want = TOGGLE_SIZE / 2.0 + FOCUS_OFFSET + FOCUS_WIDTH / 2.0;
        assert!(
            radius.is_some_and(|radius| (radius - want).abs() < 0.01),
            "circle radius {radius:?}, want {want}"
        );
        assert!(
            shapes.iter().all(|shape| !matches!(shape, Shape::Rect(_))),
            "no rectangle ring"
        );
    }

    #[test]
    fn a_focused_link_rings_square() {
        let shapes = focused("Docs", |ui| {
            link(ui, "Docs", "https://example.invalid");
        });

        let corner = shapes.iter().find_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.corner_radius),
            _ => None,
        });
        assert_eq!(
            corner.map(|corner| (corner.nw, corner.ne, corner.sw, corner.se)),
            Some((0, 0, 0, 0)),
            "corner"
        );
    }
}
