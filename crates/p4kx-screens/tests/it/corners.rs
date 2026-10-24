// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Rounded and chamfered corners of boxes, from the widget and from the style cascade.

use p4kx_testkit::{TestCanvas, TestCondition, TestDim, TestModifier, TestStyleEntry, TestWidget};

use crate::support::{GREEN, RED, SCREEN, TestResult, compose, empty_at, green_at, red_at, render};

fn canvas() -> TestCanvas {
    TestCanvas::new(SCREEN, "corners", 100.0, 50.0)
}

fn boxed() -> TestWidget {
    TestWidget::display("box")
        .size(TestDim::Fixed(40.0), TestDim::Fixed(40.0))
        .background(RED)
}

#[test]
fn rounded_corners_cut_the_background() -> TestResult {
    let image = render(&canvas().widget(boxed().rounded(20.0)))?;
    assert!(red_at(&image, 20, 20), "the middle");
    assert!(red_at(&image, 20, 2), "the middle of an edge");
    assert!(empty_at(&image, 1, 1), "the corner");
    assert!(empty_at(&image, 38, 38), "the opposite corner");
    Ok(())
}

#[test]
fn chamfered_corners_cut_the_background_straight() -> TestResult {
    let image = render(&canvas().widget(boxed().chamfered(10.0)))?;
    assert!(empty_at(&image, 2, 2), "the cut corner");
    assert!(red_at(&image, 12, 12), "inside the cut");
    assert!(red_at(&image, 20, 1), "the edge");
    Ok(())
}

#[test]
fn a_border_follows_the_rounded_outline() -> TestResult {
    let image = render(&canvas().widget(boxed().rounded(20.0).border(4.0, GREEN)))?;
    assert!(red_at(&image, 20, 20), "the background inside the ring");
    assert!(green_at(&image, 20, 1), "the ring at the top");
    assert!(green_at(&image, 1, 20), "the ring at the left");
    assert!(empty_at(&image, 1, 1), "nothing in the corner");
    Ok(())
}

#[test]
fn a_style_entry_rounds_single_corners() -> TestResult {
    let entry = TestStyleEntry::new("round")
        .when(TestCondition::Type("Base"))
        .set(TestModifier::Number("BorderTopLeftRadius", 20.0));
    let image = render(&canvas().embedded(entry).widget(boxed()))?;
    assert!(empty_at(&image, 1, 1), "the rounded corner");
    assert!(red_at(&image, 38, 1), "a square corner");
    assert!(red_at(&image, 1, 38), "another square corner");
    Ok(())
}

#[test]
fn a_gradient_background_is_drawn_plain_and_listed_for_every_corner() -> TestResult {
    for corner in [
        "topLeftColor",
        "topRightColor",
        "bottomLeftColor",
        "bottomRightCColor",
    ] {
        let composed = compose(&canvas().widget(boxed().gradient(corner, GREEN)))?;
        let image = composed.rasterize(&crate::support::stage())?.image;
        assert!(red_at(&image, 20, 20), "{corner}: the plain colour");
        assert!(
            composed
                .report()
                .unsupported
                .iter()
                .any(|entry| entry.reason.contains("gradient")),
            "{corner}: {:?}",
            composed.report().unsupported
        );
    }
    Ok(())
}

#[test]
fn a_radius_beyond_the_half_of_the_side_is_held_there() -> TestResult {
    let image = render(&canvas().widget(boxed().rounded(1.0e9)))?;
    assert!(red_at(&image, 20, 20), "a circle of the box");
    assert!(empty_at(&image, 1, 1), "no corner");
    Ok(())
}
