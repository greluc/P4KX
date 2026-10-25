// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Details of the Characters tab: the facts bar at a narrow width and the status row that is only
//! there while it has something to say.

use egui::epaint::Shape;
use egui::{Rect, accesskit::Role};
use egui_kittest::kittest::Queryable;
use p4kx_gui::Language;

use crate::characters::{TestResult, entries, opened_sized, settle};

fn text_rights(shape: &Shape, found: &mut Vec<(f32, f32)>) {
    match shape {
        Shape::Text(text) => {
            let rect = text.galley.rect.translate(text.pos.to_vec2());
            found.push((rect.center().y, rect.right()));
        }
        Shape::Vec(shapes) => shapes.iter().for_each(|one| text_rights(one, found)),
        _ => {}
    }
}

#[test]
fn the_facts_bar_cuts_what_does_not_fit_instead_of_overflowing() -> TestResult {
    for language in [Language::BritishEnglish, Language::German] {
        let (_dir, mut harness) = opened_sized(entries(), language, [300.0, 600.0])?;
        harness.run_steps(3);

        let bar = harness.get_by_label_contains("Vertices").rect();

        let mut rights = Vec::new();
        for clipped in &harness.output().shapes {
            text_rights(&clipped.shape, &mut rights);
        }
        let inside: Vec<f32> = rights
            .iter()
            .filter(|(y, _)| *y > bar.top() && *y < bar.bottom())
            .map(|(_, right)| *right)
            .collect();
        assert!(!inside.is_empty(), "the facts are painted in {language:?}");
        assert!(
            inside.iter().all(|right| *right <= bar.right()),
            "nothing leaves the bar {bar:?} in {language:?}: {inside:?}"
        );
    }
    Ok(())
}

fn first_content_gap(harness: &egui_kittest::Harness<'_, crate::characters::State>) -> f32 {
    let bar: Rect = harness.get_by_label_contains("Vertices").rect();
    let first = harness
        .query_all_by_role(Role::Label)
        .map(|node| node.rect())
        .filter(|rect| rect.top() >= bar.bottom())
        .map(|rect| rect.top())
        .fold(f32::INFINITY, f32::min);
    first - bar.bottom()
}

#[test]
fn the_status_row_is_only_there_while_it_has_something_to_say() -> TestResult {
    let (_dir, mut harness) = opened_sized(entries(), Language::BritishEnglish, [1280.0, 800.0])?;
    harness.run_steps(3);
    let idle = first_content_gap(&harness);

    let tab = harness.state_mut().model.characters_mut();
    tab.set_lod(1);
    tab.cancel();
    harness.run_steps(3);
    let stopped = first_content_gap(&harness);
    assert!(
        harness.state().model.characters().is_cancelled(),
        "stopped by the user"
    );

    harness.state_mut().model.characters_mut().restart();
    settle(&mut harness)?;
    harness.run_steps(3);
    let again = first_content_gap(&harness);

    assert!(
        stopped > idle + 5.0,
        "the row is reserved while stopped: idle {idle}, stopped {stopped}"
    );
    assert!(
        (again - idle).abs() < 1.0,
        "and gone again when idle: idle {idle}, again {again}"
    );
    Ok(())
}
