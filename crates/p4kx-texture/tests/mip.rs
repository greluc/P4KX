//! Reading one mip of a split texture and writing a decoded mip as PNG.

use std::collections::BTreeMap;
use std::io;

use p4kx_testkit::{SplitDds, TestTextureFormat};
use p4kx_texture::{Mip, SiblingReader, TextureError, read_mip, write_png};

/// Parts held in memory, by part number.
struct Parts(BTreeMap<u32, Vec<u8>>);

impl Parts {
    fn of(dds: &SplitDds) -> Self {
        Self(
            (1..=dds.part_count())
                .map(|index| (index, dds.part(index)))
                .collect(),
        )
    }
}

impl SiblingReader for Parts {
    type Part<'a> = &'a [u8];

    fn part_len(&self, index: u32) -> io::Result<Option<u64>> {
        Ok(self.0.get(&index).map(|data| data.len() as u64))
    }

    fn open_part(&self, index: u32) -> io::Result<Self::Part<'_>> {
        self.0
            .get(&index)
            .map(Vec::as_slice)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
}

fn read(dds: &SplitDds, level: u32) -> Result<Mip, TextureError> {
    let header = dds.header_bytes();
    read_mip(
        header.as_slice(),
        header.len() as u64,
        &Parts::of(dds),
        level,
    )
}

#[test]
fn every_mip_is_read_from_its_part_or_from_the_header_tail() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 64, 32).split(4);

    for level in 0..7 {
        let mip = read(&dds, level)?;
        assert_eq!(mip.level(), level, "level {level}");
        assert_eq!(mip.data(), dds.mip(level).as_slice(), "data of mip {level}");
        assert_eq!(
            (mip.width(), mip.height()),
            ((64 >> level).max(1), (32 >> level).max(1)),
            "size of mip {level}"
        );
    }
    Ok(())
}

#[test]
fn an_unsplit_texture_reads_every_mip_from_the_header() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8);

    let mip = read(&dds, 1)?;

    assert_eq!(mip.data(), dds.mip(1).as_slice(), "mip 1 after mip 0");
    Ok(())
}

#[test]
fn a_mip_beyond_the_chain_is_named() {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).split(2);

    let result = read(&dds, 5);

    assert!(
        matches!(
            result,
            Err(TextureError::NoSuchMip {
                level: 5,
                mip_count: 5
            })
        ),
        "no mip 5: {result:?}"
    );
}

#[test]
fn cubemaps_and_volumes_are_not_decoded_yet() {
    let cube = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).cubemap();
    let volume = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8).volume(4);

    for (dds, images) in [(cube, 6), (volume, 4)] {
        let result = read(&dds, 0);
        assert!(
            matches!(result, Err(TextureError::UnsupportedImage { images: found }) if found == images),
            "{images} images: {result:?}"
        );
    }
}

#[test]
fn a_missing_part_is_named() {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt5, 32, 32).split(3);
    let header = dds.header_bytes();
    let mut parts = Parts::of(&dds);
    parts.0.remove(&2);

    let result = read_mip(header.as_slice(), header.len() as u64, &parts, 1);

    assert!(
        matches!(result, Err(TextureError::MissingPart { index: 2 })),
        "part 2 is missing: {result:?}"
    );
}

#[test]
fn a_header_shorter_than_its_size_is_truncated() {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).split(1);
    let header = dds.header_bytes();
    let short = header.get(..header.len() - 4).unwrap_or_default();

    let result = read_mip(short, header.len() as u64, &Parts::of(&dds), 4);

    assert!(
        matches!(result, Err(TextureError::TruncatedHeader { .. })),
        "the smallest mip is cut off: {result:?}"
    );
}

#[test]
fn a_decoded_mip_round_trips_through_an_eight_bit_rgba_png()
-> Result<(), Box<dyn std::error::Error>> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt5, 12, 8).split(1);
    let image = read(&dds, 0)?.decode()?;
    let mut png = Vec::new();

    write_png(&image, &mut png)?;

    let decoder = png::Decoder::new(io::Cursor::new(png));
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut pixels)?;
    assert_eq!((info.width, info.height), (12, 8), "the mip's size");
    assert_eq!(info.color_type, png::ColorType::Rgba, "RGBA");
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "8 bits per channel");
    assert_eq!(pixels, image.pixels(), "the decoded pixels");
    Ok(())
}
