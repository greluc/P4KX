//! The limits of the image preview: alpha tails on their own, textures too large to show and
//! levels the window cannot hold.

mod imaging;

use egui_kittest::kittest::Queryable;
use imaging::{TestResult, image, opened, opened_with, select, shown_level};
use p4kx_gui::{ImagePreview, Language, PreviewTab};
use p4kx_testkit::{SplitDds, TestEntry, TestTextureFormat};
const GLASS: &str = "Data\\Textures\\glass_ddna.dds";
const HUGE: &str = "Data\\Textures\\huge_cm.dds";

#[test]
fn an_alpha_tail_opens_in_the_image_tab_on_its_own() -> TestResult {
    let glass = SplitDds::new(TestTextureFormat::LegacyBgra8, 32, 32)
        .split(2)
        .alpha(TestTextureFormat::R8)
        .entries(GLASS);
    let (_folder, mut harness) = opened_with(Language::BritishEnglish, glass)?;

    select(&mut harness, "glass_ddna.dds.a")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Image,
        "Image tab"
    );
    assert!(
        harness
            .query_by_label("Format R8_UNORM Size 32 × 32 Mips 6 Cubemap faces –")
            .is_some(),
        "the tail's own facts"
    );
    assert_eq!(shown_level(&harness), Some(0), "the tail is shown");
    Ok(())
}

#[test]
fn a_texture_too_large_for_the_preview_says_so() -> TestResult {
    let huge = SplitDds::new(TestTextureFormat::LegacyDxt1, 16_384, 16_384)
        .mips(1)
        .split(1);
    let entries = vec![TestEntry::stored(HUGE, huge.header_bytes())];
    let (_folder, mut harness) = opened_with(Language::BritishEnglish, entries)?;

    select(&mut harness, "huge_cm.dds")?;

    assert!(
        harness
            .query_by_label(
                "Every mip of Data\\Textures\\huge_cm.dds is larger than the preview can show."
            )
            .is_some(),
        "message"
    );
    assert_eq!(
        image(harness.state()).and_then(ImagePreview::level),
        None,
        "nothing is decoded"
    );
    Ok(())
}

#[test]
fn a_level_larger_than_the_window_can_show_is_refused() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish)?;
    select(&mut harness, "panel_diff.dds")?;
    let shown = shown_level(&harness);
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    let image = browser.image_mut().ok_or("no image preview")?;

    image.request(0, 512);
    let refused = image.level();

    assert_eq!(refused, shown, "1024 is wider than 512 and nothing changed");
    image.request(1, 512);
    assert_eq!(image.level(), Some(1), "512 is accepted");
    Ok(())
}
