//! The Audio tab of the preview for a soundbank over a synthetic Archive with BNK Entries, driven
//! headless through a fake audio output.

use crate::banking;
use crate::sounding;

use std::time::Duration;

use banking::{bank, decoded, opened_banks, select_bank, select_loose, select_sound};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_extraction::Codec;
use p4kx_gui::{
    BankSound, Language, Model, PreviewStatus, PreviewTab, SoundbankPreview, SoundbankStatus,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use sounding::{FakeOutput, TestResult, opened, player, select, wait_until};

fn press(harness: &mut Harness<'_, Model>, label: &str) {
    harness.get_by_label(label).click();
    for _ in 0..3 {
        harness.step();
    }
}

type Listed = (u32, bool, Option<Codec>, Option<Duration>);

fn sounds(harness: &Harness<'_, Model>) -> Result<Vec<Listed>, String> {
    match bank(harness.state()).map(SoundbankPreview::status) {
        Some(SoundbankStatus::Opened(sounds)) => Ok(sounds
            .iter()
            .map(|sound| {
                let info = sound.info();
                (
                    sound.id(),
                    sound.is_prefetch(),
                    info.map(|info| info.codec),
                    info.and_then(|info| info.duration),
                )
            })
            .collect()),
        other => Err(format!("not opened: {other:?}")),
    }
}

#[test]
fn selecting_a_bnk_lists_its_embedded_wems_with_id_codec_and_duration() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;

    select_bank(&mut harness, "arms.bnk")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Audio,
        "Audio tab"
    );
    let listed = sounds(&harness)?;
    let ids_and_codecs: Vec<_> = listed
        .iter()
        .map(|&(id, prefetch, codec, _)| (id, prefetch, codec))
        .collect();
    assert_eq!(
        ids_and_codecs,
        [
            (101, false, Some(Codec::Pcm)),
            (102, false, Some(Codec::ImaAdpcm)),
            (103, false, Some(Codec::Aac)),
            (104, true, None),
            (105, false, None),
        ],
        "ascending IDs with their codecs"
    );
    assert_eq!(
        listed.first().and_then(|listed| listed.3),
        Some(Duration::from_secs(1)),
        "duration of the PCM tone"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Row, "WEM 101, PCM, 0:01.000")
            .is_some(),
        "row of the first WEM"
    );
    Ok(())
}

#[test]
fn clicking_a_row_selects_the_wem_and_plays_with_the_loose_controls() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &output)?;
    select_bank(&mut harness, "arms.bnk")?;

    harness.get_by_label_contains("WEM 101,").click();
    decoded(&mut harness, 101)?;

    assert_eq!(
        bank(harness.state()).and_then(SoundbankPreview::selected),
        Some(101)
    );
    assert!(
        harness
            .query_by_label("Codec PCM Channels 2 Sample rate 48,000 Hz Duration 0:01.000")
            .is_some(),
        "facts"
    );
    assert!(harness.query_by_label("0:00 / 0:01").is_some(), "time");
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Position")
            .is_some(),
        "position bar"
    );
    press(&mut harness, "Play");
    assert_eq!(output.started(), 1, "plays");
    assert_eq!(
        player(harness.state()).map(|player| player.pcm().frames()),
        Some(48_000),
        "decoded from the bank"
    );
    assert!(
        output
            .pull(256)
            .is_some_and(|samples| samples.iter().any(|&sample| sample != 0.0)),
        "not silent"
    );
    Ok(())
}

#[test]
fn selecting_another_wem_replaces_the_sound() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &output)?;
    select_bank(&mut harness, "arms.bnk")?;
    select_sound(&mut harness, 101)?;
    press(&mut harness, "Play");

    select_sound(&mut harness, 102)?;

    assert_eq!(
        bank(harness.state()).and_then(SoundbankPreview::selected),
        Some(102)
    );
    assert!(
        harness
            .query_by_label_contains("Codec IMA ADPCM Channels 1")
            .is_some(),
        "facts of the second"
    );
    assert_eq!(
        player(harness.state()).map(|player| player.pcm().frames()),
        Some(640),
        "the second sound is loaded"
    );
    press(&mut harness, "Play");
    assert_eq!(output.started(), 2, "each sound plays on its own stream");
    Ok(())
}

#[test]
fn a_codec_without_a_decoder_is_named_for_the_embedded_wem() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "arms.bnk")?;

    select_sound(&mut harness, 103)?;

    assert!(
        harness
            .query_by_label(
                "The preview cannot play AAC sounds (WEM 103 of Data\\Sounds\\arms.bnk)."
            )
            .is_some(),
        "message"
    );
    assert!(harness.query_by_label("Play").is_none(), "no controls");
    Ok(())
}

#[test]
fn a_prefetched_wem_is_explained_instead_of_played() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "arms.bnk")?;

    select_sound(&mut harness, 104)?;

    assert!(
        harness
            .query_by_label_contains("WEM 104 is only prefetched in this soundbank")
            .is_some(),
        "note"
    );
    assert!(
        bank(harness.state())
            .and_then(SoundbankPreview::audio)
            .is_none(),
        "nothing to decode"
    );
    assert!(harness.query_by_label("Play").is_none(), "no controls");
    Ok(())
}

