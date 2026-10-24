// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The skinned model and its skeleton drawn by wgpu, rendered headless by `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::model_viewer_playback::site;
use crate::rendering::{Window, pixel, rendered, view_pixels, window_at};
use crate::viewing::{TestResult, model_preview, select};

use egui_kittest::kittest::Queryable;
use p4kx_gui::theme::palette::PRIMARY;
use p4kx_gui::{Language, ModelPreview};

type Image = (u32, u32, Vec<u8>);

fn skinned(language: Language) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some(mut window) = window_at(language, site())? else {
        return Ok(None);
    };
    select(&mut window.harness, "arm.skin")?;
    window.harness.run_steps(3);
    let [_, top, _, bottom] = view_pixels(&window.harness);
    if bottom - top < 150 {
        return Err(format!("the view is {} points high, below 150", bottom - top).into());
    }
    Ok(Some(window))
}

fn differing(first: &Image, second: &Image, view: [u32; 4]) -> usize {
    let [left, top, right, bottom] = view;
    (top..bottom)
        .flat_map(|row| (left..right).map(move |column| (column, row)))
        .filter(|&(column, row)| {
            pixel(&first.2, first.0, column, row) != pixel(&second.2, second.0, column, row)
        })
        .count()
}

fn orange(image: &Image, view: [u32; 4]) -> usize {
    let [left, top, right, bottom] = view;
    let near = |value: u8, want: u8| value.abs_diff(want) <= 8;
    (top..bottom)
        .flat_map(|row| (left..right).map(move |column| (column, row)))
        .filter(|&(column, row)| {
            let [red, green, blue] = pixel(&image.2, image.0, column, row);
            near(red, PRIMARY.r()) && near(green, PRIMARY.g()) && near(blue, PRIMARY.b())
        })
        .count()
}

#[test]
fn the_pose_of_an_animation_changes_the_picture_and_a_pause_keeps_it() -> TestResult {
    let Some(mut window) = skinned(Language::BritishEnglish)? else {
        return Ok(());
    };
    let bound = rendered(&mut window, "skin-bind-pose.png")?;
    let view = view_pixels(&window.harness);

    window.harness.get_by_label("slow").click();
    window.harness.run_steps(2);
    window.harness.get_by_label("Pause").click();
    window.harness.run_steps(2);
    window
        .harness
        .state_mut()
        .viewer_playback_mut()
        .ok_or("no playback")?
        .apply(p4kx_gui::Control::Seek(1.0));
    window.harness.run_steps(2);
    let posed = rendered(&mut window, "skin-posed.png")?;

    assert!(
        differing(&bound, &posed, view) > 200,
        "the skinned tip moved on the GPU: {} pixels differ",
        differing(&bound, &posed, view)
    );
    let passes = window.report.passes();
    for _ in 0..5 {
        rendered(&mut window, "skin-held.png")?;
    }
    assert_eq!(
        window.report.passes(),
        passes,
        "a held pose is not drawn again"
    );
    Ok(())
}

#[test]
fn a_playing_animation_draws_a_new_picture_every_frame() -> TestResult {
    let Some(mut window) = skinned(Language::BritishEnglish)? else {
        return Ok(());
    };
    window.harness.get_by_label("slow").click();
    window.harness.run_steps(2);
    let passes = window.report.passes();

    for round in 0..5 {
        rendered(&mut window, &format!("skin-playing-{round}.png"))?;
    }

    assert!(
        window.report.passes() >= passes + 4,
        "drawn while playing: {} then {}",
        passes,
        window.report.passes()
    );
    assert_eq!(window.report.uploads(), 1, "the mesh is uploaded once");
    Ok(())
}

#[test]
fn the_skeleton_is_drawn_over_the_model_when_switched_on() -> TestResult {
    for (language, name, label) in [
        (
            Language::BritishEnglish,
            "skin-skeleton-en-GB.png",
            "Skeleton",
        ),
        (Language::German, "skin-skeleton-de-DE.png", "Skelett"),
    ] {
        let Some(mut window) = skinned(language)? else {
            return Ok(());
        };
        let without = rendered(&mut window, "skin-without-skeleton.png")?;
        let view = view_pixels(&window.harness);
        assert_eq!(orange(&without, view), 0, "no overlay yet in {view:?}");

        window.harness.get_by_label(label).click();
        window.harness.run_steps(2);
        let with = rendered(&mut window, name)?;

        assert!(
            orange(&with, view) > 8,
            "the bone is drawn in the accent colour: {} pixels",
            orange(&with, view)
        );
        window.harness.get_by_label(label).click();
        window.harness.run_steps(2);
        let hidden = rendered(&mut window, "skin-skeleton-hidden.png")?;
        assert_eq!(orange(&hidden, view), 0, "gone again");
    }
    Ok(())
}

#[test]
fn the_skeleton_buffers_are_released_with_the_model() -> TestResult {
    let Some(mut window) = skinned(Language::BritishEnglish)? else {
        return Ok(());
    };
    rendered(&mut window, "skin-buffers.png")?;
    assert_eq!(
        window.report.buffers(),
        6,
        "vertices, indices, bone matrices, bones, overlay look and weights"
    );

    select(&mut window.harness, "cube.cgf")?;
    rendered(&mut window, "cube-after-skin.png")?;

    assert_eq!(
        window.report.buffers(),
        2,
        "nothing is left of the skeleton"
    );
    let preview = model_preview(window.harness.state());
    assert!(
        preview.is_some_and(|preview| ModelPreview::playback(preview).is_none()),
        "and the cube has no playback"
    );
    Ok(())
}

#[test]
fn a_static_model_with_bones_draws_its_skeleton_without_weights() -> TestResult {
    let Some(mut window) = window_at(Language::BritishEnglish, site())? else {
        return Ok(());
    };
    select(&mut window.harness, "door.cga")?;
    window.harness.run_steps(3);
    let view = view_pixels(&window.harness);
    window.harness.get_by_label("Skeleton").click();
    window.harness.run_steps(2);

    let image = rendered(&mut window, "door-skeleton.png")?;

    assert_eq!(
        window.report.buffers(),
        5,
        "no weights to upload for a model that is not skinned"
    );
    assert!(orange(&image, view) > 3, "the helper bones are drawn");
    Ok(())
}
