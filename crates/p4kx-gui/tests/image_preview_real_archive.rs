//! The image preview over the textures of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.
//!
//! The time limits assume a release build.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use p4kx_extraction::{Archive, Texture};
use p4kx_gui::{
    Browser, EntryIndex, ImagePreview, ImageStatus, Language, MipStatus, Model, Preview,
    PreviewTab, ui,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: usize = 400;
const VIEW: [u32; 2] = [512, 512];
const MAX_SIDE: u32 = 16_384;
const WAIT: Duration = Duration::from_mins(1);
const FRAME_LIMIT: Duration = Duration::from_millis(50);
const WINDOW_MAX_SIDE: u32 = 2048;

fn sampled_headers(index: &EntryIndex) -> Result<Vec<u32>, Box<dyn Error>> {
    let headers: Vec<u32> = (0..u32::try_from(index.len())?)
        .filter(|&id| {
            index
                .entry(id)
                .is_some_and(|entry| Texture::is_header_path(entry.path()))
        })
        .collect();
    let step = headers.len().div_ceil(SAMPLES).max(1);
    Ok(headers.into_iter().step_by(step).collect())
}

fn image(browser: &Browser) -> Option<&ImagePreview> {
    browser.preview().and_then(Preview::image)
}

fn wait(browser: &mut Browser, done: impl Fn(&ImagePreview) -> bool) -> TestResult {
    let deadline = Instant::now() + WAIT;
    while !image(browser).is_some_and(&done) {
        if Instant::now() > deadline {
            return Err("the image preview did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(2));
        browser.poll();
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_texture_of_every_format_in_a_real_archive_previews_without_a_crash() -> TestResult {
    let index = EntryIndex::build(Archive::open(real_archive_path()?)?);
    let samples = sampled_headers(&index)?;
    let mut browser = Browser::new(index);
    let mut formats: BTreeMap<String, (u32, bool)> = BTreeMap::new();

    for id in samples {
        browser.select_entry(id, || {});
        wait(&mut browser, |image| !image.is_busy())?;
        let Some(ImageStatus::Opened(info, _)) = image(&browser).map(ImagePreview::status) else {
            continue;
        };
        if formats.contains_key(info.format()) {
            continue;
        }
        let format = info.format().to_owned();
        let level = info.fitting_level(VIEW, MAX_SIDE);
        if let (Some(level), Some(preview)) = (level, browser.image_mut()) {
            preview.request(level, MAX_SIDE);
        }
        wait(&mut browser, |image| !image.is_busy())?;
        let decoded = match image(&browser).map(ImagePreview::status) {
            Some(ImageStatus::Opened(_, MipStatus::Decoded(..))) => true,
            Some(
                ImageStatus::Opened(_, MipStatus::Failed(_, None)) | ImageStatus::Failed(None),
            ) => {
                return Err(format!("decoding Entry {id} in {format} stopped").into());
            }
            _ => false,
        };
        formats.insert(format, (id, decoded));
    }

    assert!(
        formats.values().any(|&(_, decoded)| decoded),
        "at least one format decodes: {formats:?}"
    );
    for name in ["BC1_UNORM", "BC3_UNORM", "BC5_UNORM", "BC7_UNORM"] {
        if let Some(&(id, decoded)) = formats.get(name) {
            assert!(decoded, "{name} (Entry {id}) decodes");
        }
    }
    Ok(())
}

/// Returns the sampled texture with the most pixels whose largest mip the test window can show.
fn largest_showable() -> Result<u32, Box<dyn Error>> {
    let index = EntryIndex::build(Archive::open(real_archive_path()?)?);
    let samples = sampled_headers(&index)?;
    let mut browser = Browser::new(index);
    let mut largest = None;
    for id in samples {
        browser.select_entry(id, || {});
        wait(&mut browser, |image| !image.is_busy())?;
        if let Some(ImageStatus::Opened(info, _)) = image(&browser).map(ImagePreview::status)
            && info.is_decodable()
            && let Some([width, height]) = info.image_size(0)
            && width.max(height) <= WINDOW_MAX_SIDE
        {
            let pixels = u64::from(width) * u64::from(height);
            if largest.is_none_or(|(_, most)| pixels > most) {
                largest = Some((id, pixels));
            }
        }
    }
    largest
        .map(|(id, _)| id)
        .ok_or_else(|| "no decodable texture".into())
}

fn opened_window() -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(real_archive_path()?, || {});
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    let deadline = Instant::now() + WAIT;
    while harness.state().browser().is_none() {
        if Instant::now() > deadline {
            return Err("the Archive did not open in time".into());
        }
        std::thread::sleep(Duration::from_millis(20));
        harness.step();
    }
    Ok(harness)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn frames_stay_short_while_a_large_real_texture_decodes() -> TestResult {
    let id = largest_showable()?;
    let mut harness = opened_window()?;
    let browser = harness.state_mut().browser_mut().ok_or("not opened")?;
    browser.select_entry(id, || {});
    wait(browser, |image| !image.is_busy())?;
    if let Some(image) = browser.image_mut() {
        image.request(0, WINDOW_MAX_SIDE);
    }
    harness.state_mut().set_preview_tab(PreviewTab::Image);

    let mut slowest = Duration::ZERO;
    let deadline = Instant::now() + WAIT;
    while harness
        .state()
        .browser()
        .and_then(image)
        .is_some_and(ImagePreview::is_busy)
    {
        assert!(Instant::now() < deadline, "decoding did not finish in time");
        let started = Instant::now();
        harness.step();
        slowest = slowest.max(started.elapsed());
        std::thread::sleep(Duration::from_millis(5));
    }
    harness.step();

    let shown = harness
        .state()
        .browser()
        .and_then(image)
        .map(ImagePreview::status);
    assert!(
        matches!(
            shown,
            Some(ImageStatus::Opened(_, MipStatus::Decoded(0, Some(_))))
        ),
        "the largest mip of Entry {id} is shown: {shown:?}"
    );
    assert!(slowest < FRAME_LIMIT, "the slowest frame took {slowest:?}");
    Ok(())
}
