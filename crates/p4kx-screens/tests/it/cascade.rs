// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The style cascade: selectors, modifiers, palettes, and what the reader does not evaluate.

use p4kx_dataforge::DataForge;
use p4kx_screens::{Composed, ScreenComposer};
use p4kx_testkit::{
    TestCanvas, TestColour, TestCondition, TestModifier, TestStyle, TestStyleEntry, TestWidget,
    tag_record,
};

use crate::support::{TestResult, database, guid_text, near, pixel, stage};

const CANVAS: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0002;
const STYLE: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0003;
const PARENT_TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0004;
const CHILD_TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0005;
const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);

fn compose(
    canvas: &TestCanvas,
    extra: Vec<p4kx_testkit::TestRecord>,
) -> Result<(Composed, Vec<u8>), Box<dyn std::error::Error>> {
    let records = std::iter::once(canvas.record()).chain(extra);
    let bytes = database(records);
    let composed = {
        let forge = DataForge::open(&bytes)?;
        ScreenComposer::new(&forge).compose(&guid_text(CANVAS), Some("test"))?
    };
    Ok((composed, bytes))
}

fn palette() -> TestStyle {
    TestStyle::new(
        STYLE,
        "s_test_hud",
        vec![
            TestColour::Solid([0, 0, 255, 255]),
            TestColour::Solid([0, 255, 0, 255]),
            TestColour::Solid([255, 255, 255, 255]),
        ],
    )
}

#[test]
fn a_style_entry_sets_the_background_of_matching_widgets_only() -> TestResult {
    let entry = TestStyleEntry::new("cards")
        .when(TestCondition::Type("Card"))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(TestColour::Slot("Accent1", 1.0)));
    let canvas = TestCanvas::new(CANVAS, "cascade", 20.0, 10.0)
        .brand_style(STYLE, vec![entry])
        .widget(TestWidget::display("root").flex("Row", "Start", "Start", 0.0))
        .widget(
            TestWidget::new("BuildingBlocks_WidgetCard", "card")
                .parent("root")
                .size(
                    p4kx_testkit::TestDim::Fixed(10.0),
                    p4kx_testkit::TestDim::Fixed(10.0),
                ),
        )
        .widget(TestWidget::display("plain").parent("root").size(
            p4kx_testkit::TestDim::Fixed(10.0),
            p4kx_testkit::TestDim::Fixed(10.0),
        ));
    let (composed, _bytes) = compose(&canvas, vec![palette().record()])?;
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 5, 5), [0, 255, 0, 255], 0),
        "the card takes Accent1"
    );
    assert_eq!(
        pixel(&image, 15, 5),
        [0, 0, 0, 0],
        "the display widget stays empty"
    );
    Ok(())
}

#[test]
fn a_parent_selector_looks_at_the_tags_of_the_parent_only() -> TestResult {
    let entry = TestStyleEntry::new("children of parents")
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Parent(vec![TestCondition::Tag(PARENT_TAG)]))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(RED));
    let size = |widget: TestWidget| {
        widget.size(
            p4kx_testkit::TestDim::Fixed(10.0),
            p4kx_testkit::TestDim::Fixed(10.0),
        )
    };
    let canvas = TestCanvas::new(CANVAS, "cascade", 40.0, 10.0)
        .brand_style(STYLE, vec![entry])
        .widget(TestWidget::display("root").flex("Row", "Start", "Start", 0.0))
        .widget(size(
            TestWidget::display("tagged").parent("root").tag(PARENT_TAG),
        ))
        .widget(size(TestWidget::display("inside tagged").parent("tagged")))
        .widget(size(TestWidget::display("untagged").parent("root")))
        .widget(size(
            TestWidget::display("inside untagged").parent("untagged"),
        ));
    let extra = vec![
        palette().record(),
        tag_record(PARENT_TAG, "Parent"),
        tag_record(CHILD_TAG, "Child"),
    ];
    let (composed, _bytes) = compose(&canvas, extra)?;
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 5, 5), [255, 0, 0, 255], 0),
        "the child of the tagged parent"
    );
    assert_eq!(
        pixel(&image, 15, 5),
        [0, 0, 0, 0],
        "the child of the untagged parent"
    );
    Ok(())
}

