// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Sizing, margins, padding, flex containers and raw units of the layout.

use p4kx_dataforge::DataForge;
use p4kx_gfx::RgbaImage;
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{TestCanvas, TestColour, TestDim, TestWidget};

use crate::support::{TestResult, database, guid_text, near, pixel, stage};

const CANVAS: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0001;
const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);
const GREEN: TestColour = TestColour::Solid([0, 255, 0, 255]);
const BLUE: TestColour = TestColour::Solid([0, 0, 255, 255]);
const NOTHING: [u8; 4] = [0, 0, 0, 0];

#[expect(
    clippy::needless_pass_by_value,
    reason = "every test hands over the canvas it builds"
)]
fn render(canvas: TestCanvas) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let bytes = database([canvas.record()]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(CANVAS), None)?;
    Ok(composed.rasterize(&stage())?.image)
}

fn boxes(count: u8, direction: &str, justification: &str, gap: f32) -> TestCanvas {
    let colours = [RED, GREEN, BLUE];
    let root = TestWidget::display("root").flex(direction, justification, "Start", gap);
    (0..count)
        .filter_map(|at| {
            colours
                .get(usize::from(at))
                .copied()
                .map(|colour| (at, colour))
        })
        .fold(
            TestCanvas::new(CANVAS, "layout", 100.0, 50.0).widget(root),
            |canvas, (at, colour)| {
                canvas.widget(
                    TestWidget::display(format!("box{at}"))
                        .parent("root")
                        .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
                        .background(colour),
                )
            },
        )
}

#[test]
fn percent_and_fixed_sizes_fill_the_parent() -> TestResult {
    let image = render(
        TestCanvas::new(CANVAS, "layout", 100.0, 50.0).widget(
            TestWidget::display("half")
                .size(TestDim::Percent(0.5), TestDim::Fixed(20.0))
                .background(RED),
        ),
    )?;
    assert!(
        near(pixel(&image, 25, 10), [255, 0, 0, 255], 0),
        "inside the box"
    );
    assert_eq!(pixel(&image, 75, 10), NOTHING, "right of the box");
    assert_eq!(pixel(&image, 25, 30), NOTHING, "below the box");
    Ok(())
}

#[test]
fn space_between_puts_the_first_and_last_item_at_the_ends() -> TestResult {
    let image = render(boxes(3, "Row", "SpaceBetween", 0.0))?;
    for x in [5, 50, 95] {
        assert_ne!(pixel(&image, x, 5), NOTHING, "an item at x = {x}");
    }
    assert_eq!(
        pixel(&image, 25, 5),
        NOTHING,
        "free space between the items"
    );
    Ok(())
}

#[test]
fn a_reversed_row_starts_with_the_last_item() -> TestResult {
    let image = render(boxes(2, "RowReverse", "Start", 0.0))?;
    assert!(
        near(pixel(&image, 5, 5), [0, 255, 0, 255], 0),
        "the second box first"
    );
    assert!(
        near(pixel(&image, 15, 5), [255, 0, 0, 255], 0),
        "then the first"
    );
    Ok(())
}

#[test]
fn a_column_with_a_gap_stacks_downwards() -> TestResult {
    let image = render(boxes(2, "Column", "Start", 5.0))?;
    assert!(
        near(pixel(&image, 5, 5), [255, 0, 0, 255], 0),
        "the first box at the top"
    );
    assert_eq!(pixel(&image, 5, 12), NOTHING, "the gap");
    assert!(
        near(pixel(&image, 5, 20), [0, 255, 0, 255], 0),
        "the second box after the gap"
    );
    Ok(())
}

#[test]
fn items_share_the_free_space_by_their_grow() -> TestResult {
    let canvas = TestCanvas::new(CANVAS, "layout", 100.0, 50.0)
        .widget(TestWidget::display("root").flex("Row", "Start", "Start", 0.0))
        .widget(
            TestWidget::display("a")
                .parent("root")
                .size(TestDim::Fixed(0.0), TestDim::Fixed(10.0))
                .grow(1.0)
                .background(RED),
        )
        .widget(
            TestWidget::display("b")
                .parent("root")
                .size(TestDim::Fixed(0.0), TestDim::Fixed(10.0))
                .grow(3.0)
                .background(GREEN),
        );
    let image = render(canvas)?;
    assert!(
        near(pixel(&image, 24, 5), [255, 0, 0, 255], 0),
        "the quarter of the first"
    );
    assert!(
        near(pixel(&image, 26, 5), [0, 255, 0, 255], 0),
        "the rest of the second"
    );
    assert!(
        near(pixel(&image, 99, 5), [0, 255, 0, 255], 0),
        "up to the end"
    );
    Ok(())
}

#[test]
fn margin_and_padding_move_boxes_inwards() -> TestResult {
    let canvas = TestCanvas::new(CANVAS, "layout", 100.0, 50.0)
        .widget(TestWidget::display("outer").padding([10.0, 0.0, 0.0, 20.0]))
        .widget(
            TestWidget::display("inner")
                .parent("outer")
                .margin([5.0, 0.0, 0.0, 5.0])
                .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
                .background(RED),
        );
    let image = render(canvas)?;
    assert_eq!(pixel(&image, 24, 14), NOTHING, "inside padding and margin");
    assert!(
        near(pixel(&image, 26, 16), [255, 0, 0, 255], 0),
        "the box after both"
    );
    Ok(())
}

#[test]
fn a_share_of_the_height_keeps_a_box_square() -> TestResult {
    let image = render(
        TestCanvas::new(CANVAS, "layout", 100.0, 50.0).widget(
            TestWidget::display("square")
                .size(TestDim::OfHeight(1.0), TestDim::Percent(1.0))
                .background(RED),
        ),
    )?;
    assert!(
        near(pixel(&image, 49, 25), [255, 0, 0, 255], 0),
        "as wide as high"
    );
    assert_eq!(pixel(&image, 51, 25), NOTHING, "not wider");
    Ok(())
}

#[test]
fn a_widget_out_of_the_flow_sits_at_its_offset() -> TestResult {
    let canvas = boxes(1, "Row", "Start", 0.0).widget(
        TestWidget::display("floating")
            .parent("root")
            .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
            .offset(60.0, 30.0)
            .out_of_flow()
            .background(BLUE),
    );
    let image = render(canvas)?;
    assert!(
        near(pixel(&image, 65, 35), [0, 0, 255, 255], 0),
        "at the offset"
    );
    assert!(
        near(pixel(&image, 5, 5), [255, 0, 0, 255], 0),
        "the flow item stays first"
    );
    Ok(())
}

#[test]
fn raw_canvases_count_fixed_values_in_canvas_heights() -> TestResult {
    let image = render(
        TestCanvas::new(CANVAS, "layout", 200.0, 100.0)
            .raw()
            .widget(
                TestWidget::display("quarter")
                    .size(TestDim::Fixed(0.5), TestDim::Fixed(0.25))
                    .background(RED),
            ),
    )?;
    assert!(
        near(pixel(&image, 49, 24), [255, 0, 0, 255], 0),
        "50 x 25 pixels"
    );
    assert_eq!(pixel(&image, 51, 24), NOTHING, "not wider");
    assert_eq!(pixel(&image, 49, 26), NOTHING, "not higher");
    Ok(())
}
