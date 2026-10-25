// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The kind of a Channel and its test-build notice.

use std::path::Path;

use p4kx_channels::ChannelKind;

#[test]
fn the_folder_name_tells_the_kind_ignoring_case() {
    let cases = [
        ("LIVE", ChannelKind::Live),
        ("PTU", ChannelKind::Ptu),
        ("ptu", ChannelKind::Ptu),
        ("EPTU", ChannelKind::Eptu),
        ("HOTFIX", ChannelKind::Hotfix),
        ("Tech-Preview", ChannelKind::TechPreview),
        ("StarCitizen", ChannelKind::Unknown),
        ("", ChannelKind::Unknown),
    ];
    for (name, kind) in cases {
        assert_eq!(ChannelKind::from_name(name), kind, "{name:?}");
    }
}

#[test]
fn an_archive_is_told_by_the_folder_that_holds_it() {
    let base = std::env::temp_dir();
    assert_eq!(
        ChannelKind::of_archive(&base.join("PTU").join("Data.p4k")),
        ChannelKind::Ptu
    );
    assert_eq!(
        ChannelKind::of_archive(&base.join("Elsewhere").join("Data.p4k")),
        ChannelKind::Unknown
    );
    assert_eq!(ChannelKind::of_archive(Path::new("")), ChannelKind::Unknown);
}

#[test]
fn only_test_channels_carry_a_notice() {
    for kind in [
        ChannelKind::Ptu,
        ChannelKind::Eptu,
        ChannelKind::Hotfix,
        ChannelKind::TechPreview,
    ] {
        assert!(kind.is_test_build());
        let notice = kind.notice().unwrap_or_default();
        assert!(notice.contains(kind.name().unwrap_or("?")), "{notice}");
        assert!(notice.contains("confidential"), "{notice}");
        assert!(
            notice.contains("Channel, which carries Star Citizen test builds"),
            "{notice}"
        );
    }
    for kind in [ChannelKind::Live, ChannelKind::Unknown] {
        assert!(!kind.is_test_build());
        assert_eq!(kind.notice(), None);
    }
}