#[test]
fn modifiers_can_hide_a_subtree_and_set_alpha() -> TestResult {
    let hide = TestStyleEntry::new("hide")
        .when(TestCondition::Tag(PARENT_TAG))
        .set(TestModifier::Active(false));
    let fade = TestStyleEntry::new("fade")
        .when(TestCondition::Tag(CHILD_TAG))
        .set(TestModifier::Alpha(0.5));
    let size = |widget: TestWidget| {
        widget.size(
            p4kx_testkit::TestDim::Fixed(10.0),
            p4kx_testkit::TestDim::Fixed(10.0),
        )
    };
    let canvas = TestCanvas::new(CANVAS, "cascade", 20.0, 10.0)
        .embedded(hide)
        .embedded(fade)
        .widget(TestWidget::display("root").flex("Row", "Start", "Start", 0.0))
        .widget(size(
            TestWidget::display("hidden")
                .parent("root")
                .tag(PARENT_TAG)
                .background(RED),
        ))
        .widget(size(
            TestWidget::display("faded")
                .parent("root")
                .tag(CHILD_TAG)
                .background(RED),
        ));
    let extra = vec![
        tag_record(PARENT_TAG, "Parent"),
        tag_record(CHILD_TAG, "Child"),
    ];
    let (composed, _bytes) = compose(&canvas, extra)?;
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 5, 5), [255, 0, 0, 128], 1),
        "IsActive false takes the widget out of the layout, and the other has half alpha"
    );
    assert_eq!(
        pixel(&image, 15, 5),
        [0, 0, 0, 0],
        "the hidden widget leaves no room"
    );
    Ok(())
}

#[test]
fn a_selector_the_reader_does_not_evaluate_is_reported() -> TestResult {
    let entry = TestStyleEntry::new("mystery")
        .when(TestCondition::Unknown)
        .set(TestModifier::Active(false));
    let canvas = TestCanvas::new(CANVAS, "cascade", 20.0, 10.0)
        .embedded(entry)
        .widget(TestWidget::display("root").background(RED));
    let (composed, _bytes) = compose(&canvas, Vec::new())?;
    let report = composed.report();
    let note = report
        .unsupported
        .iter()
        .find(|entry| entry.element == "StyleEntry")
        .ok_or("the entry is not reported")?;
    assert!(
        note.reason.contains("Unknown"),
        "the selector is named: {}",
        note.reason
    );
    assert_eq!(note.examples, ["mystery"], "the entry is named");
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 5, 5), [255, 0, 0, 255], 0),
        "an entry with an unevaluated selector does not apply"
    );
    Ok(())
}

#[test]
fn a_colour_slot_without_a_palette_is_reported_and_skipped() -> TestResult {
    let canvas = TestCanvas::new(CANVAS, "cascade", 20.0, 10.0)
        .widget(TestWidget::display("root").background(TestColour::Slot("Base", 1.0)));
    let (composed, _bytes) = compose(&canvas, Vec::new())?;
    assert!(
        composed
            .report()
            .unsupported
            .iter()
            .any(|entry| entry.reason.contains("colour slot Base")),
        "the slot is reported: {:?}",
        composed.report().unsupported
    );
    assert_eq!(composed.report().shapes, 0, "nothing is drawn");
    Ok(())
}

#[test]
fn style_entries_win_over_widget_fields_and_inner_canvases_over_outer_ones() -> TestResult {
    let colour = |colour| {
        TestStyleEntry::new("paint")
            .when(TestCondition::Type("Base"))
            .set(TestModifier::EnableBackground(true))
            .set(TestModifier::Background(colour))
    };
    let inner_guid = CANVAS + 0x100;
    let outer = TestCanvas::new(CANVAS, "outer", 10.0, 10.0)
        .embedded(colour(TestColour::Solid([0, 0, 255, 255])))
        .widget(TestWidget::display("outer root"))
        .widget(
            TestWidget::new("BuildingBlocks_WidgetCanvas", "inner")
                .parent("outer root")
                .with("canvas", p4kx_testkit::TestValue::Reference(inner_guid)),
        );
    let inner = TestCanvas::new(inner_guid, "inner", 10.0, 10.0)
        .embedded(colour(TestColour::Solid([0, 255, 0, 255])))
        .widget(TestWidget::display("inner root").background(RED));
    let bytes = database([outer.record(), inner.record()]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(CANVAS), None)?;
    let image = composed.rasterize(&stage())?.image;
    assert!(
        near(pixel(&image, 5, 5), [0, 255, 0, 255], 0),
        "the entry of the inner canvas beats the widget's red and the outer entry's blue"
    );
    Ok(())
}
