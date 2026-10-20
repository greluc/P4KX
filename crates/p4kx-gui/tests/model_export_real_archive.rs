//! The Info tab and Export as GLB over models of a real Archive: the info is read and a sample of
//! models exports as GLB files that pass the checker.
//!
//! Models that need the Archive key are skipped and counted; the tests never try to obtain the key.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, Language, Model, ModelExport, ModelExportFailure, ModelExportStatus, ModelPreview,
    ModelStatus, Preview, is_model_path,
};
use p4kx_testkit::{check_glb, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: usize = 16;
const LOAD_LIMIT: Duration = Duration::from_secs(120);

fn opened(path: PathBuf) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(600);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("the Archive did not open in time".into());
        }
        std::thread::sleep(Duration::from_millis(20));
        model.poll();
    }
    Ok(model)
}

fn sample(browser: &Browser) -> Vec<u32> {
    let index = browser.index();
    let all: Vec<u32> = (0..u32::try_from(index.len()).unwrap_or(u32::MAX))
        .filter(|&id| {
            index.entry(id).is_some_and(|entry| {
                let path = entry.path().to_ascii_lowercase();
                is_model_path(&path) && entry.uncompressed_size() < 8 * 1024 * 1024
            })
        })
        .collect();
    let stride = (all.len() / SAMPLE).max(1);
    all.into_iter().step_by(stride).take(SAMPLE).collect()
}

fn select(model: &mut Model, id: u32) -> TestResult {
    let browser = model.browser_mut().ok_or("not opened")?;
    browser.select_entry(id, || {});
    let deadline = Instant::now() + LOAD_LIMIT;
    while browser
        .preview()
        .and_then(Preview::model)
        .is_some_and(ModelPreview::is_loading)
    {
        if Instant::now() > deadline {
            return Err("a model did not load in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        browser.poll();
    }
    Ok(())
}

fn wait(model: &mut Model) -> TestResult {
    let deadline = Instant::now() + LOAD_LIMIT;
    while model.model_export().is_some_and(ModelExport::is_running) {
        if Instant::now() > deadline {
            return Err("an export did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll_model_export();
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_models_list_their_facts_and_export_as_valid_glb_files() -> TestResult {
    let mut model = opened(real_archive_path()?)?;
    let ids = sample(model.browser().ok_or("not opened")?);
    assert!(ids.len() >= SAMPLE / 2, "models were found");
    let folder = tempfile::tempdir()?;
    let (mut exported, mut skipped_key, mut unreadable) = (0, 0, 0);

    for (position, id) in ids.into_iter().enumerate() {
        select(&mut model, id)?;
        let readable = model
            .browser()
            .and_then(Browser::preview)
            .and_then(Preview::model)
            .map(ModelPreview::status);
        if !matches!(
            readable,
            Some(ModelStatus::Ready(_) | ModelStatus::Unshowable(_))
        ) {
            unreadable += 1;
            continue;
        }
        let info = model
            .browser()
            .and_then(Browser::preview)
            .and_then(Preview::model)
            .and_then(ModelPreview::info)
            .ok_or("a model that was read has info")?;
        assert_eq!(
            info.chunks.total,
            info.chunks.read + info.chunks.skipped + info.chunks.unknown,
            "every chunk is counted once"
        );
        let file = folder.path().join(format!("model-{position}.glb"));
        model.begin_model_export();
        let setup = model.model_export_setup_mut().ok_or("no setup")?;
        setup.set_file(file.clone());
        assert!(model.submit_model_export(|| {}), "started");
        wait(&mut model)?;
        match model.model_export().map(ModelExport::status) {
            Some(ModelExportStatus::Finished(summary)) => {
                let bytes = std::fs::read(&file)?;
                check_glb(&bytes)?;
                assert_eq!(
                    u64::try_from(bytes.len())?,
                    summary.written().bytes,
                    "the length the writer counted"
                );
                exported += 1;
            }
            Some(ModelExportStatus::Failed(ModelExportFailure::KeyRequired)) => skipped_key += 1,
            other => return Err(format!("an export failed: {other:?}").into()),
        }
        model.close_model_export();
    }

    assert!(
        exported >= SAMPLE / 2,
        "most sampled models export: {exported} exported, {skipped_key} need the Archive key, {unreadable} unreadable"
    );
    Ok(())
}
