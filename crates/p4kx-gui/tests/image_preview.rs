//! The Image tab of the preview over a synthetic Archive with split textures, driven headless.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Browser, ImagePreview, ImageStatus, Language, MipStatus, Model, Preview, PreviewTab, ui,
};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const PANEL: &str = "Data\\Textures\\panel_diff.dds";
const PROBE: &str = "Data\\Textures\\probe_cm.dds";
const ODD: &str = "Data\\Textures\\odd_spec.dds";
const LOST: &str = "Data\\Textures\\lost_ddna.dds";
const R11G11B10_FLOAT: TestTextureFormat = TestTextureFormat::Dx10Pixel {
    dxgi_format: 26,
    pixel_bytes: 4,
};

fn archive() -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    let mut entries = SplitDds::new(TestTextureFormat::LegacyBgra8, 1024, 1024)
        .split(4)
        .entries(PANEL);
    entries.extend(
        SplitDds::new(TestTextureFormat::LegacyBgra8, 16, 16)
            .cubemap()
            .split(1)
            .entries(PROBE),
    );
    entries.extend(SplitDds::new(R11G11B10_FLOAT, 8, 8).entries(ODD));
    let mut lost = SplitDds::new(TestTextureFormat::LegacyBgra8, 64, 64)
        .split(7)
        .entries(LOST);
    lost.truncate(1);
    entries.extend(lost);
    entries.push(TestEntry::stored("Data\\readme.txt", "plain text"));
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    Ok((folder, path))
}

fn opened(language: Language) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let (folder, path) = archive()?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((folder, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run();
    Ok(())
}

fn image(model: &Model) -> Option<&ImagePreview> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::image)
}

fn settled(model: &Model) -> bool {
    image(model).is_some_and(|image| match image.status() {
        ImageStatus::Opened(info, MipStatus::Unchosen) => !info.is_decodable(),
        ImageStatus::Opening
        | ImageStatus::Opened(_, MipStatus::Decoding(_) | MipStatus::Decoded(_, None)) => false,
        _ => true,
    })
}

fn select(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, settled)
}

fn shown_level(harness: &Harness<'_, Model>) -> Option<u32> {
    match image(harness.state())?.status() {
        ImageStatus::Opened(_, MipStatus::Decoded(level, Some(_))) => Some(level),
        _ => None,
    }
}

#[test]
fn selecting_a_texture_shows_the_largest_mip_that_fits_the_view() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "panel_diff.dds")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Image,
        "Image tab"
    );
    let level = shown_level(&harness).ok_or("no mip shown")?;
    let side = 1024_f32 / 2_f32.powi(i32::try_from(level)?);
    let view = harness
        .get_by_label(&format!("Mip {level}: {side} × {side} pixels"))
        .rect();
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Image, &format!("Mip {level}: {side} × {side} pixels"))
            .rect(),
        view,
        "the mip is shown as an image"
    );
    assert!(
        side <= view.width() && side <= view.height(),
        "{side} fits into {view:?}"
    );
    assert!(
        level == 0 || side * 2.0 > view.width().min(view.height()),
        "the next larger mip would not fit into {view:?}"
    );
    Ok(())
}

#[test]
fn the_mip_selector_switches_levels() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;

    harness.get_by_label("1024").click();
    wait_until(&mut harness, |model| {
        settled(model) && image(model).and_then(ImagePreview::level) == Some(0)
    })?;

    assert_eq!(shown_level(&harness), Some(0), "largest mip");
    assert!(
        harness
            .query_by_label("Mip 0: 1024 × 1024 pixels")
            .is_some(),
        "shown"
    );

    harness.get_by_label("2").click();
    wait_until(&mut harness, |model| {
        settled(model) && image(model).and_then(ImagePreview::level) == Some(9)
    })?;

    assert!(
        harness.query_by_label("Mip 9: 2 × 2 pixels").is_some(),
        "a small mip from the header"
    );
    Ok(())
}

#[test]
fn the_info_shows_format_size_mips_and_cubemap_faces() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "panel_diff.dds")?;
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Size 1024 × 1024 Mips 11 Cubemap faces –")
            .is_some(),
        "plain texture"
    );

    select(&mut harness, "probe_cm.dds")?;
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Size 16 × 16 Mips 5 Cubemap faces 6")
            .is_some(),
        "cubemap"
    );
    assert!(
        harness.query_by_label("Mip 0: 64 × 48 pixels").is_some(),
        "the faces are shown as a cross"
    );
    Ok(())
}

#[test]
fn an_unsupported_format_is_named_and_the_text_and_hex_tabs_still_work() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "odd_spec.dds")?;

    assert!(
        harness
            .query_by_label(
                "The preview cannot decode R11G11B10_FLOAT textures. The Text and Hex tabs \
                 still show the data of Data\\Textures\\odd_spec.dds."
            )
            .is_some(),
        "message"
    );
    harness.get_by_label("Hex").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("00000000  44 44 53 20")
            .is_some(),
        "hex shows the DDS magic"
    );
    harness.get_by_label("Text").click();
    harness.run();
    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    Ok(())
}

#[test]
fn a_mip_that_cannot_be_read_names_the_reason() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;

    select(&mut harness, "lost_ddna.dds")?;

    assert!(
        harness
            .query_by_label_contains("Cannot decode Data\\Textures\\lost_ddna.dds: ")
            .is_some(),
        "failure"
    );
    Ok(())
}

#[test]
fn selecting_another_entry_leaves_the_image_tab() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;

    harness.get_by_label("readme.txt").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    assert!(harness.query_by_label("plain text").is_some(), "text shown");
    assert!(harness.query_by_label("Image").is_none(), "no Image tab");
    Ok(())
}

#[test]
fn decoding_runs_off_the_window_thread() -> TestResult {
    let (folder, path) = archive()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    let browser = model.browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "panel_diff.dds")
        .ok_or("no texture")?;

    browser.select_entry(id, || {});
    let opening = browser.image_mut().map(|image| image.is_busy());
    while browser
        .image_mut()
        .is_some_and(|image| matches!(image.status(), ImageStatus::Opening))
    {
        std::thread::sleep(Duration::from_millis(5));
        browser.poll();
    }
    let image = browser.image_mut().ok_or("no image preview")?;
    image.request(0);
    let decoding = matches!(
        image.status(),
        ImageStatus::Opened(_, MipStatus::Decoding(0))
    );

    assert_eq!(opening, Some(true), "the header is read in the background");
    assert!(decoding, "the request returns while the mip is decoded");
    drop(folder);
    Ok(())
}

#[test]
fn the_german_image_tab_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened(Language::German)?;

    select(&mut harness, "panel_diff.dds")?;

    assert!(harness.query_by_label("Bild").is_some(), "tab");
    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Größe 1024 × 1024 Mips 11 Cubemap-Seiten –")
            .is_some(),
        "facts"
    );
    Ok(())
}
