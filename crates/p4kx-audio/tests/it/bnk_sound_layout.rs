// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layout of a Sound object by bank version: no `cacheID` in version 150, one from version 151.

use p4kx_audio::{HircValue, MediaStorage, SoundSource, Soundbank};
use p4kx_testkit::{TestBank, TestHircObject};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SOURCE: u32 = 617_647_119;
const SIZE: u32 = 252_487;

const PARENT: u32 = 0x2C21_1B2E;

const fn source(parent_id: Option<u32>) -> SoundSource {
    SoundSource {
        plugin_id: 0x0004_0001,
        storage: MediaStorage::Embedded,
        source_id: SOURCE,
        in_memory_size: SIZE,
        source_bits: 0,
        parent_id,
    }
}

fn value_of(bytes: &[u8]) -> Result<HircValue, Box<dyn std::error::Error>> {
    let bank = Soundbank::parse(bytes)?;
    Ok(bank.object(7).ok_or("sound 7")?.value.clone())
}

#[test]
fn version_150_has_no_cache_id() -> TestResult {
    let bytes = TestBank::new(1)
        .version(150)
        .object(TestHircObject::sound_without_cache_id(7, SOURCE, 0, SIZE))
        .bytes();

    assert_eq!(value_of(&bytes)?, HircValue::Sound(source(None)));
    Ok(())
}

#[test]
fn versions_from_151_on_have_a_cache_id() -> TestResult {
    for version in [151, 152, 154] {
        let bytes = TestBank::new(1)
            .version(version)
            .object(TestHircObject::sound(7, SOURCE, 0, SIZE))
            .bytes();

        let parent = (version == 154).then_some(PARENT);
        assert_eq!(
            value_of(&bytes)?,
            HircValue::Sound(source(parent)),
            "version {version}: the parent is read from version 154 on"
        );
    }
    Ok(())
}

#[test]
fn a_sound_of_an_older_version_stays_raw() -> TestResult {
    let bytes = TestBank::new(1)
        .version(145)
        .object(TestHircObject::sound_without_cache_id(7, SOURCE, 0, SIZE))
        .bytes();

    assert_eq!(value_of(&bytes)?, HircValue::Raw);
    Ok(())
}
