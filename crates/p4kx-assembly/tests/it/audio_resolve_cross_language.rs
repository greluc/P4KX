// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Trigger resolution across soundbanks: the choice among soundbanks of several languages.

use std::error::Error;

use p4kx_assembly::audio::{LanguageChoice, SoundVariant};
use p4kx_testkit::{TestBank, TestHircObject, wwise_short_id};

use crate::audio_resolve_cross::{PLAY, TRIGGER, music, resolve, stingers};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn stingers_of_other_languages_do_not_break_the_one_of_the_chosen_language() -> TestResult {
    let banks = [
        ("Music", music(&[(TRIGGER, 0x77)])),
        ("English(US)\\Stingers", stingers(0x77)),
        ("German\\Stingers", stingers(0x77)),
    ];
    for preferred in ["German", "English(US)"] {
        resolve(
            &banks,
            &LanguageChoice::new(Some(preferred), Some("English(US)")),
            |sounds| {
                let [variant] = sounds.variants() else {
                    return Err(format!("one variant: {:?}", sounds.variants()).into());
                };
                assert_eq!(variant.soundbank().language(), Some(preferred));
                assert!(
                    sounds
                        .breaks()
                        .iter()
                        .all(|link| link.soundbank().and_then(|bank| bank.language())
                            == Some(preferred)),
                    "{:?}",
                    sounds.breaks()
                );
                Ok(())
            },
        )?;
    }
    resolve(
        &banks,
        &LanguageChoice::new(Some("French"), None),
        |sounds| {
            assert_eq!(sounds.variants().len(), 0, "no variants");
            let kinds: Vec<_> = sounds.breaks().iter().map(|b| b.reason().kind()).collect();
            assert_eq!(kinds, ["no_localisation"]);
            Ok(())
        },
    )
}

#[test]
fn a_named_localised_soundbank_is_chosen_by_the_language() -> TestResult {
    let voice = |sound: u32| {
        TestBank::new(wwise_short_id("Voice")).object(TestHircObject::sound(10, sound, 2, 0))
    };
    let music = TestBank::new(wwise_short_id("Music"))
        .object(TestHircObject::event(wwise_short_id("Play_Music"), &[1]))
        .object(TestHircObject::action_in_bank(
            1,
            PLAY,
            10,
            wwise_short_id("Voice"),
        ));
    let banks = [
        ("Music", music),
        ("English(US)\\Voice", voice(7000)),
        ("German\\Voice", voice(7001)),
    ];
    for (preferred, expected) in [("German", 7001), ("English(US)", 7000)] {
        resolve(
            &banks,
            &LanguageChoice::new(Some(preferred), Some("English(US)")),
            |sounds| {
                assert_eq!(
                    sounds
                        .variants()
                        .iter()
                        .map(SoundVariant::wem_id)
                        .collect::<Vec<_>>(),
                    [Some(expected)]
                );
                Ok(())
            },
        )?;
    }
    Ok(())
}
