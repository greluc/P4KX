// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The optional check for a newer Release and the hint it leaves in the status bar.

use crate::waiting::{RunQuiet, wait_for};
use std::error::Error;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ReleaseCheck, ui};

type TestResult = Result<(), Box<dyn Error>>;

type State = (Model, ui::Requests);

const HINT: &str = "Version 99.0.0 is available";
const SWITCH: &str = "Check for a new version at start";

fn serve_newer_release() -> std::io::Result<ReleaseCheck> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/releases/latest", listener.local_addr()?);
    thread::spawn(move || -> std::io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 2 {
            line.clear();
        }
        let body = r#"{"tag_name":"v99.0.0","html_url":"https://example.invalid/v99.0.0"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    });
    Ok(ReleaseCheck::new(&endpoint))
}

fn harness(model: Model) -> Harness<'static, State> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1200.0, 800.0])
        .build_ui_state(
            |ui, (model, requests): &mut State| {
                let frame = ui::show(ui, model);
                requests.settings_changed |= frame.settings_changed;
            },
            (model, ui::Requests::default()),
        )
}

fn wait_for_hint(harness: &mut Harness<'_, State>) -> TestResult {
    wait_for(harness, "a newer Release to arrive", |harness| {
        harness.state().0.newer_release().is_some()
    })?;
    harness.run_quiet();
    Ok(())
}

#[test]
fn a_newer_release_leaves_a_link_in_the_status_bar() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_release_check(serve_newer_release()?);
    model.set_update_check(true);
    model.check_for_update(|| {});
    let mut harness = harness(model);

    wait_for_hint(&mut harness)?;

    assert!(harness.query_by_label(HINT).is_some(), "the hint is shown");
    Ok(())
}

#[test]
fn a_switched_off_check_asks_nobody() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_release_check(ReleaseCheck::new(&format!(
        "http://{}/releases/latest",
        listener.local_addr()?
    )));
    model.check_for_update(|| {});
    let mut harness = harness(model);
    harness.run_quiet();
    thread::sleep(Duration::from_millis(200));
    harness.run_quiet();

    let accepted = listener.accept();

    assert!(
        accepted
            .as_ref()
            .is_err_and(|error| error.kind() == ErrorKind::WouldBlock),
        "no connection: {accepted:?}"
    );
    assert!(harness.query_by_label(HINT).is_none(), "no hint");
    Ok(())
}

#[test]
fn the_help_menu_switches_the_check_and_asks_to_store_it() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_release_check(serve_newer_release()?);
    let mut harness = harness(model);
    harness.run_quiet();

    harness.get_by_label("Help").click();
    harness.run_quiet();
    harness.get_by_label(SWITCH).click();
    wait_for_hint(&mut harness)?;

    assert!(harness.state().0.update_check(), "switched on");
    assert!(harness.state().1.settings_changed, "to be stored");
    assert!(harness.query_by_label(HINT).is_some(), "checked at once");

    harness.state_mut().1 = ui::Requests::default();
    harness.get_by_label("Help").click();
    harness.run_quiet();
    harness.get_by_label(SWITCH).click();
    harness.run_quiet();

    assert!(!harness.state().0.update_check(), "switched off");
    assert!(harness.state().1.settings_changed, "to be stored again");
    assert!(harness.query_by_label(HINT).is_none(), "the hint is gone");
    Ok(())
}
