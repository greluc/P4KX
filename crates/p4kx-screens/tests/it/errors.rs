// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Canvases that cannot be found or drawn, and canvases that refer to themselves.

use p4kx_dataforge::DataForge;
use p4kx_screens::{ScreenComposer, ScreenError};
use p4kx_testkit::{TestCanvas, TestDim, TestValue, TestWidget, tag_record};

use crate::support::{TestResult, database, guid_text};

const FIRST: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0010;
const SECOND: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0011;
const TAG: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0012;

#[test]
fn an_unknown_canvas_is_an_error() -> TestResult {
    let bytes = database([TestCanvas::new(FIRST, "first", 10.0, 10.0).record()]);
    let forge = DataForge::open(&bytes)?;
    let error = ScreenComposer::new(&forge).compose("libs/none", None);
    assert!(
        matches!(error, Err(ScreenError::UnknownCanvas { .. })),
        "{error:?}"
    );
    let by_guid = ScreenComposer::new(&forge).compose(&guid_text(SECOND), None);
    assert!(
        matches!(by_guid, Err(ScreenError::UnknownCanvas { .. })),
        "{by_guid:?}"
    );
    Ok(())
}

#[test]
fn a_record_that_is_no_canvas_is_an_error() -> TestResult {
    let bytes = database([tag_record(TAG, "Tag")]);
    let forge = DataForge::open(&bytes)?;
    let error = ScreenComposer::new(&forge).compose(&guid_text(TAG), None);
    assert!(
        matches!(error, Err(ScreenError::NotACanvas { ref type_name, .. }) if type_name == "Tag"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn canvases_sharing_a_record_path_are_ambiguous() -> TestResult {
    let bytes = database([
        TestCanvas::new(FIRST, "shared", 10.0, 10.0).record(),
        TestCanvas::new(SECOND, "shared", 10.0, 10.0).record(),
    ]);
    let forge = DataForge::open(&bytes)?;
    let error = ScreenComposer::new(&forge)
        .compose("libs/foundry/records/ui/buildingblocks/test/shared", None);
    assert!(
        matches!(error, Err(ScreenError::AmbiguousCanvas { count: 2, .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_canvas_without_an_area_is_an_error() -> TestResult {
    let bytes = database([TestCanvas::new(FIRST, "flat", 10.0, 0.0).record()]);
    let forge = DataForge::open(&bytes)?;
    let error = ScreenComposer::new(&forge).compose(&guid_text(FIRST), None);
    assert!(
        matches!(error, Err(ScreenError::EmptyCanvas { .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_canvas_that_nests_itself_is_drawn_once_and_reported() -> TestResult {
    let canvas = |guid, name, nested| {
        TestCanvas::new(guid, name, 10.0, 10.0).widget(
            TestWidget::new("BuildingBlocks_WidgetCanvas", "again")
                .size(TestDim::Percent(1.0), TestDim::Percent(1.0))
                .with("canvas", TestValue::Reference(nested)),
        )
    };
    let bytes = database([
        canvas(FIRST, "first", SECOND).record(),
        canvas(SECOND, "second", FIRST).record(),
    ]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(FIRST), None)?;
    let report = composed.report();
    assert_eq!(report.canvases, 2, "each canvas of the cycle once");
    assert_eq!(report.missing_canvases.len(), 1, "the way back is reported");
    assert!(
        report
            .missing_canvases
            .iter()
            .any(|entry| entry.contains("contains itself")),
        "{:?}",
        report.missing_canvases
    );
    assert!(!report.is_complete(), "a cycle makes the render incomplete");
    Ok(())
}

#[test]
fn a_nested_canvas_that_is_missing_is_reported() -> TestResult {
    let canvas = TestCanvas::new(FIRST, "first", 10.0, 10.0).widget(
        TestWidget::new("BuildingBlocks_WidgetCanvas", "gone")
            .size(TestDim::Percent(1.0), TestDim::Percent(1.0))
            .with("canvas", TestValue::Reference(SECOND)),
    );
    let bytes = database([canvas.record()]);
    let forge = DataForge::open(&bytes)?;
    let composed = ScreenComposer::new(&forge).compose(&guid_text(FIRST), None)?;
    assert_eq!(composed.report().canvases, 1, "only the first canvas");
    assert_eq!(
        composed.report().missing_canvases,
        ["gone (a reference that no Record answers)"],
        "the widget with the dangling reference is named"
    );
    Ok(())
}
