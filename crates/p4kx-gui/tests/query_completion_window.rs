//! Completion in the Query editor, driven headless over a synthetic Archive with a DataForge.

#[expect(
    dead_code,
    reason = "each test file uses a part of the shared Query fixtures"
)]
mod querying;

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, WorkTab, ui};
use p4kx_testkit::DataForgeBuilder;
use querying::{TestResult, archive, database, referencing};
use tempfile::TempDir;

fn opened(forge: &DataForgeBuilder) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), forge)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                let _requests = ui::show(ui, model);
            },
            Model::new(Language::BritishEnglish),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Query);
    harness.run_steps(2);
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
    harness.run_steps(3);
    Ok(())
}

fn shown(model: &Model) -> Vec<String> {
    model
        .query()
        .completion_shown()
        .map(|(completion, _)| {
            completion
                .candidates()
                .iter()
                .map(|candidate| candidate.name().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn type_text(harness: &mut Harness<'_, Model>, text: &str) {
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.run_steps(2);
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(text);
    harness.run_steps(2);
}

#[test]
fn typing_after_select_lists_the_fields_of_the_from_type() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;

    type_text(&mut harness, "from Ship select ");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    assert_eq!(
        shown(harness.state()),
        ["crew", "mass", "name"],
        "inherited too"
    );
    for name in ["crew", "mass", "name"] {
        assert!(
            harness
                .query_by_role_and_label(Role::ListBoxOption, name)
                .is_some(),
            "option {name}"
        );
    }
    Ok(())
}

#[test]
fn the_arrow_keys_choose_and_enter_writes_the_name() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;
    type_text(&mut harness, "from Ship select ");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);

    assert_eq!(
        harness.state().query().text(),
        "from Ship select mass",
        "written, no line break"
    );
    assert!(
        harness.state().query().completion_shown().is_none(),
        "closed"
    );
    Ok(())
}

#[test]
fn a_type_name_is_completed_by_a_click_and_typing_goes_on() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;
    type_text(&mut harness, "from Sh");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    assert_eq!(
        shown(harness.state()),
        ["Ship"],
        "filtered by the typed word"
    );
    harness
        .get_by_role_and_label(Role::ListBoxOption, "Ship")
        .click();
    harness.run_steps(3);
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(" select");
    harness.run_steps(2);

    assert_eq!(
        harness.state().query().text(),
        "from Ship select",
        "replaced the word and kept the focus"
    );
    Ok(())
}

#[test]
fn escape_closes_the_list_and_ctrl_space_opens_it_again() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;
    type_text(&mut harness, "from ");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    harness.key_press(Key::Escape);
    harness.run_steps(2);
    assert!(shown(harness.state()).is_empty(), "closed");
    assert_eq!(harness.state().query().text(), "from ", "text kept");

    harness.key_press_modifiers(Modifiers::COMMAND, Key::Space);
    harness.run_steps(3);
    assert_eq!(shown(harness.state()), ["Item", "Ship"], "open again");
    Ok(())
}

#[test]
fn an_arrow_lists_the_fields_of_the_linked_struct() -> TestResult {
    let (_dir, mut harness) = opened(&referencing())?;

    type_text(&mut harness, "from Ship select maker->");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    assert_eq!(shown(harness.state()), ["name"], "Maker fields");
    Ok(())
}

#[test]
fn brackets_list_the_element_struct_and_its_subtypes() -> TestResult {
    let forge = DataForgeBuilder::new()
        .structure(
            p4kx_testkit::TestStruct::new("Part").field("mass", p4kx_testkit::TestType::Single),
        )
        .structure(
            p4kx_testkit::TestStruct::new("Engine")
                .parent("Part")
                .field("thrust", p4kx_testkit::TestType::Single),
        )
        .structure(p4kx_testkit::TestStruct::new("Ship").array(
            "Components",
            p4kx_testkit::TestType::StrongPointer("Part".to_owned()),
        ));
    let (_dir, mut harness) = opened(&forge)?;

    type_text(&mut harness, "from Ship select Components[");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    assert_eq!(shown(harness.state()), ["Engine", "Part"], "subtypes");
    Ok(())
}

#[test]
fn nothing_is_listed_where_no_name_fits() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;
    type_text(&mut harness, "from ");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;

    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text("Ship at \"lib");
    harness.run_steps(3);

    assert!(shown(harness.state()).is_empty(), "inside a string");
    Ok(())
}

#[test]
fn enter_breaks_the_line_when_the_chosen_name_is_written_already() -> TestResult {
    let (_dir, mut harness) = opened(&database())?;
    type_text(&mut harness, "from Ship select ");
    wait_until(&mut harness, |model| !shown(model).is_empty())?;
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text("mass");
    harness.run_steps(3);
    assert_eq!(shown(harness.state()), ["mass"], "still offered");

    harness.key_press(Key::Enter);
    harness.run_steps(3);

    assert_eq!(
        harness.state().query().text(),
        "from Ship select mass\n",
        "a line break, not the name again"
    );
    Ok(())
}
