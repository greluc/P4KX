// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What a frame of the Tags tab allocates once the tree is read and nothing changes: the
//! allocations per frame are counted over a run of frames, in the windows the design is checked at.
//!
//! The binary holds this one test, so that no other test allocates while the frames are counted.
//! `--nocapture` shows the counts.

use std::alloc::System;
use std::error::Error;
use std::io::Write;

use egui_kittest::Harness;
use p4kx_gui::{Language, Model, TagsStatus, WorkTab, ui};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[expect(
    dead_code,
    reason = "the waiting helpers are shared with the window tests, which use the rest"
)]
#[path = "it/waiting.rs"]
mod waiting;

use waiting::{RunQuiet, wait_for_model};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

type TestResult = Result<(), Box<dyn Error>>;

const TAGS: usize = 400;
const FRAMES: u64 = 20;
const DATABASE: u128 = 0x0d;
const PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

/// The most allocations one steady frame of the Tags tab may make in the larger window beyond a
/// frame of the empty Search tab, which holds the same window and no table.
const TAB_BUDGET: u64 = 450;

const fn guid_of(index: usize) -> u128 {
    0x00c0_0000_0000_4000_8000_0000_0000_0000 + index as u128
}

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn forge() -> Vec<u8> {
    let roots: Vec<u128> = (0..TAGS).map(guid_of).collect();
    let mut builder = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Tag")
                .field("tagName", TestType::String)
                .field("legacyGUID", TestType::UInt32)
                .array("children", TestType::ReferenceTo("Tag".to_owned())),
        )
        .structure(
            TestStruct::new("TagDatabase").array("tags", TestType::ReferenceTo("Tag".to_owned())),
        )
        .record(
            TestRecord::new(DATABASE, "TagDatabase", "TagDatabase.TagDatabase", PATH)
                .value("tags", references(&roots)),
        );
    for index in 0..TAGS {
        builder = builder.record(
            TestRecord::new(guid_of(index), "Tag", format!("Tag.{index}"), PATH)
                .value("tagName", TestValue::text(format!("Tag{index}")))
                .value(
                    "legacyGUID",
                    TestValue::UInt32(1000 + u32::try_from(index).unwrap_or_default()),
                )
                .value("children", references(&[])),
        );
    }
    builder.build()
}

fn window(
    dir: &std::path::Path,
    language: Language,
    size: [f32; 2],
) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge()))
        .write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_for_model(&mut harness, "the Archive to open", |model| {
        model.browser().is_some()
    })?;
    harness.state_mut().set_work_tab(WorkTab::Tags);
    harness.run_steps(2);
    wait_for_model(&mut harness, "the tags to be read", |model| {
        !matches!(
            model.tags().status(),
            TagsStatus::Idle | TagsStatus::Loading
        )
    })?;
    harness.run_quiet();
    Ok(harness)
}

fn allocations_per_frame(harness: &mut Harness<'_, Model>) -> u64 {
    harness.run_steps(3);
    let region = Region::new(GLOBAL);
    for _ in 0..FRAMES {
        harness.step();
    }
    region.change().allocations as u64 / FRAMES
}

fn report(line: &str) -> std::io::Result<()> {
    writeln!(std::io::stderr().lock(), "{line}")
}

#[test]
fn a_steady_frame_of_the_tags_tab_stays_within_its_allocation_budget() -> TestResult {
    let mut worst = 0;
    for language in [Language::BritishEnglish, Language::German] {
        for (large, size) in [(true, [1280.0, 800.0]), (false, [640.0, 400.0])] {
            let dir = tempfile::tempdir()?;
            let mut harness = window(dir.path(), language, size)?;
            let idle = allocations_per_frame(&mut harness);
            harness.state_mut().tags_mut().set_query("Tag1");
            let searching = allocations_per_frame(&mut harness);
            harness.state_mut().set_work_tab(WorkTab::Search);
            let other = allocations_per_frame(&mut harness);
            report(&format!(
                "{language:?} {size:?}: {idle} allocations per idle frame, {searching} while a search shows, {other} on the Search tab"
            ))?;
            if large {
                worst = worst.max(idle.max(searching).saturating_sub(other));
            }
        }
    }
    assert!(
        worst <= TAB_BUDGET,
        "the tab makes {worst} allocations per frame, over the budget of {TAB_BUDGET}"
    );
    Ok(())
}
