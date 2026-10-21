//! The limits of text, exercised with small values through the private composer.

use p4kx_testkit::{
    TestEditText, TestGlyph, TestMovie, TestPlace, TestShapeRecord, TestSignature, font3_body,
};

use crate::movie::Movie;
use crate::scene::Scene;
use crate::scene_compose::{Composer, SceneLimits};
use crate::scene_report::SkipReason;
use crate::text_fonts::TextOptions;

fn glyph() -> TestGlyph {
    TestGlyph {
        code: u16::from(b'A'),
        advance: 12_000,
        records: vec![
            TestShapeRecord::Move {
                to: (0, 0),
                fill0: 0,
                fill1: 1,
                line: 0,
            },
            TestShapeRecord::Line(10_000, 0),
            TestShapeRecord::Line(0, -14_000),
            TestShapeRecord::Line(-10_000, 0),
            TestShapeRecord::Line(0, 14_000),
        ],
    }
}

fn texts(text: &str, placements: u16) -> Vec<u8> {
    let field = TestEditText::new(7, [0, 2000, 0, 600])
        .text(text)
        .font(3, 200);
    let mut movie = TestMovie::new(TestSignature::Gfx)
        .stage(100, 100)
        .tag(75, &font3_body(3, "Font", &[glyph()]))
        .tag(37, &field.body());
    for depth in 1..=placements {
        movie = movie.tag(26, &TestPlace::new(depth, 7).body2());
    }
    movie.tag(1, &[]).bytes()
}

fn compose(bytes: Vec<u8>, limits: SceneLimits) -> Scene {
    let movie = Movie::parse(bytes).unwrap_or_else(|error| unreachable!("invalid movie: {error}"));
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    Composer::compose_with(&assets, movie.header().frame_size, limits)
}

fn reasons(scene: &Scene) -> Vec<SkipReason> {
    scene.skipped.iter().map(|skipped| skipped.reason).collect()
}

#[test]
fn characters_beyond_the_glyph_limit_stop_the_composition() {
    let limits = SceneLimits {
        glyphs: 10,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(texts("AAAA", 4), limits);
    assert_eq!(
        reasons(&scene),
        [SkipReason::TooManyGlyphs],
        "4 characters when prepared, 4 when placed, then the limit"
    );
    assert_eq!(scene.items.len(), 4, "the first placement is drawn");
    let roomy = compose(texts("AAAA", 4), SceneLimits::DEFAULT);
    assert_eq!(roomy.items.len(), 16);
    assert!(roomy.skipped.is_empty(), "{:?}", roomy.skipped);
}

#[test]
fn a_glyph_is_converted_once_for_all_its_placements() {
    let limits = SceneLimits {
        records: 8,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(texts("AAAA", 3), limits);
    assert_eq!(scene.shapes.len(), 1, "one glyph shape for twelve glyphs");
    assert_eq!(scene.items.len(), 12);
    assert!(scene.skipped.is_empty(), "{:?}", scene.skipped);
}

#[test]
fn glyph_conversions_are_charged_to_the_record_budget() {
    let limits = SceneLimits {
        records: 3,
        ..SceneLimits::DEFAULT
    };
    let scene = compose(texts("A", 1), limits);
    assert_eq!(reasons(&scene), [SkipReason::TooManyRecords]);
    assert_eq!(scene.items.len(), 0);
}

#[test]
fn a_text_longer_than_the_character_limit_is_cut_and_listed() {
    let long = "A".repeat(crate::text_html::MAX_TEXT_CHARS + 50);
    let scene = compose(texts(&long, 1), SceneLimits::DEFAULT);
    assert_eq!(reasons(&scene), [SkipReason::TextTruncated]);
    assert_eq!(scene.items.len(), crate::text_html::MAX_TEXT_CHARS);
    let unused = TextOptions::default();
    assert!(unused.fonts.is_empty() && unused.localise.is_none());
}
