//! Alpha and smoothness tails: reading the headerless tail texture and merging it into the alpha
//! channel of the decoded colour.

use std::collections::BTreeMap;
use std::io;

use p4kx_testkit::{SplitDds, TestTextureFormat};
use p4kx_texture::{
    DxgiFormat, RgbaImage, SiblingReader, TextureError, TextureFormat, decode, merge_alpha,
    read_mip,
};

struct Parts(BTreeMap<u32, Vec<u8>>);

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

const R8: TextureFormat = TextureFormat::Dxgi(DxgiFormat(61));
const A8: TextureFormat = TextureFormat::Dxgi(DxgiFormat(65));

fn colour() -> Result<RgbaImage, TextureError> {
    RgbaImage::new(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8])
}

#[test]
fn a_single_channel_tail_becomes_the_alpha_channel() -> Result<(), TextureError> {
    let mut image = colour()?;
    let alpha = decode(R8, 2, 1, &[0x10, 0xF0])?;

    merge_alpha(&mut image, &alpha, R8)?;

    assert_eq!(
        image.pixels(),
        [1, 2, 3, 0x10, 5, 6, 7, 0xF0],
        "red of the tail into alpha, colour kept"
    );
    Ok(())
}

#[test]
fn an_alpha_only_tail_keeps_its_alpha() -> Result<(), TextureError> {
    let mut image = colour()?;
    let alpha = decode(A8, 2, 1, &[0x22, 0x99])?;

    merge_alpha(&mut image, &alpha, A8)?;

    assert_eq!(
        image.pixels(),
        [1, 2, 3, 0x22, 5, 6, 7, 0x99],
        "alpha of the tail"
    );
    Ok(())
}

#[test]
fn a_tail_of_another_size_is_rejected() -> Result<(), TextureError> {
    let mut image = colour()?;
    let alpha = decode(R8, 1, 2, &[0, 0])?;

    let result = merge_alpha(&mut image, &alpha, R8);

    assert!(
        matches!(
            result,
            Err(TextureError::AlphaTailSize {
                width: 1,
                height: 2,
                expected_width: 2,
                expected_height: 1
            })
        ),
        "{result:?}"
    );
    assert_eq!(image.pixels(), colour()?.pixels(), "the image is unchanged");
    Ok(())
}

#[test]
fn the_headerless_split_tail_reads_like_any_texture() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 8)
        .split(2)
        .alpha(TestTextureFormat::R8);
    let Some(tail) = dds.alpha_tail() else {
        panic!("the texture has an alpha tail");
    };
    let header = tail.header_bytes();
    assert_ne!(header.get(..4), Some(b"DDS ".as_slice()), "no magic");
    let parts = Parts(
        (1..=tail.part_count())
            .map(|index| (index, tail.part(index)))
            .collect(),
    );

    for level in 0..4 {
        let mip = read_mip(header.as_slice(), header.len() as u64, &parts, level)?;
        assert_eq!(mip.data(), tail.mip(level).as_slice(), "mip {level}");
        assert_eq!(mip.header().format(), R8, "R8");
    }
    Ok(())
}
