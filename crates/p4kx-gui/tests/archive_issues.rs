//! An open Archive whose Entries have defects: the window shows a warning notice with their number.

use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn open(builder: &ArchiveBuilder) -> Result<(tempfile::TempDir, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("timed out opening the Archive".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok((dir, model))
}

fn harness(model: Model) -> Harness<'static, Model> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

#[test]
fn defective_entries_are_announced_with_their_number() -> TestResult {
    let (_dir, model) = open(
        &ArchiveBuilder::new()
            .entry(TestEntry::stored("Data\\a.txt", "a"))
            .entry(TestEntry::stored("Data\\b.txt", "b").with_encryption_field(2))
            .entry(TestEntry::stored("Data\\c.txt", "c").with_encryption_field(3)),
    )?;
    let mut harness = harness(model);

    harness.step();

    harness.get_by_label_contains("2 Entries of the Archive have defects");
    Ok(())
}

#[test]
fn a_sound_archive_shows_no_notice() -> TestResult {
    let (_dir, model) = open(&ArchiveBuilder::new().entry(TestEntry::stored("Data\\a.txt", "a")))?;
    let mut harness = harness(model);

    harness.step();

    assert!(
        harness.query_by_label_contains("have defects").is_none(),
        "nothing to announce"
    );
    Ok(())
}
