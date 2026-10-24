// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tag stream: counts, unknown tags, long headers, broken tags, exports and exporter info.

use p4kx_gfx::{FrameLabel, GfxError, Movie, tag_name};
use p4kx_testkit::{
    TestMovie, TestSignature, exporter_info_body, exports_body, place_object2_body, sprite_body,
};

fn movie(movie: &TestMovie) -> Movie {
    Movie::parse(movie.bytes()).unwrap_or_else(|error| unreachable!("invalid movie: {error}"))
}

#[test]
fn every_tag_is_counted_by_code_sprite_tags_included() {
    let sprite = sprite_body(5, 2, &[(1, Vec::new()), (1, Vec::new())]);
    let movie = movie(
        &TestMovie::new(TestSignature::Gfx)
            .tag(39, &sprite)
            .tag(1, &[])
            .tag(1, &[]),
    );
    let counts: Vec<(u16, u32)> = movie
        .assets()
        .tag_counts
        .iter()
        .map(|count| (count.code, count.count))
        .collect();
    assert_eq!(
        counts,
        [(0, 2), (1, 4), (39, 1)],
        "End twice, four ShowFrames"
    );
    assert_eq!(tag_name(39), Some("DefineSprite"), "named");
}

#[test]
fn unknown_tags_are_kept_with_their_body_and_counted() {
    let movie = movie(
        &TestMovie::new(TestSignature::Fws)
            .tag(250, b"secret")
            .tag(1, &[])
            .tag(250, b"again")
            .tag(1003, b"gradient"),
    );
    let assets = movie.assets();
    let unknown: Vec<(u16, &[u8])> = assets
        .unknown_tags
        .iter()
        .map(|tag| (tag.code, tag.body))
        .collect();
    assert_eq!(
        unknown,
        [(250, b"secret".as_slice()), (250, b"again".as_slice())],
        "only codes without a name are unknown"
    );
    let count = assets.tag_counts.iter().find(|count| count.code == 250);
    assert_eq!(count.map(|count| count.count), Some(2), "counted");
    assert_eq!(count.and_then(p4kx_gfx::TagCount::name), None, "no name");
    assert_eq!(
        tag_name(1003),
        Some("DefineExternalGradient"),
        "GFx tag known"
    );
}

#[test]
fn long_tag_headers_are_read() {
    let movie = movie(&TestMovie::new(TestSignature::Cfx).long_tag(250, b"x"));
    let assets = movie.assets();
    assert_eq!(assets.unknown_tags.len(), 1, "one unknown tag");
    assert_eq!(assets.unknown_tags[0].body, b"x", "body");
}

#[test]
fn a_tag_running_past_the_movie_ends_the_stream_as_a_failure() {
    let mut bytes = TestMovie::new(TestSignature::Fws)
        .tag(1, &[])
        .raw(&[0x3F, 0x00, 0xFF, 0xFF, 0x00, 0x00])
        .bytes();
    let declared = u32::try_from(bytes.len()).expect("fits");
    bytes[4..8].copy_from_slice(&declared.to_le_bytes());
    let movie = movie_from(bytes);
    let assets = movie.assets();
    assert_eq!(assets.timeline.frames, 1, "the frame before is read");
    assert_eq!(assets.failures.len(), 1, "one failure");
    assert!(
        matches!(
            assets.failures[0].error,
            GfxError::TagLength {
                code: 0,
                len: 0xFFFF,
                ..
            }
        ),
        "{:?}",
        assets.failures[0].error
    );
}

fn movie_from(bytes: Vec<u8>) -> Movie {
    Movie::parse(bytes).unwrap_or_else(|error| unreachable!("invalid header: {error}"))
}

#[test]
fn a_broken_definition_is_a_failure_and_the_rest_is_read() {
    let movie = movie(
        &TestMovie::new(TestSignature::Gfx)
            .tag(2, &[1])
            .tag(56, &exports_body(&[(7, "icon_ship")])),
    );
    let assets = movie.assets();
    assert_eq!(assets.failures.len(), 1, "the shape fails");
    assert_eq!(assets.failures[0].code, 2, "DefineShape");
    assert!(
        matches!(assets.failures[0].error, GfxError::Parse(_)),
        "truncated"
    );
    assert_eq!(
        assets.export_name(7),
        Some("icon_ship"),
        "the exports are read"
    );
}

#[test]
fn exports_frame_labels_and_placements_are_read() {
    let movie = movie(
        &TestMovie::new(TestSignature::Gfx)
            .tag(56, &exports_body(&[(3, "Button"), (4, "Panel")]))
            .tag(43, b"intro\0")
            .tag(26, &place_object2_body(1, 4))
            .tag(26, &place_object2_body(2, 3))
            .tag(26, &place_object2_body(3, 4))
            .tag(26, &[0x01, 1, 0])
            .tag(1, &[]),
    );
    let assets = movie.assets();
    assert_eq!(assets.export_name(3), Some("Button"), "first");
    assert_eq!(assets.export_name(4), Some("Panel"), "second");
    assert_eq!(
        assets.timeline.labels,
        [FrameLabel {
            frame: 0,
            name: "intro".to_owned()
        }],
        "labels"
    );
    assert_eq!(assets.timeline.placed, [3, 4], "sorted, each once");
    assert_eq!(assets.timeline.frames, 1, "frames");
}

#[test]
fn the_gfx_exporter_info_is_read() {
    let movie = movie(
        &TestMovie::new(TestSignature::Gfx).tag(1000, &exporter_info_body("HUD_Font_LocFont")),
    );
    let info = movie.assets().exporter_info.expect("exporter info");
    assert_eq!(info.version, 0x036F, "version");
    assert_eq!(info.flags, 0x10, "flags from version 0x10A on");
    assert_eq!(info.bitmap_format, 19, "TIFF");
    assert_eq!(info.prefix, "", "no prefix");
    assert_eq!(info.swf_name, "HUD_Font_LocFont", "SWF name");
}
