//! Corrupt movies never panic: every byte of a movie with every kind of definition and a first
//! frame to render is flipped, and the movie is cut at every length.

use std::collections::HashMap;

use p4kx_gfx::{Background, CanvasSize, Movie, RasterOptions, RgbaImage};
use p4kx_testkit::{
    TestFill, TestGlyph, TestMovie, TestPlace, TestShape, TestShapeRecord, TestSignature,
    exports_body, external_image_body, font3_body, gray_jpeg, jpeg_body, lossless_body,
    morph_shape_body, place_object2_body, progressive_gray_jpeg, sprite_body, sub_image_body,
};

fn rich_movie(signature: TestSignature) -> Vec<u8> {
    let line = TestShapeRecord::Line(40, -30);
    let moved = TestShapeRecord::Move {
        to: (5, 5),
        fill0: 1,
        fill1: 0,
        line: 1,
    };
    let shape = TestShape::new(1, 4, [0, 100, 0, 100])
        .fill(TestFill::Linear([1, 2, 3, 4], [5, 6, 7, 8]))
        .fill(TestFill::ScaledBitmap {
            id: 3,
            scale: 20,
            at: (1, 1),
            repeating: true,
            smoothed: true,
        })
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
    let movie = TestMovie::new(signature)
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
        .tag(21, &jpeg_body(10, &progressive_gray_jpeg(16, 8, 40), None))
        .tag(1009, &external_image_body(5, 2, 14, (8, 8), ("a", "a.dds")))
        .tag(1008, &sub_image_body(6, 3, [0, 0, 1, 1]))
        .tag(75, &font3_body(7, "F", &[glyph.clone(), glyph]))
        .tag(
            39,
            &sprite_body(8, 1, &[(26, place_object2_body(1, 1)), (1, Vec::new())]),
        )
        .tag(56, &exports_body(&[(1, "one")]));
    first_frame(movie).tag(1, &[]).bytes()
}

fn first_frame(movie: TestMovie) -> TestMovie {
    movie
        .tag(
            70,
            &TestPlace::new(1, 8)
                .color([0.5, 1.0, 1.0, 0.5], [3, 0, 0, 0])
                .blur()
                .body3(),
        )
        .tag(26, &TestPlace::new(2, 1).clip(4).at(3, 3).body2())
        .tag(26, &TestPlace::new(3, 2).ratio(30_000).body2())
        .tag(
            26,
            &TestPlace::new(4, 9)
                .matrix([2.0, 0.5, 0.25, 0.0], (7, 7))
                .body2(),
        )
}

const SIGNATURES: [TestSignature; 4] = [
    TestSignature::Gfx,
    TestSignature::Cws,
    TestSignature::Zws,
    TestSignature::Cfx,
];

fn exercise(bytes: Vec<u8>) {
    let Ok(movie) = Movie::parse(bytes) else {
        return;
    };
    let assets = movie.assets();
    for bitmap in &assets.bitmaps {
        let _ignored = assets.bitmap_size(bitmap);
        if let Ok(image) = assets.decode_bitmap(bitmap) {
            let _ignored = image
                .crop((0, 0), (1, 1))
                .map(|image| image.write_png(Vec::new()));
        }
    }
    for shape in &assets.shapes {
        let _ignored = shape.write_json(Vec::new());
    }
    let scene = movie.first_frame(&assets);
    let bitmaps: HashMap<u16, RgbaImage> = scene
        .bitmap_ids()
        .into_iter()
        .filter_map(|id| {
            let bitmap = assets.bitmaps.iter().find(|bitmap| bitmap.id == id)?;
            Some((id, assets.decode_bitmap(bitmap).ok()?))
        })
        .collect();
    let options = RasterOptions {
        size: CanvasSize::Fit {
            width: 16,
            height: 12,
        },
        background: Background::Transparent,
    };
    let _ignored = scene.rasterize(&bitmaps, &options);
}

#[test]
fn every_flipped_byte_is_read_without_panicking() {
    for signature in SIGNATURES {
        let movie = rich_movie(signature);
        for index in 0..movie.len() {
            let mut bytes = movie.clone();
            bytes[index] ^= 0xFF;
            exercise(bytes);
        }
    }
}

#[test]
fn every_truncation_is_read_without_panicking() {
    let movie = rich_movie(TestSignature::Gfx);
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
    let movie = Movie::parse(rich_movie(TestSignature::Zws)).expect("valid");
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    assert_eq!(assets.bitmaps.len(), 5, "bitmaps");
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
