//! The tag tree as a table: head and cells share their columns, the tag column keeps its width
//! and the table scrolls sideways in a narrow window.

use crate::tagging;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::theme::tags_metrics::{TAGS_GUID_WIDTH, TAGS_LEGACY_WIDTH, TAGS_NAME_MIN_WIDTH};
use p4kx_gui::{Language, Model};
use p4kx_testkit::TestEntry;
use tagging::{TestResult, forge_many, many_guid, opened_sized, text};

const DEPTH: usize = 12;

fn open(
    size: [f32; 2],
) -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn std::error::Error>> {
    let (dir, mut harness) = opened_sized(
        vec![TestEntry::zstd("Data\\Game2.dcb", forge_many(100, DEPTH))],
        Language::BritishEnglish,
        size,
    )?;
    for position in 0..DEPTH - 1 {
        harness.state_mut().tags_mut().toggle(position);
    }
    harness.run_steps(4);
    Ok((dir, harness))
}

fn cell_rect(harness: &Harness<'_, Model>, role: Role, label: &str) -> egui::Rect {
    harness.get_by_role_and_label(role, label).rect()
}

fn assert_columns_agree(harness: &Harness<'_, Model>, tag: usize) {
    let heads = [
        cell_rect(harness, Role::ColumnHeader, "Tag"),
        cell_rect(harness, Role::ColumnHeader, "GUID"),
        cell_rect(harness, Role::ColumnHeader, "Legacy ID"),
    ];
    let cells = [
        cell_rect(harness, Role::Button, &path_of(tag)),
        cell_rect(harness, Role::Cell, &text(many_guid(tag))),
        cell_rect(harness, Role::Cell, &(1000 + tag).to_string()),
    ];
    for (column, (head, cell)) in heads.into_iter().zip(cells).enumerate() {
        assert!(
            (head.left() - cell.left()).abs() < 0.5 && (head.right() - cell.right()).abs() < 0.5,
            "column {column}: head {head:?}, cell {cell:?}"
        );
    }
}

/// Returns the tag path of tag `index` of the chain: `Tag0.Tag1. …`.
fn path_of(index: usize) -> String {
    (0..=index)
        .map(|level| format!("Tag{level}"))
        .collect::<Vec<_>>()
        .join(".")
}

#[test]
fn the_head_and_the_cells_share_their_columns() -> TestResult {
    let (_dir, harness) = open([1400.0, 900.0])?;

    assert_columns_agree(&harness, 0);
    assert_columns_agree(&harness, DEPTH - 1);
    let guid = cell_rect(&harness, Role::ColumnHeader, "GUID");
    let legacy = cell_rect(&harness, Role::ColumnHeader, "Legacy ID");
    assert!(
        (guid.width() - TAGS_GUID_WIDTH).abs() < 0.5,
        "GUID column {guid:?}"
    );
    assert!(
        (legacy.width() - TAGS_LEGACY_WIDTH).abs() < 0.5,
        "legacy ID column {legacy:?}"
    );
    Ok(())
}

#[test]
fn a_narrow_window_keeps_the_tag_column_wide_and_scrolls_sideways() -> TestResult {
    let (_dir, harness) = open([640.0, 1000.0])?;

    let name = cell_rect(&harness, Role::Button, &path_of(DEPTH - 1));
    let guid = cell_rect(&harness, Role::Cell, &text(many_guid(DEPTH - 1)));
    assert!(
        name.width() >= TAGS_NAME_MIN_WIDTH - 0.5,
        "the tag column keeps {TAGS_NAME_MIN_WIDTH}: {name:?}"
    );
    assert!(
        name.right() <= guid.left() + 0.5,
        "the name cell {name:?} ends before the GUID cell {guid:?}"
    );
    assert_columns_agree(&harness, DEPTH - 1);
    Ok(())
}
