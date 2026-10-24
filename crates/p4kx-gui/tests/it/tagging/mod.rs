// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Tags tab tests: a synthetic Archive with a tag tree, opened in a headless
//! window.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::OutputCommand;
use egui_kittest::Harness;
use p4kx_gui::{Language, Model, TagsStatus, WorkTab, ui};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const DATABASE: u128 = 0x0d;
pub(crate) const WEAPON: u128 = 0xa0;
pub(crate) const GUN: u128 = 0xa1;
pub(crate) const MISSILE: u128 = 0xa2;
pub(crate) const ARMOUR: u128 = 0xb0;
const PATH: &str = "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase.xml";

fn references(guids: &[u128]) -> TestValue {
    TestValue::Array(guids.iter().map(|g| TestValue::Reference(*g)).collect())
}

fn tag(guid: u128, name: &str, legacy: u32, children: &[u128]) -> TestRecord {
    TestRecord::new(guid, "Tag", format!("Tag.{name}"), PATH)
        .value("tagName", TestValue::text(name))
        .value("legacyGUID", TestValue::UInt32(legacy))
        .value("children", references(children))
}

/// Returns the text form of the GUID `value`.
pub(crate) fn text(value: u128) -> String {
    let hex = format!("{value:032x}");
    let part = |from: usize, to: usize| hex.get(from..to).unwrap_or_default().to_owned();
    format!(
        "{}-{}-{}-{}-{}",
        part(0, 8),
        part(8, 12),
        part(12, 16),
        part(16, 20),
        part(20, 32)
    )
}

/// Returns a DataForge with the tags Weapon (Gun, Missile) and Armour.
pub(crate) fn forge() -> Vec<u8> {
    DataForgeBuilder::new()
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
                .value("tags", references(&[WEAPON, ARMOUR])),
        )
        .record(tag(WEAPON, "Weapon", 7, &[GUN, MISSILE]))
        .record(tag(GUN, "Gun", u32::MAX, &[]))
        .record(tag(MISSILE, "Missile", 9, &[]))
        .record(tag(ARMOUR, "Armour", 4, &[]))
        .build()
}

/// Returns the GUID of tag `index` of [`forge_many`].
pub(crate) const fn many_guid(index: usize) -> u128 {
    0x00c0_0000_0000_4000_8000_0000_0000_0000 + index as u128
}

/// Returns a DataForge with `count` tags: a chain of `depth` tags, each below the one before, then
/// the others as roots; tag `i` is called `Tag{i}` and has the legacy ID `1000 + i`.
pub(crate) fn forge_many(count: usize, depth: usize) -> Vec<u8> {
    let roots: Vec<u128> = std::iter::once(0)
        .chain(depth..count)
        .map(many_guid)
        .collect();
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
    for index in 0..count {
        let children = if index + 1 < depth {
            vec![many_guid(index + 1)]
        } else {
            Vec::new()
        };
        let legacy = 1000 + u32::try_from(index).unwrap_or_default();
        builder = builder.record(tag(
            many_guid(index),
            &format!("Tag{index}"),
            legacy,
            &children,
        ));
    }
    builder.build()
}

pub(crate) fn archive(dir: &Path, entries: Vec<TestEntry>) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    let mut builder = ArchiveBuilder::new();
    for entry in entries {
        builder = builder.entry(entry);
    }
    builder.write_to(&path)?;
    Ok(path)
}

pub(crate) fn harness(model: Model) -> Harness<'static, Model> {
    harness_sized(model, [1400.0, 900.0])
}

pub(crate) fn harness_sized(model: Model, size: [f32; 2]) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(60);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_steps(3);
    Ok(())
}

pub(crate) fn read(model: &Model) -> bool {
    !matches!(
        model.tags().status(),
        TagsStatus::Idle | TagsStatus::Loading
    )
}

/// Opens the Archive of `entries` in a window on the Tags tab and waits until the tags are read.
pub(crate) fn opened(
    entries: Vec<TestEntry>,
    language: Language,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    opened_sized(entries, language, [1400.0, 900.0])
}

/// Opens the Archive of `entries` like [`opened`] in a window of `size` points.
pub(crate) fn opened_sized(
    entries: Vec<TestEntry>,
    language: Language,
    size: [f32; 2],
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), entries)?;
    let mut harness = harness_sized(Model::new(language), size);
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Tags);
    harness.run_steps(2);
    wait_until(&mut harness, read)?;
    Ok((dir, harness))
}

/// Opens the synthetic tag tree.
pub(crate) fn opened_tags(
    language: Language,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    opened(vec![TestEntry::zstd("Data\\Game2.dcb", forge())], language)
}

pub(crate) fn copied(harness: &Harness<'_, Model>) -> Vec<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}
