//! The language of the texts in a real Record preview; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, KeyText, Language, LineValue, Model, RecordNodeKind, RecordPreviewStatus,
    RecordTextChoice, RecordTreeStatus, RecordView, TextLanguage,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SHIP: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(600);
    while !done(model) {
        if Instant::now() > deadline {
            return Err(format!(
                "not settled; wanted {:?}, view {:?}",
                model.record_text_language(),
                view(model).map(RecordView::language)
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(10));
        model.poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

fn view(model: &Model) -> Option<&RecordView> {
    match model.browser()?.record_preview()?.status() {
        RecordPreviewStatus::Ready(view) => Some(view),
        _ => None,
    }
}

fn read_in(language: Option<TextLanguage>) -> impl Fn(&Model) -> bool {
    move |model| view(model).is_some_and(|view| view.language() == language)
}

fn switch(model: &mut Model, choice: RecordTextChoice) -> TestResult {
    model.set_record_text_choice(Some(choice));
    model
        .browser_mut()
        .ok_or("no Archive")?
        .load_record_preview(|| {});
    Ok(())
}

fn key_texts(view: &RecordView) -> Vec<(&str, &KeyText)> {
    view.lines()
        .iter()
        .filter_map(|line| match line.value() {
            LineValue::Key { key, text } => Some((&**key, text)),
            _ => None,
        })
        .collect()
}

fn count(view: &RecordView, wanted: impl Fn(&KeyText) -> bool) -> usize {
    key_texts(view)
        .into_iter()
        .filter(|(_, text)| wanted(text))
        .count()
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario over a real Archive")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_ship_shows_its_texts_in_the_chosen_language_without_reading_again() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    let languages = model.record_text_languages().to_vec();
    assert!(languages.contains(&TextLanguage::English), "English");
    assert!(languages.contains(&TextLanguage::German), "German");

    model.browser_mut().ok_or("no Archive")?.load_records(|| {});
    wait_until(&mut model, |model| {
        model
            .browser()
            .and_then(Browser::records)
            .is_some_and(|tree| !matches!(tree.status(), RecordTreeStatus::Loading))
    })?;
    let tree = model
        .browser()
        .and_then(Browser::records)
        .ok_or("no DataForge")?;
    let mut node = tree.find(SHIP).ok_or("no ship")?;
    if tree.node(node).kind() == RecordNodeKind::RecordFolder {
        node = *tree
            .node(node)
            .children()
            .first()
            .ok_or("an empty record folder")?;
    }
    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.select_record_node(node, || {});
    browser.load_record_preview(|| {});
    wait_until(&mut model, read_in(Some(TextLanguage::English)))?;

    let english = view(&model).ok_or("no view")?;
    let english_keys = key_texts(english).len();
    let english_texts = count(english, |text| matches!(text, KeyText::Text { .. }));
    assert!(english_texts > 0, "the ship has keys with English texts");
    assert_eq!(
        count(english, |text| matches!(text, KeyText::Missing)),
        0,
        "every key of the ship has an English text"
    );

    switch(&mut model, RecordTextChoice::Language(TextLanguage::German))?;
    wait_until(&mut model, read_in(Some(TextLanguage::German)))?;
    let german = view(&model).ok_or("no view")?;
    assert_eq!(key_texts(german).len(), english_keys, "the same keys");
    assert!(
        count(german, |text| matches!(text, KeyText::Text { .. })) > 0,
        "German texts are shown"
    );

    switch(&mut model, RecordTextChoice::KeysOnly)?;
    wait_until(&mut model, read_in(None))?;
    let keys_only = view(&model).ok_or("no view")?;
    assert_eq!(
        count(keys_only, |text| matches!(text, KeyText::None)),
        key_texts(keys_only).len(),
        "keys alone"
    );
    assert_eq!(
        model.browser().map(Browser::dataforge_reads),
        Some(1),
        "DataForge was read once for three languages"
    );
    Ok(())
}
