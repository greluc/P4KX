//! Extracting the changes of one Comparison side from the window, drawn and driven headless over
//! a pair of synthetic Archives.

mod comparing;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use comparing::{Pair, TestResult, pair};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{ComparisonSide, ComparisonStatus, Language, Model, WorkTab, ui};

fn harness(pair: &Pair, language: Language, out: &Path) -> Harness<'static, Model> {
    let mut model = Model::new(language);
    model.set_work_tab(WorkTab::Comparison);
    model.set_extraction_folder(Some(out.to_path_buf()));
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, pair.old.clone());
    tab.set_archive(ComparisonSide::New, pair.new.clone());
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        )
}

fn settle(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.step();
    harness.step();
    Ok(())
}

fn finished(model: &Model) -> bool {
    matches!(model.comparison().status(), ComparisonStatus::Finished(view) if !view.is_filtering())
}

fn ended(model: &Model) -> bool {
    model
        .extraction()
        .is_some_and(|extraction| !extraction.is_running())
}

fn compared(
    pair: &Pair,
    language: Language,
    out: &Path,
) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let mut harness = harness(pair, language, out);
    harness.step();
    let start = if language == Language::German {
        "Vergleichen"
    } else {
        "Compare"
    };
    harness.get_by_label(start).click();
    settle(&mut harness, finished)?;
    Ok(harness)
}

fn files(out: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut found = BTreeSet::new();
    let mut folders = vec![out.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for item in fs::read_dir(&folder)? {
            let path = item?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                found.insert(path.strip_prefix(out)?.to_string_lossy().replace('/', "\\"));
            }
        }
    }
    Ok(found)
}

#[test]
fn both_sides_offer_their_changes_as_ghost_actions_with_counts() -> TestResult {
    let pair = pair()?;
    let out = pair.dir.path().join("out");

    let harness = compared(&pair, Language::BritishEnglish, &out)?;

    for label in ["Extract old (2)…", "Extract new (3)…"] {
        assert!(
            !harness.get_by_label(label).accesskit_node().is_disabled(),
            "{label} is enabled"
        );
    }
    Ok(())
}

#[test]
fn the_new_side_extracts_through_the_setup_and_shows_the_summary() -> TestResult {
    let pair = pair()?;
    let out = pair.dir.path().join("out");
    fs::create_dir_all(&out)?;
    let mut harness = compared(&pair, Language::BritishEnglish, &out)?;

    harness.get_by_label("Extract new (3)…").click();
    settle(&mut harness, |model| model.extraction_setup().is_some())?;
    harness.get_by_label("Added and changed Entries of the new Archive (3)");
    harness.get_by_label("Existing folder").click();
    harness.step();
    harness.step();
    harness.get_by_label("Extract").click();
    harness.step();
    harness.step();
    settle(&mut harness, ended)?;

    harness.get_by_label("Extraction finished");
    harness.get_by_label("Extracted 3, skipped 0, failed 0, cancelled 0");
    assert_eq!(
        files(&out)?,
        BTreeSet::from([
            "Data\\Libs\\crc.xml".to_owned(),
            "Data\\Libs\\size.xml".to_owned(),
            "Engine\\added.cfg".to_owned(),
        ]),
        "the added and changed Entries of the new side"
    );
    Ok(())
}

#[test]
fn the_old_side_never_extracts_the_removed_entry() -> TestResult {
    let pair = pair()?;
    let out = pair.dir.path().join("out");
    fs::create_dir_all(&out)?;
    let mut harness = compared(&pair, Language::BritishEnglish, &out)?;

    harness.get_by_label("Extract old (2)…").click();
    settle(&mut harness, |model| model.extraction_setup().is_some())?;
    harness.get_by_label("Changed Entries of the old Archive, in their old version (2)");
    harness.get_by_label("Existing folder").click();
    harness.step();
    harness.step();
    harness.get_by_label("Extract").click();
    harness.step();
    harness.step();
    settle(&mut harness, ended)?;

    assert_eq!(
        files(&out)?,
        BTreeSet::from([
            "Data\\Libs\\crc.xml".to_owned(),
            "Data\\Libs\\size.xml".to_owned(),
        ]),
        "the removed Entry is not part of it"
    );
    Ok(())
}

#[test]
fn the_filter_narrows_the_actions_and_the_setup_names_it() -> TestResult {
    let pair = pair()?;
    let out = pair.dir.path().join("out");
    let mut harness = compared(&pair, Language::BritishEnglish, &out)?;

    harness.get_by_role(Role::TextInput).click();
    harness.step();
    harness.get_by_role(Role::TextInput).type_text("engine");
    settle(&mut harness, |model| {
        finished(model)
            && model
                .comparison()
                .view()
                .is_some_and(|view| view.filter_text() == "engine")
    })?;

    assert!(
        harness
            .get_by_label("Extract old (0)…")
            .accesskit_node()
            .is_disabled(),
        "no changed Entry of the old side matches"
    );
    harness.get_by_label("Extract new (1)…").click();
    settle(&mut harness, |model| model.extraction_setup().is_some())?;
    harness.get_by_label("Added and changed Entries of the new Archive (1)");
    harness.get_by_label("Only the changes matching the Filter engine");
    Ok(())
}

#[test]
fn the_german_window_speaks_german() -> TestResult {
    let pair = pair()?;
    let out = pair.dir.path().join("out");
    let mut harness = compared(&pair, Language::German, &out)?;

    harness.get_by_label("Alt extrahieren (2) …");
    harness.get_by_label("Neu extrahieren (3) …").click();
    settle(&mut harness, |model| model.extraction_setup().is_some())?;

    harness.get_by_label("Hinzugefügte und geänderte Einträge des neuen Archivs (3)");
    Ok(())
}
