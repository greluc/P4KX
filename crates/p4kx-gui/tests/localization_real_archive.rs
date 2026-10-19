//! The localisation tab over a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names
//! one.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{Language, LocalizationStatus, Model, TextLanguage};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(600);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(10));
        model.poll();
        model.localization_mut().poll();
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_texts_of_a_real_archive_are_listed_and_searched() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;

    if !model.load_localization(|| {}) {
        return Err("the texts did not start loading".into());
    }
    wait_until(&mut model, |model| {
        !matches!(
            model.localization().status(),
            LocalizationStatus::Idle | LocalizationStatus::Loading
        )
    })?;
    let LocalizationStatus::Ready(table) = model.localization().status() else {
        return Err(format!("not read: {:?}", model.localization().status()).into());
    };
    assert!(
        table.languages().contains(&TextLanguage::English),
        "English is read"
    );
    assert!(table.len() > 10_000, "many keys: {}", table.len());
    let total = table.len();

    let started = Instant::now();
    model.localization_mut().set_query("aurora", || {});
    wait_until(&mut model, |model| !model.localization().is_searching())?;

    let found = model.localization().shown_len();
    assert!(found > 0 && found < total, "{found} of {total} match");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "searched the full table in {:?}",
        started.elapsed()
    );
    Ok(())
}
