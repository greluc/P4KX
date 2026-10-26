// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Tags tab over a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{Language, Model, TagsStatus};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(120);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(10));
        model.poll();
        model.tags_mut().poll();
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_tag_tree_of_a_real_archive_is_listed_expanded_and_searched() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;

    if !model.load_tags(|| {}) {
        return Err("the tags did not start loading".into());
    }
    wait_until(&mut model, |model| {
        !matches!(
            model.tags().status(),
            TagsStatus::Idle | TagsStatus::Loading
        )
    })?;
    let TagsStatus::Ready(view) = model.tags().status() else {
        return Err(format!("not read: {:?}", model.tags().status()).into());
    };
    let roots = view.shown_len();
    assert!(
        roots > 10 && roots < 500,
        "only the roots are shown: {roots}"
    );
    assert!(
        view.tree().len() > 10_000,
        "many tags: {}",
        view.tree().len()
    );

    model.tags_mut().set_query("Ship.Series.Avenger");
    let TagsStatus::Ready(view) = model.tags().status() else {
        return Err("not ready".into());
    };
    let shown: Vec<_> = (0..view.shown_len())
        .filter_map(|index| view.row(index))
        .map(|row| row.tag().path().to_owned())
        .collect();
    assert!(
        shown.iter().any(|path| path == "Ship.Series.Avenger"),
        "the match is shown with its ancestors: {shown:?}"
    );
    assert!(
        shown.iter().any(|path| path == "Ship.Series"),
        "the parent is shown: {shown:?}"
    );
    assert!(
        (0..view.shown_len())
            .filter_map(|index| view.row(index))
            .all(|row| !row.is_expandable()),
        "no row shows a chevron while the search filters"
    );
    Ok(())
}