#[test]
fn plug_in_media_is_listed_and_explained_instead_of_played() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "arms.bnk")?;

    select_sound(&mut harness, 105)?;

    assert!(
        bank(harness.state())
            .and_then(SoundbankPreview::selected_sound)
            .is_some_and(BankSound::is_plugin),
        "recognised as plug-in media"
    );
    assert!(
        harness
            .query_by_label_contains("WEM 105 is plug-in media")
            .is_some(),
        "note"
    );
    assert!(harness.query_by_label("Play").is_none(), "no controls");
    Ok(())
}
#[test]
fn a_corrupt_bank_shows_a_message_and_the_window_keeps_working() -> TestResult {
    let output = FakeOutput::default();
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &output)?;

    select_bank(&mut harness, "broken.bnk")?;

    assert!(
        matches!(
            bank(harness.state()).map(SoundbankPreview::status),
            Some(SoundbankStatus::Failed(Some(_)))
        ),
        "failed"
    );
    assert!(
        harness
            .query_by_label_contains("Cannot read the soundbank Data\\Sounds\\broken.bnk: ")
            .is_some(),
        "message"
    );
    select_loose(&mut harness, "tone.wem")?;
    press(&mut harness, "Play");
    assert_eq!(output.started(), 1, "a loose WEM still plays");
    Ok(())
}

#[test]
fn a_bank_too_large_to_read_says_so() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;

    select_bank(&mut harness, "huge.bnk")?;

    assert!(
        matches!(
            bank(harness.state()).map(SoundbankPreview::status),
            Some(SoundbankStatus::TooLarge)
        ),
        "too large"
    );
    assert!(
        harness
            .query_by_label_contains("Data\\Sounds\\huge.bnk is 256.0 MiB. Entries larger than")
            .is_some(),
        "message"
    );
    Ok(())
}

#[test]
fn a_bank_of_a_changed_archive_is_not_read() -> TestResult {
    let (folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "arms.bnk")?;

    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\patch.txt", "patch"))
        .write_to(folder.path().join("Data.p4k"))?;
    let browser = harness.state_mut().browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "big.bnk")
        .ok_or("no bank")?;
    browser.select_entry(id, || {});
    wait_until(&mut harness, |model| {
        model.browser().is_some_and(|browser| {
            browser
                .preview()
                .is_some_and(|preview| matches!(preview.status(), PreviewStatus::ArchiveChanged))
        })
    })?;

    assert!(bank(harness.state()).is_none(), "no soundbank preview");
    Ok(())
}
#[test]
fn a_bank_without_wems_says_so() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;

    select_bank(&mut harness, "empty.bnk")?;

    assert!(
        harness
            .query_by_label("Data\\Sounds\\empty.bnk embeds no WEMs.")
            .is_some(),
        "message"
    );
    Ok(())
}

#[test]
fn selecting_a_loose_wem_after_a_bank_leaves_the_soundbank() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "arms.bnk")?;
    select_sound(&mut harness, 101)?;

    select_loose(&mut harness, "tone.wem")?;

    assert!(bank(harness.state()).is_none(), "no soundbank preview");
    assert!(harness.query_by_label("Play").is_some(), "loose controls");
    Ok(())
}
#[test]
fn a_loose_wem_has_no_soundbank_list() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, &FakeOutput::default())?;

    select(&mut harness, "tone.wem")?;

    assert!(bank(harness.state()).is_none(), "no soundbank preview");
    assert!(
        harness.query_by_label("Embedded WEMs").is_none(),
        "no WEM list"
    );
    assert!(harness.query_by_label("Play").is_some(), "loose controls");
    Ok(())
}
#[test]
fn a_long_list_scrolls_with_the_keyboard_and_keeps_the_selected_row_in_view() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::BritishEnglish, &FakeOutput::default())?;
    select_bank(&mut harness, "big.bnk")?;
    assert!(
        harness.query_by_label_contains("WEM 1,").is_some(),
        "first row"
    );
    assert!(
        harness.query_by_label_contains("WEM 60,").is_none(),
        "the last row is not laid out"
    );

    harness.get_by_label("Embedded WEMs").focus();
    harness.step();
    harness.key_press(egui::Key::End);
    for _ in 0..3 {
        harness.step();
    }

    assert!(
        harness.query_by_label_contains("WEM 60,").is_some(),
        "End scrolls to the last row"
    );
    select_sound(&mut harness, 1)?;
    for _ in 0..3 {
        harness.step();
    }
    assert!(
        harness.query_by_label_contains("WEM 1,").is_some(),
        "the selected row is scrolled into view"
    );
    Ok(())
}
#[test]
fn the_german_list_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened_banks(Language::German, &FakeOutput::default())?;

    select_bank(&mut harness, "arms.bnk")?;

    assert!(
        harness
            .query_by_role_and_label(Role::Row, "WEM 101, PCM, 0:01.000")
            .is_some(),
        "row"
    );
    select_sound(&mut harness, 101)?;
    assert!(harness.query_by_label("Abspielen").is_some(), "play");
    select_sound(&mut harness, 104)?;
    assert!(
        harness
            .query_by_label_contains("WEM 104 ist in dieser Soundbank nur vorgeladen")
            .is_some(),
        "prefetch note"
    );
    Ok(())
}
