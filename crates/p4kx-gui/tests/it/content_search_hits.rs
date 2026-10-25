// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! What opening a hit of the Search tab does to the preview, and a search that starts right after
//! a cancelled one.

use std::error::Error;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, PreviewStatus, PreviewTab, WorkTab, ui};
use p4kx_search::Source;

use crate::content_search_window::{archive, click_row, finished, save, searched, wait_until};

type TestResult = Result<(), Box<dyn Error>>;

fn preview_ready(model: &Model) -> bool {
    model
        .browser()
        .and_then(p4kx_gui::Browser::preview)
        .is_some_and(|preview| matches!(preview.status(), PreviewStatus::Ready(_)))
}

#[test]
fn a_click_on_an_entry_hit_scrolls_the_preview_to_its_line() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "needleword")?;
    assert!(harness.query_by_label("row 1").is_none(), "no preview yet");

    click_row(&harness, "Data\\Scripts\\long.xml")?;
    wait_until(&mut harness, preview_ready)?;
    harness.run_steps(6);

    assert!(
        harness.query_by_label("row 300 needleword").is_some(),
        "the line of the hit is shown"
    );
    assert!(
        harness.query_by_label("row 1").is_none(),
        "the start of the file is scrolled away"
    );
    Ok(())
}

#[test]
fn a_click_on_a_cryxml_hit_scrolls_the_xml_view_to_its_line() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "xmlneedle")?;

    click_row(&harness, "Data\\Scripts\\wide.xml")?;
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(p4kx_gui::Browser::preview)
            .and_then(p4kx_gui::Preview::xml)
            .is_some_and(|xml| xml.text().is_some())
    })?;
    harness.run_steps(8);

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Xml,
        "the XML tab opened"
    );
    assert!(
        harness
            .query_by_label_contains("row 300 xmlneedle")
            .is_some(),
        "the line of the hit is shown"
    );
    assert!(
        harness.query_by_label_contains("\"row 2\"").is_none(),
        "the start of the document is scrolled away"
    );
    Ok(())
}

#[test]
fn a_search_started_right_after_a_cancelled_one_runs_to_its_end() -> TestResult {
    let (_dir, mut harness) = searched(Language::BritishEnglish, "gladi")?;

    assert!(harness.state_mut().run_content_search(|| {}), "started");
    harness.state_mut().content_search_mut().cancel();
    assert!(
        harness.state_mut().run_content_search(|| {}),
        "started again at once"
    );
    wait_until(&mut harness, finished)?;

    let result = harness
        .state()
        .content_search()
        .result()
        .ok_or("no result after the second search")?;
    assert_eq!(result.len(Source::Records), 2, "the second search finished");
    Ok(())
}

#[test]
#[ignore = "needs a GPU adapter: set P4KX_RENDER_DIR and pass --ignored"]
fn the_preview_scrolled_to_a_hit_renders_at_both_sizes_in_both_languages() -> TestResult {
    for size in [[1280.0, 800.0], [640.0, 400.0]] {
        for (language, suffix) in [
            (Language::BritishEnglish, "en-GB"),
            (Language::German, "de-DE"),
        ] {
            let dir = tempfile::tempdir()?;
            let mut harness = Harness::builder()
                .with_context_setup(p4kx_gui::theme::apply)
                .with_size(size)
                .build_ui_state(
                    |ui, model: &mut Model| {
                        ui::show(ui, model);
                    },
                    Model::new(language),
                );
            harness.state_mut().open(archive(dir.path())?, || {});
            wait_until(&mut harness, |model| model.browser().is_some())?;
            harness.state_mut().set_work_tab(WorkTab::Search);
            harness
                .state_mut()
                .content_search_mut()
                .set_text("needleword");
            assert!(harness.state_mut().run_content_search(|| {}), "started");
            wait_until(&mut harness, finished)?;
            harness.state_mut().open_hit(0, || {});
            wait_until(&mut harness, preview_ready)?;
            harness.run_steps(6);
            let image = harness.render()?;
            let (width, height) = (image.width(), image.height());
            let name = format!("hit-line-{}x{}-{suffix}.png", size[0], size[1]);
            save(&name, width, height, &image.into_raw())?;
        }
    }
    Ok(())
}
