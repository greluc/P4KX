//! The Record preview over a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, Language, LineValue, Model, RecordNodeKind, RecordPreviewStatus, RecordTreeStatus,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SHIP: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(600);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(10));
        model.poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

fn browser(model: &mut Model) -> Result<&mut Browser, Box<dyn Error>> {
    model.browser_mut().ok_or_else(|| "no Archive".into())
}

fn preview_read(model: &mut Model) -> TestResult {
    browser(model)?.load_record_preview(|| {});
    wait_until(model, |model| {
        model
            .browser()
            .and_then(Browser::record_preview)
            .is_some_and(|preview| !matches!(preview.status(), RecordPreviewStatus::Loading))
    })
}

fn selected_path(model: &Model) -> Option<String> {
    let browser = model.browser()?;
    Some(
        browser
            .records()?
            .record(browser.selected_record()?)?
            .record_path()
            .to_owned(),
    )
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_ship_is_previewed_and_its_references_are_followed_and_back() -> TestResult {
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    browser(&mut model)?.load_records(|| {});
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
    browser(&mut model)?.select_record_node(node, || {});

    let started = Instant::now();
    preview_read(&mut model)?;
    let took = started.elapsed();
    let preview = model
        .browser()
        .and_then(Browser::record_preview)
        .ok_or("no preview")?;
    let RecordPreviewStatus::Ready(view) = preview.status() else {
        return Err(format!("not read: {:?}", preview.status()).into());
    };
    let link = view
        .lines()
        .iter()
        .find_map(|line| match line.value() {
            LineValue::Reference(link) if link.target().is_some() && link.at().is_none() => {
                Some(link.clone())
            }
            _ => None,
        })
        .ok_or("no Reference to another Record")?;
    let lines = view.lines().len();
    browser(&mut model)?.follow(&link);
    preview_read(&mut model)?;
    let followed = selected_path(&model);
    browser(&mut model)?.go_back();
    let back = selected_path(&model);
    let reads = model.browser().map(Browser::dataforge_reads);

    assert!(lines > 100, "a ship has many fields: {lines}");
    assert!(took < Duration::from_secs(30), "read in {took:?}");
    assert_eq!(followed.as_deref(), Some(link.record_path()), "the target");
    assert_eq!(back.as_deref(), Some(SHIP), "back at the ship");
    assert_eq!(reads, Some(1), "DataForge read once");
    Ok(())
}
