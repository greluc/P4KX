// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The entries that select the text a text field holds apply to the field, and only those.

use p4kx_dataforge::DataForge;
use p4kx_gfx::RgbaImage;
use p4kx_screens::{ScreenComposer, TextSources};
use p4kx_testkit::{
    SCREEN_TEXT_STYLE, TestCanvas, TestColour, TestCondition, TestDim, TestInstance, TestModifier,
    TestStyleEntry, TestValue, TestWidget, screens_text_sample, tag_record,
};

use crate::support::{SCREEN, TestResult, guid_text, near, pixel, stage};
use crate::text::fonts;

pub(crate) const PART: u128 = SCREEN + 0x300;
pub(crate) const BELOW: u128 = SCREEN + 0x301;
pub(crate) const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);
pub(crate) const GREEN: TestColour = TestColour::Solid([0, 255, 0, 255]);
pub(crate) const BLUE: TestColour = TestColour::Solid([0, 0, 255, 255]);

pub(crate) fn field(name: &'static str, at: f32) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetTextField", name)
        .parent("root")
        .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
        .offset(at, 0.0)
        .with(
            "labelProperties",
            TestValue::Struct(
                TestInstance::new("BuildingBlocks_ComponentLabelProperties")
                    .value("label", TestValue::text("A")),
            ),
        )
}

pub(crate) fn field_entry() -> TestStyleEntry {
    TestStyleEntry::new("field")
        .when(TestCondition::Type("TextField"))
        .set(TestModifier::FontStyle(SCREEN_TEXT_STYLE))
        .set(TestModifier::FontSize(10.0))
        .set(TestModifier::Fill(RED))
}

pub(crate) fn render(canvas: &TestCanvas) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let bytes = screens_text_sample()
        .record(canvas.record())
        .record(tag_record(PART, "Part"))
        .record(tag_record(BELOW, "Below"))
        .build();
    let forge = DataForge::open(&bytes)?;
    let fonts = fonts()?;
    let composed = ScreenComposer::new(&forge)
        .with_texts(TextSources {
            fonts: &fonts,
            localise: None,
        })
        .compose(&guid_text(SCREEN), None)?;
    Ok(composed.rasterize(&stage())?.image)
}

fn canvas() -> TestCanvas {
    let untyped = TestStyleEntry::new("untyped").set(TestModifier::Fill(GREEN));
    let part = TestStyleEntry::new("part")
        .when(TestCondition::Type("Text"))
        .when(TestCondition::Parent(vec![TestCondition::Tag(PART)]))
        .set(TestModifier::Fill(BLUE));
    TestCanvas::new(SCREEN, "text_part", 60.0, 20.0)
        .embedded(untyped)
        .embedded(field_entry())
        .embedded(part)
        .widget(TestWidget::display("root"))
        .widget(field("plain", 0.0))
        .widget(field("tagged", 30.0).tag(PART))
}

fn below() -> TestCanvas {
    let by_parent = TestStyleEntry::new("by_parent")
        .when(TestCondition::Parent(vec![TestCondition::Tag(BELOW)]))
        .set(TestModifier::Fill(GREEN));
    TestCanvas::new(SCREEN, "text_below", 60.0, 20.0)
        .embedded(field_entry())
        .embedded(by_parent)
        .widget(TestWidget::display("root"))
        .widget(field("plain", 0.0))
        .widget(field("tagged", 30.0).tag(BELOW))
}

#[test]
fn only_entries_typed_text_apply_to_the_text_of_a_field() -> TestResult {
    let image = render(&canvas())?;
    assert!(
        near(pixel(&image, 2, 4), [255, 0, 0, 255], 2),
        "an untyped entry before the entry of the field does not win: {:?}",
        pixel(&image, 2, 4)
    );
    assert!(
        near(pixel(&image, 32, 4), [0, 0, 255, 255], 2),
        "the entry typed Text below the tagged field applies: {:?}",
        pixel(&image, 32, 4)
    );
    Ok(())
}

#[test]
fn an_untyped_entry_that_selects_by_parent_applies_to_the_text_below_the_tagged_field() -> TestResult
{
    let image = render(&below())?;
    assert!(
        near(pixel(&image, 2, 4), [255, 0, 0, 255], 2),
        "a field without the tag keeps the colour of its own entry: {:?}",
        pixel(&image, 2, 4)
    );
    assert!(
        near(pixel(&image, 32, 4), [0, 255, 0, 255], 2),
        "the text below the tagged field takes the colour of the entry: {:?}",
        pixel(&image, 32, 4)
    );
    Ok(())
}
