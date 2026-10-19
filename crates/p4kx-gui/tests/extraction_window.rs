//! The Extraction windows driven headless through `egui_kittest`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Extraction, Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\good.txt", "good"))
        .entry(TestEntry::stored("Data\\NUL", "device"))
        .entry(TestEntry::stored("readme.txt", "readme"))
        .write_to(&path)?;
    Ok(path)
}

fn opened() -> Result<(TempDir, PathBuf, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_extraction_folder(Some(out.clone()));
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, out, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    settle(harness);
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        settle(harness);
    }
    settle(harness);
    Ok(())
}

fn settle(harness: &mut Harness<'_, Model>) {
    harness.run_steps(4);
}

fn ended(model: &Model) -> bool {
    model
        .extraction()
        .is_some_and(|extraction| !extraction.is_running())
}

#[test]
fn extract_all_asks_before_it_writes_and_then_shows_the_summary() -> TestResult {
    let (_dir, out, mut harness) = opened()?;

    harness.get_by_label("Extract all…").click();
    settle(&mut harness);
    harness.get_by_label("Whole Archive (3)");
    harness.get_by_label("Existing folder").click();
    settle(&mut harness);
    harness.get_by_label("Extract").click();
    settle(&mut harness);
    harness.get_by_label("Extract the whole Archive, 3 Entries of 16 B?");
    let written_before = fs::read_dir(&out)?.count();
    harness.get_by_label("Extract whole Archive").click();
    wait_until(&mut harness, ended)?;

    assert_eq!(written_before, 0, "nothing is written before confirming");
    harness.get_by_label("Extraction finished");
    harness.get_by_label("Extracted 2, skipped 0, failed 1, cancelled 0");
    harness.get_by_label("Data\\NUL: the path would leave the target folder");
    assert_eq!(fs::read(out.join("readme.txt"))?, b"readme", "written");
    Ok(())
}

#[test]
fn back_returns_from_the_confirmation_without_extracting() -> TestResult {
    let (_dir, _out, mut harness) = opened()?;

    harness.get_by_label("Extract all…").click();
    settle(&mut harness);
    harness.get_by_label("Extract").click();
    settle(&mut harness);
    harness.get_by_label("Back").click();
    settle(&mut harness);

    assert!(harness.state().extraction().is_none(), "nothing runs");
    assert!(
        harness
            .state()
            .extraction_setup()
            .is_some_and(|setup| !setup.is_confirming()),
        "back at the choices"
    );
    harness.get_by_label("Choose folder…");
    Ok(())
}

#[test]
fn closing_the_summary_dismisses_the_extraction() -> TestResult {
    let (_dir, _out, mut harness) = opened()?;
    harness.get_by_label("Extract shown…").click();
    settle(&mut harness);
    harness.get_by_label("Existing folder").click();
    settle(&mut harness);
    harness.get_by_label("Extract").click();
    settle(&mut harness);
    harness.get_by_label("Extract whole Archive").click();
    wait_until(&mut harness, ended)?;

    harness.get_by_label("Close").click();
    settle(&mut harness);

    assert!(
        harness
            .state()
            .extraction()
            .is_none_or(Extraction::is_running),
        "dismissed"
    );
    Ok(())
}

#[test]
fn the_summary_window_ends_below_its_close_button() -> TestResult {
    let (_dir, _out, mut harness) = opened()?;
    harness.get_by_label("Extract shown…").click();
    settle(&mut harness);
    harness.get_by_label("Existing folder").click();
    settle(&mut harness);
    harness.get_by_label("Extract").click();
    settle(&mut harness);
    harness.get_by_label("Extract whole Archive").click();
    wait_until(&mut harness, ended)?;

    let window = harness.get_by_label("Extraction finished").rect();
    let close = harness.get_by_label("Close").rect();

    assert!(
        window.max.y - close.max.y < 40.0,
        "no empty space under the foot: window {window:?}, Close {close:?}"
    );
    Ok(())
}
