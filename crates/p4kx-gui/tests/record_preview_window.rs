//! The Record tab of the preview in the window, driven headless over a synthetic DataForge:
//! showing a Record, following References, back and forward, and a dangling Reference.

mod previewing_records;

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{Browser, Language, Model, PreviewTab, RecordPreviewStatus, ui};
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

fn selected_path(model: &Model) -> Option<String> {
    let browser = model.browser()?;
    let id = browser.selected_record()?;
    Some(browser.records()?.record(id)?.record_path().to_owned())
}

fn selected_is(path: &'static str) -> impl Fn(&Model) -> bool {
    move |model| selected_path(model).as_deref() == Some(path) && read(model)
}

fn open_record(harness: &mut Harness<'_, Model>, folder: &str, name: &str) -> TestResult {
    harness.get_by_label("Expand DataForge").click();
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    harness.get_by_label("Expand libs").click();
    harness.run_steps(2);
    harness.get_by_label(&format!("Expand {folder}")).click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, name).click();
    wait_until(harness, read)
}

#[test]
fn selecting_a_record_opens_its_type_guid_path_and_fields() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;

    open_record(&mut harness, "items", "sample")?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Record, "the tab");
    assert!(
        harness.query_by_label(ITEM_PATH).is_some(),
        "the record path"
    );
    assert!(
        harness
            .query_by_label_contains("Type Item GUID 70000000-0000-4000-8000-000000000002")
            .is_some(),
        "type and GUID"
    );
    for row in ["maker: libs/makers/one", "nodes: 2 elements"] {
        assert!(
            harness.query_by_role_and_label(Role::Row, row).is_some(),
            "{row}"
        );
    }
    Ok(())
}

#[test]
fn a_reference_link_selects_its_target_in_the_tree_and_the_preview() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, "items", "sample")?;

    harness
        .get_by_role_and_label(Role::Link, "Follow the Reference to libs/makers/one")
        .click();
    wait_until(&mut harness, selected_is(MAKER_PATH))?;

    assert!(
        harness
            .query_by_role_and_label(Role::Row, "rank: 7")
            .is_some(),
        "the Maker's field"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "one")
            .is_some(),
        "revealed in the tree"
    );
    Ok(())
}

#[test]
fn back_and_forward_work_with_buttons_keys_and_mouse_buttons() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, "ships", "hauler")?;
    harness
        .get_by_role_and_label(Role::Link, "Follow the Reference to libs/items/sample")
        .click();
    wait_until(&mut harness, selected_is(ITEM_PATH))?;
    harness
        .get_by_role_and_label(Role::Link, "Follow the Reference to libs/makers/one")
        .click();
    wait_until(&mut harness, selected_is(MAKER_PATH))?;

    harness.get_by_role_and_label(Role::Button, "Back").click();
    wait_until(&mut harness, selected_is(ITEM_PATH))?;
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    wait_until(&mut harness, selected_is(SHIP_PATH))?;
    harness
        .get_by_role_and_label(Role::Button, "Forward")
        .click();
    wait_until(&mut harness, selected_is(ITEM_PATH))?;
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    wait_until(&mut harness, selected_is(MAKER_PATH))?;
    mouse(&mut harness, PointerButton::Extra1);
    wait_until(&mut harness, selected_is(ITEM_PATH))?;
    mouse(&mut harness, PointerButton::Extra2);
    wait_until(&mut harness, selected_is(MAKER_PATH))?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Record, "the tab");
    Ok(())
}

#[test]
fn alt_left_goes_back_after_a_link_was_followed_with_the_keyboard() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, "ships", "hauler")?;

    let mut focused = false;
    for _ in 0..60 {
        harness.key_press(Key::Tab);
        harness.run_steps(2);
        focused = harness
            .get_by_role_and_label(Role::Link, "Follow the Reference to libs/items/sample")
            .accesskit_node()
            .is_focused();
        if focused {
            break;
        }
    }
    assert!(focused, "Tab reaches the link");
    harness.key_press(Key::Enter);
    wait_until(&mut harness, selected_is(ITEM_PATH))?;
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    wait_until(&mut harness, selected_is(SHIP_PATH))?;

    assert_eq!(
        selected_path(harness.state()).as_deref(),
        Some(SHIP_PATH),
        "back to the Ship"
    );
    Ok(())
}

fn mouse(harness: &mut Harness<'_, Model>, button: PointerButton) {
    let pos = pos2(800.0, 500.0);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
}

#[test]
fn a_dangling_reference_shows_a_message_instead_of_navigating() -> TestResult {
    let (_dir, mut harness) = window(Language::BritishEnglish)?;
    open_record(&mut harness, "items", "sample")?;

    harness
        .get_by_role_and_label(
            Role::Link,
            "Error: the Reference refers to the Record 70000000-0000-4000-8000-0000000000ff, which does not exist",
        )
        .click();
    harness.run_steps(3);

    assert!(
        harness
            .query_by_label_contains(
                "The Reference points to the Record 70000000-0000-4000-8000-0000000000ff"
            )
            .is_some(),
        "the message"
    );
    assert_eq!(
        selected_path(harness.state()).as_deref(),
        Some(ITEM_PATH),
        "still the Item"
    );
    Ok(())
}

#[test]
fn the_record_tab_speaks_german() -> TestResult {
    let (_dir, mut harness) = window(Language::German)?;
    harness.get_by_label("DataForge aufklappen").click();
    harness.run_steps(1);

    let expand = |harness: &Harness<'_, Model>, name: &str| {
        harness
            .query_by_label(&format!("{name} aufklappen"))
            .is_some()
    };
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| tree.rows().len() > 1)
    })?;
    assert!(expand(&harness, "libs"), "the tree in German");
    harness.get_by_label("libs aufklappen").click();
    harness.run_steps(2);
    harness.get_by_label("items aufklappen").click();
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "sample")
        .click();
    wait_until(&mut harness, read)?;

    for label in ["Record", "Zurück", "Vor", "Feld Wert"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        harness
            .query_by_role_and_label(Role::Row, "nodes: 2 Elemente")
            .is_some(),
        "values in German"
    );
    Ok(())
}
