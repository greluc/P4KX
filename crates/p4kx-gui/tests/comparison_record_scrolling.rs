//! The field list of a Record change scrolls with the keyboard once it takes the focus.

mod wide_forge;

use std::time::{Duration, Instant};

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    ComparisonLevels, ComparisonSide, ComparisonStatus, ComparisonView, Language, Model,
    RecordChangeView, RecordDetailStatus, WorkTab, ui,
};
use wide_forge::{FIELDS, TestResult, archive, name};

fn row(index: i32, old: i32, new: i32) -> String {
    format!("{}: {old} to {new}", name(index))
}

fn settle(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn detail_loaded(model: &Model) -> bool {
    model
        .comparison()
        .view()
        .and_then(ComparisonView::records)
        .and_then(RecordChangeView::detail)
        .is_some_and(|detail| !matches!(detail.status(), RecordDetailStatus::Loading))
}

fn changed_record()
-> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let old = archive(dir.path(), "LIVE", 0)?;
    let new = archive(dir.path(), "PTU", 1)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_work_tab(WorkTab::Comparison);
    let tab = model.comparison_mut();
    tab.set_archive(ComparisonSide::Old, old);
    tab.set_archive(ComparisonSide::New, new);
    tab.set_levels(ComparisonLevels {
        entries: false,
        records: true,
    });
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run();
    harness.get_by_label("Compare").click();
    settle(&mut harness, |model| {
        matches!(model.comparison().status(), ComparisonStatus::Finished(_))
    })?;
    harness.get_by_role_and_label(Role::Tab, "Records").click();
    harness.run();
    harness.get_by_label("Changed: libs/wide/record").click();
    settle(&mut harness, detail_loaded)?;
    Ok((dir, harness))
}

#[test]
fn the_focused_field_list_scrolls_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = changed_record()?;
    let first = row(0, 0, 1);
    let second = row(1, 0, 1);
    let last = row(FIELDS - 1, 0, 1);
    assert!(harness.query_by_label(&first).is_some(), "the first field");
    assert!(harness.query_by_label(&last).is_none(), "the last is below");

    harness.get_by_label("Changed fields").focus();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    assert!(
        harness.query_by_label(&last).is_some(),
        "End shows the last"
    );
    assert!(
        harness.query_by_label(&first).is_none(),
        "the first is gone"
    );

    harness.key_press(Key::Home);
    harness.run();
    assert!(
        harness.query_by_label(&first).is_some(),
        "Home shows the first"
    );

    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.query_by_label(&first).is_none(), "one row down");
    assert!(harness.query_by_label(&second).is_some(), "only one row");

    harness.key_press(Key::PageDown);
    harness.run();
    assert!(harness.query_by_label(&second).is_none(), "a page down");

    harness.key_press(Key::PageUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(harness.query_by_label(&first).is_some(), "back at the top");
    Ok(())
}
