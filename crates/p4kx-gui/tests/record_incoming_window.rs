//! The "Referenced by" tab of the preview in the window, driven headless over a synthetic
//! DataForge: the list, following a line, a Record nobody refers to and German.

mod previewing_records;

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, IncomingStatus, Language, Model, PreviewTab, RecordPreviewStatus, ui};
use previewing_records::{ITEM_PATH, MAKER_PATH, SHIP_PATH, TestResult, archive, database};
use tempfile::TempDir;

fn window(language: Language) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (dir, path) = archive(&database())?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 1000.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_steps(2);
    Ok(())
}

fn read(model: &Model) -> bool {
    model
        .browser()
        .and_then(Browser::record_preview)
        .is_some_and(|preview| matches!(preview.status(), RecordPreviewStatus::Ready(_)))
}

fn listed(model: &Model) -> bool {
    model
        .browser()
        .and_then(Browser::incoming_preview)
        .is_some_and(|list| matches!(list.status(), IncomingStatus::Ready(_)))
}

fn selected_path(model: &Model) -> Option<String> {
    let browser = model.browser()?;
    let id = browser.selected_record()?;
    Some(browser.records()?.record(id)?.record_path().to_owned())
}

fn open_record(
    harness: &mut Harness<'_, Model>,
    language: Language,
    (folder, name): (&str, &str),
) -> TestResult {
    let expand = |what: &str| match language {
        Language::German => format!("{what} aufklappen"),
        Language::BritishEnglish => format!("Expand {what}"),
    };
    harness.get_by_label(&expand("DataForge")).click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label(&expand("libs")).click();
    harness.run_steps(2);
    harness.get_by_label(&expand(folder)).click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, name).click();
    wait_until(harness, read)
}
#[test]
fn the_tab_lists_the_referring_records_and_a_line_follows_its_record() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, Language::BritishEnglish, ("makers", "one"))?;

    harness
        .get_by_role_and_label(Role::Tab, "Referenced by")
        .click();
    wait_until(&mut harness, listed)?;

    assert_eq!(selected_path(harness.state()).as_deref(), Some(MAKER_PATH));
    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Incoming,
        "the tab"
    );
    assert!(
        harness
            .query_by_label_contains("1 Reference from 1 Record")
            .is_some(),
        "the count line"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Row, "maker: libs/items/sample")
            .is_some(),
        "the field and the referring Record"
    );
    harness
        .get_by_role_and_label(Role::Link, "Follow the Reference to libs/items/sample")
        .click();
    wait_until(&mut harness, |model| {
        selected_path(model).as_deref() == Some(ITEM_PATH) && listed(model)
    })?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Incoming,
        "the tab stays"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Row, "cargo: libs/ships/hauler")
            .is_some(),
        "the list of the Item"
    );
    Ok(())
}

#[test]
fn a_record_nobody_refers_to_says_so() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, Language::BritishEnglish, ("ships", "hauler"))?;

    harness
        .get_by_role_and_label(Role::Tab, "Referenced by")
        .click();
    wait_until(&mut harness, listed)?;

    assert!(
        harness
            .query_by_label("No Record refers to this Record.")
            .is_some(),
        "the note"
    );
    assert_eq!(selected_path(harness.state()).as_deref(), Some(SHIP_PATH));
    Ok(())
}

#[test]
fn the_tab_speaks_german() -> TestResult {
    let (_dir, mut harness) = window(Language::German)?;
    open_record(&mut harness, Language::German, ("makers", "one"))?;

    harness
        .get_by_role_and_label(Role::Tab, "Referenziert von")
        .click();
    wait_until(&mut harness, listed)?;

    assert!(
        harness
            .query_by_label_contains("1 Referenz von 1 Record")
            .is_some(),
        "the count line"
    );
    Ok(())
}
