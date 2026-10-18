//! Bitmaps: lossless and JPEG-family images decode to RGBA, GFx external images and sub-images
//! are read as references.

use p4kx_gfx::{Assets, BitmapSource, EmbeddedFormat, GfxError, ImageFileFormat, Movie};
use p4kx_testkit::{
    TestMovie, TestSignature, color_mapped_body, external_image_body, gray_jpeg, jpeg_body,
    lossless_body, sub_image_body, zlib,
};

const PIXELS: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 128],
    [255, 255, 255, 0],
];

fn movie(tags: &[(u16, Vec<u8>)]) -> Movie {
    let movie = tags
        .iter()
        .fold(TestMovie::new(TestSignature::Gfx), |movie, (code, body)| {
            movie.tag(*code, body)
        });
    Movie::parse(movie.bytes()).unwrap_or_else(|error| unreachable!("invalid movie: {error}"))
}

fn decoded(assets: &Assets<'_>, index: usize) -> Result<Vec<u8>, GfxError> {
    let bitmap = assets.bitmaps.get(index).ok_or(GfxError::NotEmbedded)?;
    assets
        .decode_bitmap(bitmap)
        .map(|image| image.pixels().to_vec())
}

#[test]
fn lossless_bitmaps_decode_to_straight_alpha() {
    let movie = movie(&[
        (20, lossless_body(1, 2, 2, &PIXELS, false)),
        (36, lossless_body(2, 2, 2, &PIXELS, true)),
    ]);
    let assets = movie.assets();
    let opaque: Vec<u8> = PIXELS
        .iter()
        .flat_map(|&[red, green, blue, _]| [red, green, blue, 255])
        .collect();
    assert_eq!(decoded(&assets, 0).expect("v1"), opaque, "v1 is opaque");
    let straight = decoded(&assets, 1).expect("v2");
    assert_eq!(straight[..8], opaque[..8], "opaque pixels unchanged");
    assert_eq!(straight[8..12], [0, 0, 255, 128], "unmultiplied");
    assert_eq!(straight[12..], [0, 0, 0, 0], "transparent");
    let image = assets.decode_bitmap(&assets.bitmaps[1]).expect("v2");
    assert_eq!((image.width(), image.height()), (2, 2), "size");
}

#[test]
fn a_color_mapped_bitmap_looks_up_its_palette_and_skips_row_padding() {
    let body = color_mapped_body(
        4,
        3,
        &[[10, 20, 30], [40, 50, 60]],
        &[&[0, 1, 1], &[1, 0, 9]],
    );
    let movie = movie(&[(20, body)]);
    let assets = movie.assets();
    let pixels = decoded(&assets, 0).expect("decodes");
    assert_eq!(pixels.len(), 3 * 2 * 4, "3 x 2");
    assert_eq!(pixels[..4], [10, 20, 30, 255], "first entry");
    assert_eq!(
        pixels[12..16],
        [40, 50, 60, 255],
        "second row starts after padding"
    );
    assert_eq!(
        pixels[20..],
        [0, 0, 0, 0],
        "an index beyond the palette is transparent"
    );
}

#[test]
fn a_jpeg_decodes_and_takes_its_alpha_channel_unmultiplied() {
    let jpeg = gray_jpeg(16, 8, 60);
    let alpha: Vec<u8> = (128..=255).collect();
    let movie = movie(&[
        (21, jpeg_body(1, &jpeg, None)),
        (35, jpeg_body(2, &jpeg, Some(&alpha))),
        (35, jpeg_body(3, &jpeg, Some(&[0; 128]))),
    ]);
    let assets = movie.assets();
    let BitmapSource::Jpeg(source) = &assets.bitmaps[0].source else {
        panic!("a JPEG");
    };
    assert_eq!(source.format(), EmbeddedFormat::Jpeg, "format");
    let plain = decoded(&assets, 0).expect("JPEG2");
    assert!(
        plain.chunks(4).all(|pixel| pixel == [60, 60, 60, 255]),
        "grey"
    );
    let with_alpha = decoded(&assets, 1).expect("JPEG3");
    assert_eq!(with_alpha[..4], [119, 119, 119, 128], "unmultiplied by 128");
    assert_eq!(with_alpha[508..], [60, 60, 60, 255], "opaque");
    let transparent = decoded(&assets, 2).expect("JPEG3");
    assert!(
        transparent.iter().all(|&byte| byte == 0),
        "clamped to the alpha"
    );
}
#[test]
fn a_define_bits_jpeg_is_joined_with_the_jpeg_tables() {
    let jpeg = gray_jpeg(8, 8, 60);
    let split = jpeg
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("SOF");
    let mut tables = jpeg[..split].to_vec();
    tables.extend_from_slice(&[0xFF, 0xD9]);
    let mut image = vec![0xFF, 0xD9, 0xFF, 0xD8, 0xFF, 0xD8];
    image.extend_from_slice(&jpeg[split..]);
    let mut body = 3_u16.to_le_bytes().to_vec();
    body.extend_from_slice(&image);
    let movie = movie(&[(8, tables), (6, body)]);
    let assets = movie.assets();
    assert!(assets.jpeg_tables.is_some(), "tables kept");
    let pixels = decoded(&assets, 0).expect("joined JPEG");
    assert!(
        pixels.chunks(4).all(|pixel| pixel == [60, 60, 60, 255]),
        "grey"
    );
}

