// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic canvas that uses the layout features together, against a stored reference image.
//!
//! Set `P4KX_UPDATE_REFERENCE` to write the reference again after a deliberate change.

use p4kx_testkit::{TestCanvas, TestColour, TestDim, TestWidget};

use crate::support::{BLUE, GREEN, RED, SCREEN, TestResult, matches_reference, render};

const REFERENCE: &str = "tests/it/reference/layout_sample.png";
const YELLOW: TestColour = TestColour::Solid([255, 255, 0, 255]);
const GREY: TestColour = TestColour::Solid([60, 60, 70, 255]);

fn sample() -> TestCanvas {
    TestCanvas::new(SCREEN, "sample", 160.0, 100.0)
        .raw()
        .template(
            TestWidget::display("tile")
                .size(TestDim::Fixed(18.0), TestDim::Fixed(18.0))
                .background(RED),
        )
        .static_array("tiles", 3)
        .widget(TestWidget::display("backdrop").background(GREY))
        .widget(
            TestWidget::list("tiles", "tile", "tiles")
                .size(TestDim::Fixed(70.0), TestDim::Fixed(20.0))
                .offset(4.0, 4.0)
                .flex("Row", "SpaceBetween", "Center", 0.0),
        )
        .widget(
            TestWidget::display("grid")
                .size(TestDim::Fixed(60.0), TestDim::Fixed(40.0))
                .offset(90.0, 4.0)
                .grid("Horizontal", 2.0),
        )
        .widget(cell("g0", GREEN))
        .widget(cell("g1", BLUE))
        .widget(cell("g2", YELLOW))
        .widget(cell("g3", RED))
        .widget(
            TestWidget::display("bar")
                .size(TestDim::Fixed(60.0), TestDim::Fixed(8.0))
                .anchor(0.3, 0.7)
                .pivot(0.5, 0.5)
                .rotate(-30.0)
                .background(YELLOW),
        )
        .widget(
            TestWidget::display("window")
                .size(TestDim::Fixed(40.0), TestDim::Fixed(30.0))
                .offset(100.0, 60.0)
                .overflow("Clip")
                .background(BLUE),
        )
        .widget(
            TestWidget::display("wide")
                .parent("window")
                .size(TestDim::Fixed(80.0), TestDim::Fixed(10.0))
                .offset(-10.0, 10.0)
                .background(GREEN),
        )
        .widget(
            TestWidget::display("scaled")
                .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
                .anchor(0.0, 1.0)
                .pivot(0.0, 1.0)
                .scale(2.0, 1.0)
                .background(RED),
        )
}

fn cell(name: &str, colour: TestColour) -> TestWidget {
    TestWidget::display(name)
        .parent("grid")
        .size(TestDim::Fixed(28.0), TestDim::Fixed(18.0))
        .background(colour)
}

#[test]
fn the_layout_sample_matches_its_reference_image() -> TestResult {
    let image = render(&sample())?;
    matches_reference(REFERENCE, &image, 3, 8)
}
