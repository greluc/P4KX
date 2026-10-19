//! Helpers of the soundbank tests on top of the synthetic Archive of the sounding helpers.

use std::error::Error;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Browser, Language, Model, SoundbankPreview, SoundbankStatus};
use p4kx_testkit::{TestBank, TestCodec, TestEntry, TestWem, test_signal};
use tempfile::TempDir;

use crate::sounding::{FakeOutput, TestResult, audio, opened_with, wait_until};

/// The soundbanks of the synthetic Archive: rms.bnk with WEMs of IDs 101 to 105, the fourth only a
/// prefetched start and the fifth plug-in media, ig.bnk with 60 short WEMs, an empty and a corrupt bank.
fn banks() -> Vec<TestEntry> {
    let tone = test_signal(48_000, 2);
    let pcm = TestWem::pcm(TestCodec::WwisePcm, 2, 48_000, &tone).bytes();
    let ima = TestWem::ima_adpcm(1, 48_000, &test_signal(640, 1)).bytes();
    let aac = TestWem::encoded(TestCodec::Tag(0xAAC0), 1, 48_000, 960).bytes();
    let prefetch = pcm.get(..64).unwrap_or_default().to_vec();
    let arms = TestBank::new(0x0B)
        .wem(101, pcm)
        .wem(102, ima)
        .wem(103, aac)
        .wem(104, prefetch)
        .wem(105, b"PLUG\x10\x00\x00\x00hash".to_vec())
        .bytes();
    let small = TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(48, 1)).bytes();
    let big = (1..=60).fold(TestBank::new(0x0D), |bank, id| bank.wem(id, small.clone()));
    vec![
        TestEntry::stored("Data\\Sounds\\arms.bnk", arms),
        TestEntry::stored("Data\\Sounds\\big.bnk", big.bytes()),
        TestEntry::stored("Data\\Sounds\\empty.bnk", TestBank::new(0x0C).bytes()),
        TestEntry::stored("Data\\Sounds\\broken.bnk", b"BKHD".to_vec()),
    ]
}

/// Opens the synthetic Archive of the sounds with the soundbanks in a headless window.
pub(crate) fn opened_banks(
    language: Language,
    output: &FakeOutput,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    opened_with(language, output, banks())
}

pub(crate) fn bank(model: &Model) -> Option<&SoundbankPreview> {
    model.browser().and_then(Browser::soundbank)
}

/// Selects the soundbank `name` and waits until it was read.
pub(crate) fn select_bank(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, |model| {
        bank(model).is_some_and(|bank| !matches!(bank.status(), SoundbankStatus::Opening))
    })?;
    for _ in 0..3 {
        harness.step();
    }
    Ok(())
}

/// Waits until the WEM `id` of the soundbank is selected and its sound was decoded.
pub(crate) fn decoded(harness: &mut Harness<'_, Model>, id: u32) -> TestResult {
    wait_until(harness, |model| {
        bank(model).is_some_and(|bank| {
            bank.selected() == Some(id) && bank.audio().is_none_or(|audio| !audio.is_busy())
        })
    })
}
/// Selects the WEM `id` of the selected soundbank as a click on its row would, and waits until its
/// sound was decoded.
pub(crate) fn select_sound(harness: &mut Harness<'_, Model>, id: u32) -> TestResult {
    harness
        .state_mut()
        .browser_mut()
        .ok_or("not opened")?
        .select_sound(id, || {});
    decoded(harness, id)
}

/// Selects the WEM Entry `name` as a click on its row would, and waits until its sound was
/// decoded; the rows below a tall preview are not laid out.
pub(crate) fn select_loose(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == name)
        .ok_or("no such Entry")?;
    browser.select_entry(id, || {});
    wait_until(harness, |model| {
        audio(model).is_some_and(|audio| !audio.is_busy())
    })
}