#[test]
fn an_embedded_png_decodes_and_a_gif_is_refused() {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, 2, 1);
    encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().expect("header");
    writer
        .write_image_data(&[1, 2, 3, 4, 5, 6])
        .expect("pixels");
    writer.finish().expect("finish");
    let movie = movie(&[
        (21, jpeg_body(1, &png, None)),
        (21, jpeg_body(2, b"GIF89a....", None)),
    ]);
    let assets = movie.assets();
    assert_eq!(
        decoded(&assets, 0).expect("PNG"),
        [1, 2, 3, 255, 4, 5, 6, 255],
        "RGB"
    );
    assert!(matches!(decoded(&assets, 1), Err(GfxError::Gif)), "GIF");
}

#[test]
fn external_images_and_sub_images_are_references() {
    let movie = movie(&[
        (
            1009,
            external_image_body(
                5,
                2,
                14,
                (34, 34),
                ("Textures\\Button", "Textures\\Button.dds"),
            ),
        ),
        (
            1001,
            external_image_body(6, 1, 19, (64, 32), ("bg", "../Textures/bg.tif")),
        ),
        (1008, sub_image_body(7, 5, [1, 2, 11, 12])),
    ]);
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    let BitmapSource::External(dds) = &assets.bitmaps[0].source else {
        panic!("external");
    };
    assert_eq!(assets.bitmaps[0].id, 5, "id after the ID type");
    assert_eq!(dds.format, ImageFileFormat::DDS, "DDS-backed");
    assert_eq!(dds.format.name(), Some("dds"), "named");
    assert_eq!((dds.target_width, dds.target_height), (34, 34), "size");
    assert_eq!(dds.export_name, "Textures\\Button", "export name");
    assert_eq!(dds.file_name, "Textures\\Button.dds", "file");
    let BitmapSource::External(tif) = &assets.bitmaps[1].source else {
        panic!("external");
    };
    assert_eq!(tif.format.name(), Some("tiff"), "TIFF");
    assert_eq!(tif.file_name, "../Textures/bg.tif", "file of version 1");
    let BitmapSource::SubImage(sub) = &assets.bitmaps[2].source else {
        panic!("sub-image");
    };
    assert_eq!((sub.image_id, sub.x2, sub.y2), (5, 11, 12), "rectangle");
    assert!(
        matches!(decoded(&assets, 0), Err(GfxError::NotEmbedded)),
        "not embedded"
    );
}

#[test]
fn a_bitmap_above_the_pixel_limit_is_refused_before_inflating() {
    let mut body = lossless_body(1, 2, 2, &PIXELS, false);
    body[3..7].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    let movie = movie(&[(20, body)]);
    let assets = movie.assets();
    assert!(matches!(
        decoded(&assets, 0),
        Err(GfxError::BitmapTooLarge {
            width: 65535,
            height: 65535,
            ..
        })
    ));
}

#[test]
fn corrupt_or_short_pixel_data_is_an_error() {
    let mut body = lossless_body(1, 2, 2, &PIXELS, false);
    body.truncate(7);
    body.extend_from_slice(&zlib(&[0; 4]));
    let mut unknown = lossless_body(2, 2, 2, &PIXELS, false);
    unknown[2] = 9;
    let movie = movie(&[
        (20, body),
        (20, unknown),
        (21, jpeg_body(3, b"\xFF\xD8junk", None)),
    ]);
    let assets = movie.assets();
    assert!(
        matches!(decoded(&assets, 0), Err(GfxError::BitmapData { .. })),
        "short"
    );
    assert!(
        matches!(
            decoded(&assets, 1),
            Err(GfxError::LosslessFormat { format: 9 })
        ),
        "format"
    );
    assert!(
        matches!(decoded(&assets, 2), Err(GfxError::Jpeg { .. })),
        "JPEG"
    );
}

#[test]
fn a_decoded_bitmap_writes_as_png() {
    let movie = movie(&[(36, lossless_body(1, 2, 2, &PIXELS, true))]);
    let assets = movie.assets();
    let image = assets.decode_bitmap(&assets.bitmaps[0]).expect("decodes");
    let mut file = Vec::new();
    image.write_png(&mut file).expect("writes");
    let decoder = png::Decoder::new(std::io::Cursor::new(file));
    let mut reader = decoder.read_info().expect("PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("size")];
    reader.next_frame(&mut pixels).expect("frame");
    assert_eq!(pixels, image.pixels(), "round trip");
}
