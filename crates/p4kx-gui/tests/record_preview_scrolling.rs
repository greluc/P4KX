//! The field lines of the Record tab scroll with the keyboard once they take the focus.

mod wide_forge;

use std::time::{Duration, Instant};

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, RecordPreviewStatus, ui};
use wide_forge::{FIELDS, TestResult, archive, name};

fn row(index: i32) -> String {
    format!("{}: 1", name(index))
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

fn previewed() -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), "LIVE", 1)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.get_by_label("Expand DataForge").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label("Expand libs").click();
    harness.run_steps(2);
    harness.get_by_label("Expand wide").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "record")
        .click();
    wait_until(&mut harness, read)?;
    Ok((dir, harness))
}

#[test]
fn the_focused_record_lines_scroll_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = previewed()?;
    let first = row(0);
    let second = row(1);
    let last = row(FIELDS - 1);
    let shown = |harness: &Harness<'_, Model>, label: &str| {
        harness.query_by_role_and_label(Role::Row, label).is_some()
    };
    assert!(shown(&harness, &first), "the first field");
    assert!(!shown(&harness, &last), "the last is below");

    harness.get_by_label("Record fields").focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    assert!(shown(&harness, &last), "End shows the last");
    assert!(!shown(&harness, &first), "the first is gone");

    harness.key_press(Key::Home);
    harness.run();
    assert!(shown(&harness, &first), "Home shows the first");

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(!shown(&harness, &first), "one row down");
    assert!(shown(&harness, &second), "only one row");

    harness.key_press(Key::PageDown);
    harness.run();
    assert!(!shown(&harness, &second), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(shown(&harness, &first), "back at the top");
    Ok(())
}
