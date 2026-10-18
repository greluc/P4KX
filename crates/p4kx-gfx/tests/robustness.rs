//! Corrupt movies never panic: every byte of a movie with every kind of definition is flipped,
//! and the movie is cut at every length.

use p4kx_gfx::{Movie, RgbaImage};
use p4kx_testkit::{
    TestFill, TestGlyph, TestMovie, TestShape, TestShapeRecord, TestSignature, exports_body,
    external_image_body, font3_body, gray_jpeg, jpeg_body, lossless_body, morph_shape_body,
    place_object2_body, sprite_body, sub_image_body,
};

fn rich_movie() -> Vec<u8> {
    let line = TestShapeRecord::Line(40, -30);
    let moved = TestShapeRecord::Move {
        to: (5, 5),
        fill0: 1,
        fill1: 0,
        line: 1,
    };
    let shape = TestShape::new(1, 4, [0, 100, 0, 100])
        .fill(TestFill::Linear([1, 2, 3, 4], [5, 6, 7, 8]))
        .line(20, [9, 9, 9, 9])
        .record(moved)
        .record(line)
        .record(TestShapeRecord::Curve(3, 4, 5, 6));
    let glyph = TestGlyph {
        code: 65,
        advance: 10,
        records: vec![moved, line],
    };
    let jpeg = gray_jpeg(8, 8, 90);
    TestMovie::new(TestSignature::Gfx)
        .tag(shape.code(), &shape.body())
        .tag(
            46,
            &morph_shape_body(2, ([1; 4], [2; 4]), &[moved, line], &[line]),
        )
        .tag(
            36,
            &lossless_body(3, 2, 1, &[[1, 2, 3, 4], [5, 6, 7, 8]], true),
        )
        .tag(35, &jpeg_body(4, &jpeg, Some(&[7; 64])))
        .tag(1009, &external_image_body(5, 2, 14, (8, 8), ("a", "a.dds")))
        .tag(1008, &sub_image_body(6, 3, [0, 0, 1, 1]))
        .tag(75, &font3_body(7, "F", &[glyph.clone(), glyph]))
        .tag(
            39,
            &sprite_body(8, 1, &[(26, place_object2_body(1, 1)), (1, Vec::new())]),
        )
        .tag(56, &exports_body(&[(1, "one")]))
        .tag(1, &[])
        .bytes()
}

fn exercise(bytes: Vec<u8>) {
    let Ok(movie) = Movie::parse(bytes) else {
        return;
    };
    let assets = movie.assets();
    for bitmap in &assets.bitmaps {
        if let Ok(image) = assets.decode_bitmap(bitmap) {
            let _ignored = image
                .crop((0, 0), (1, 1))
                .map(|image| image.write_png(Vec::new()));
        }
    }
    for shape in &assets.shapes {
        let _ignored = shape.write_json(Vec::new());
    }
}

#[test]
fn every_flipped_byte_is_read_without_panicking() {
    let movie = rich_movie();
    for index in 0..movie.len() {
        let mut bytes = movie.clone();
        bytes[index] ^= 0xFF;
        exercise(bytes);
    }
}

#[test]
fn every_truncation_is_read_without_panicking() {
    let movie = rich_movie();
    for len in 0..movie.len() {
        let mut bytes = movie[..len].to_vec();
        if let Some(declared) = bytes.get_mut(4..8) {
            declared.copy_from_slice(&u32::try_from(len).expect("small").to_le_bytes());
        }
        exercise(bytes);
    }
}

#[test]
fn the_rich_movie_reads_without_failures() {
    let movie = Movie::parse(rich_movie()).expect("valid");
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    assert_eq!(assets.bitmaps.len(), 4, "bitmaps");
    assert_eq!(assets.fonts.len(), 1, "fonts");
    assert_eq!(assets.sprites.len(), 1, "sprites");
    assert_eq!(assets.morph_shapes.len(), 1, "morph shapes");
    let decoded = assets.decode_bitmap(&assets.bitmaps[0]).expect("lossless");
    let cropped = decoded.crop((1, 0), (5, 5)).expect("inside");
    assert_eq!(
        (cropped.width(), cropped.height()),
        (1, 1),
        "cut to the image"
    );
    assert_eq!(cropped.pixels(), &decoded.pixels()[4..], "second pixel");
    assert!(decoded.crop((2, 0), (3, 1)).is_none(), "outside");
    assert!(
        RgbaImage::from_pixels(2, 2, vec![0; 15]).is_err(),
        "wrong length"
    );
}
