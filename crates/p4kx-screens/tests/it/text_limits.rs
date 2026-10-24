// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The limits of ADR-0045 hold for the texts of screens too.

use p4kx_dataforge::DataForge;
use p4kx_gfx::{MAX_SCENE_GLYPHS, MAX_TEXT_CHARS};
use p4kx_screens::{ScreenComposer, TextSources};
use p4kx_testkit::{
    DataForgeBuilder, TestCanvas, TestColour, TestCondition, TestDim, TestModifier, TestStyleEntry,
    TestValue, TestWidget, entity_schema, screens_schema,
};

use crate::support::TestResult;
use crate::text::fonts;

const LONG: u128 = 0x7a15_c3e0_29b4_4d68_91f7_e5a2_0b83_d6c4;
const PATH: &str = "libs/foundry/records/ui/buildingblocks/test/long_texts";

fn long_texts(count: usize, text: &str) -> Vec<u8> {
    let entry = TestStyleEntry::new("texts")
        .when(TestCondition::Type("Text"))
        .set(TestModifier::Fill(TestColour::Solid([255, 255, 255, 255])));
    let canvas = (0..count).fold(
        TestCanvas::new(LONG, "long_texts", 200.0, 100.0)
            .embedded(entry)
            .widget(TestWidget::display("root")),
        |canvas, index| {
            canvas.widget(
                TestWidget::new("BuildingBlocks_WidgetText", format!("text{index}"))
                    .parent("root")
                    .size(TestDim::Fixed(200.0), TestDim::Fixed(100.0))
                    .with("locString", TestValue::text(text))
                    .with("fontSize", TestValue::Single(1.0)),
            )
        },
    );
    screens_schema(entity_schema(DataForgeBuilder::new()))
        .record(canvas.record())
        .build()
}

#[test]
fn a_text_is_cut_to_the_characters_of_a_text_field() -> TestResult {
    let fonts = fonts()?;
    let bytes = long_texts(1, &"A".repeat(MAX_TEXT_CHARS + 904));
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge)
        .with_texts(TextSources {
            fonts: &fonts,
            localise: None,
        })
        .compose(PATH, None)?;
    let report = composed.report();
    assert!(
        report
            .unsupported
            .iter()
            .any(|entry| entry.reason == format!("text cut to {MAX_TEXT_CHARS} characters")),
        "{:?}",
        report.unsupported
    );
    assert_eq!(report.texts_drawn, 1, "the cut text is drawn");
    assert!(
        report.shapes <= MAX_TEXT_CHARS,
        "at most the characters kept"
    );
    Ok(())
}

#[test]
fn the_glyphs_of_a_screen_are_bounded() -> TestResult {
    let fonts = fonts()?;
    let count = MAX_SCENE_GLYPHS / MAX_TEXT_CHARS + 8;
    let bytes = long_texts(count, &"A ".repeat(MAX_TEXT_CHARS / 2));
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge)
        .with_texts(TextSources {
            fonts: &fonts,
            localise: None,
        })
        .compose(PATH, None)?;
    let report = composed.report();
    assert!(report.truncated, "the limit stops the composition");
    assert!(
        report.texts_drawn < count && report.texts_left_out > 0,
        "{} drawn, {} left out",
        report.texts_drawn,
        report.texts_left_out
    );
    assert!(report.shapes <= MAX_SCENE_GLYPHS, "{}", report.shapes);
    Ok(())
}
