// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Versions outside 6 and 8 are a typed error before any other field is read (ADR-0136).

use p4kx_dataforge::{DataForge, DataForgeError, SUPPORTED_VERSIONS};
use p4kx_testkit::DataForgeBuilder;

fn header_of(version: u32, tail: usize) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.resize(8 + tail, 0xFF);
    bytes
}

#[test]
fn version_5_is_unsupported() {
    let bytes = DataForgeBuilder::new().version(5).build();

    let error = DataForge::open(&bytes).err();

    assert_eq!(
        error,
        Some(DataForgeError::UnsupportedVersion { version: 5 })
    );
}

#[test]
fn version_4_is_unsupported() {
    let bytes = DataForgeBuilder::new().version(4).build();

    let error = DataForge::open(&bytes).err();

    assert_eq!(
        error,
        Some(DataForgeError::UnsupportedVersion { version: 4 })
    );
}

#[test]
fn the_oldest_supported_version_opens() {
    let oldest = SUPPORTED_VERSIONS.iter().copied().min();
    assert_eq!(oldest, Some(6), "oldest supported version");
    let bytes = DataForgeBuilder::new().version(6).build();

    let forge = DataForge::open(&bytes).map(|forge| forge.version()).ok();

    assert_eq!(forge, Some(6));
}

#[test]
fn an_old_header_cut_after_the_version_is_still_unsupported_not_truncated() {
    for version in [0, 1, 4, 5, 7, u32::MAX] {
        let error = DataForge::open(&header_of(version, 0)).err();

        assert_eq!(
            error,
            Some(DataForgeError::UnsupportedVersion { version }),
            "version {version}"
        );
    }
}

#[test]
fn an_old_header_with_garbage_behind_it_is_unsupported_not_corrupt() {
    let error = DataForge::open(&header_of(5, 4096)).err();

    assert_eq!(
        error,
        Some(DataForgeError::UnsupportedVersion { version: 5 })
    );
}

#[test]
fn the_message_names_the_version_and_the_supported_ones() {
    let message = DataForgeError::UnsupportedVersion { version: 5 }.to_string();

    assert_eq!(
        message,
        "DataForge version 5 is not supported; this version of P4KX reads versions 6 and 8"
    );
}
