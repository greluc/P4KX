// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Merging a split DDS header with its mip parts through a caller-supplied sibling reader.

use std::collections::BTreeMap;
use std::io;

use p4kx_testkit::{SplitDds, TestTextureFormat};
use p4kx_texture::{MergeReport, SiblingReader, TextureError, merge};

/// Parts held in memory, each with the length it claims and the bytes it delivers.
#[derive(Default)]
struct Parts(BTreeMap<u32, (u64, Vec<u8>)>);

impl Parts {
    fn of(dds: &SplitDds) -> Self {
        Self(
            (1..=dds.part_count())
                .map(|index| {
                    let data = dds.part(index);
                    (index, (data.len() as u64, data))
                })
                .collect(),
        )
    }
}

impl SiblingReader for Parts {
    type Part<'a> = &'a [u8];

    fn part_len(&self, index: u32) -> io::Result<Option<u64>> {
        Ok(self.0.get(&index).map(|(len, _)| *len))
    }

    fn open_part(&self, index: u32) -> io::Result<Self::Part<'_>> {
        self.0
            .get(&index)
            .map(|(_, data)| data.as_slice())
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
}

fn merge_with(dds: &SplitDds, parts: &Parts) -> Result<(MergeReport, Vec<u8>), TextureError> {
    let header = dds.header_bytes();
    let mut out = Vec::new();
    let report = merge(header.as_slice(), header.len() as u64, parts, &mut out)?;
    Ok((report, out))
}

fn standard_layout(dds: &SplitDds, mip_count: u32) -> Vec<u8> {
    let mut expected = dds
        .header_bytes()
        .get(..dds.header_len())
        .unwrap_or_default()
        .to_vec();
    for level in 0..mip_count {
        expected.extend_from_slice(&dds.mip(level));
    }
    expected
}

#[test]
fn a_split_texture_merges_into_header_and_mips_largest_first() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 32).split(4);

    let (report, merged) = merge_with(&dds, &Parts::of(&dds))?;

    assert_eq!(merged, standard_layout(&dds, 7), "header, then mips 0 to 6");
    assert_eq!(report.parts, 4, "four parts were read");
    assert_eq!(report.size, merged.len() as u64, "size of the merged DDS");
    Ok(())
}

#[test]
fn a_legacy_texture_with_every_mip_split_merges() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 16, 16).split(5);

    let (report, merged) = merge_with(&dds, &Parts::of(&dds))?;

    assert_eq!(merged, standard_layout(&dds, 5), "every mip from a part");
    assert_eq!(report.parts, 5, "five parts");
    Ok(())
}

#[test]
fn a_texture_without_parts_stays_as_it_is() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8);

    let (report, merged) = merge_with(&dds, &Parts::default())?;

    assert_eq!(merged, dds.header_bytes(), "unchanged");
    assert_eq!(report.parts, 0, "no parts");
    Ok(())
}

#[test]
fn an_unsplit_cubemap_stays_as_it_is() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8).cubemap();

    let (_, merged) = merge_with(&dds, &Parts::default())?;

    assert_eq!(merged, dds.header_bytes(), "unchanged");
    Ok(())
}

#[test]
fn a_missing_part_is_named() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let mut parts = Parts::of(&dds);
    parts.0.remove(&2);

    let result = merge_with(&dds, &parts);

    assert!(
        matches!(result, Err(TextureError::MissingPart { index: 2 })),
        "{result:?}"
    );
}

#[test]
fn a_short_part_is_truncated() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let mut parts = Parts::of(&dds);
    if let Some((len, data)) = parts.0.get_mut(&3) {
        data.truncate(100);
        *len = 100;
    }

    let result = merge_with(&dds, &parts);

    assert!(
        matches!(
            result,
            Err(TextureError::TruncatedPart {
                index: 3,
                expected: 4096,
                actual: 100
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_part_whose_data_ends_early_is_truncated() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let mut parts = Parts::of(&dds);
    if let Some((_, data)) = parts.0.get_mut(&1) {
        data.truncate(10);
    }

    let result = merge_with(&dds, &parts);

    assert!(
        matches!(
            result,
            Err(TextureError::TruncatedPart {
                index: 1,
                expected: 256,
                actual: 10
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_long_part_is_oversized() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let mut parts = Parts::of(&dds);
    if let Some((len, data)) = parts.0.get_mut(&2) {
        data.push(0);
        *len += 1;
    }

    let result = merge_with(&dds, &parts);

    assert!(
        matches!(
            result,
            Err(TextureError::OversizedPart {
                index: 2,
                expected: 1024,
                actual: 1025
            })
        ),
        "{result:?}"
    );
}

fn merge_with_trailer(dds: &SplitDds) -> Result<(MergeReport, Vec<u8>), TextureError> {
    merge_with_trailer_bytes(dds, *b"CExtCEnd")
}

fn merge_with_trailer_bytes(
    dds: &SplitDds,
    trailer: [u8; 8],
) -> Result<(MergeReport, Vec<u8>), TextureError> {
    let mut header = dds.header_bytes();
    header.extend_from_slice(&trailer);
    let mut out = Vec::new();
    let report = merge(
        header.as_slice(),
        header.len() as u64,
        &Parts::of(dds),
        &mut out,
    )?;
    Ok((report, out))
}

#[test]
fn the_cryengine_trailer_of_an_unsplit_cubemap_is_not_merged() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8).cubemap();

    let (report, merged) = merge_with_trailer(&dds)?;

    assert_eq!(merged, dds.header_bytes(), "the trailer is dropped");
    assert_eq!(report.parts, 0, "no parts");
    assert_eq!(report.size, merged.len() as u64, "size of the merged DDS");
    Ok(())
}

#[test]
fn the_cryengine_trailer_of_a_split_texture_is_not_merged() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 32).split(4);

    let (report, merged) = merge_with_trailer(&dds)?;

    assert_eq!(merged, standard_layout(&dds, 7), "header, then mips 0 to 6");
    assert_eq!(report.parts, 4, "four parts were read");
    assert_eq!(report.size, merged.len() as u64, "size of the merged DDS");
    Ok(())
}

#[test]
fn eight_wrong_trailer_bytes_are_rejected_when_streaming() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 32).split(4);

    let result = merge_with_trailer_bytes(&dds, *b"CExtXXXX");

    assert!(
        matches!(
            result,
            Err(TextureError::UnknownTrailer { found }) if &found == b"CExtXXXX"
        ),
        "{result:?}"
    );
}

#[test]
fn eight_wrong_trailer_bytes_are_rejected_for_several_images() {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8)
        .cubemap()
        .split(2);

    let result = merge_with_trailer_bytes(&dds, *b"CExtXXXX");

    assert!(
        matches!(
            result,
            Err(TextureError::UnknownTrailer { found }) if &found == b"CExtXXXX"
        ),
        "{result:?}"
    );
}

#[test]
fn the_cryengine_trailer_of_a_split_cubemap_is_not_merged() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8)
        .cubemap()
        .split(2);

    let (report, merged) = merge_with_trailer(&dds)?;

    assert_eq!(report.size, merged.len() as u64, "size of the merged DDS");
    assert_eq!(report.parts, 2, "two parts were read");
    Ok(())
}

#[test]
fn a_header_tail_that_is_no_whole_mip_is_rejected() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let mut header = dds.header_bytes();
    header.push(0);

    let result = merge(
        header.as_slice(),
        header.len() as u64,
        &Parts::of(&dds),
        &mut io::sink(),
    );

    assert!(
        matches!(result, Err(TextureError::TailMismatch { .. })),
        "{result:?}"
    );
}

#[test]
fn a_header_whose_data_ends_early_is_truncated() {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 64).split(3);
    let header = dds.header_bytes();
    let cut = header.get(..header.len() - 1).unwrap_or_default();

    let result = merge(cut, header.len() as u64, &Parts::of(&dds), &mut io::sink());

    assert!(
        matches!(result, Err(TextureError::TruncatedHeader { .. })),
        "{result:?}"
    );
}

#[test]
fn a_format_of_unknown_size_cannot_be_merged() {
    let format = TestTextureFormat::Dx10Block {
        dxgi_format: 200,
        block_bytes: 16,
    };
    let dds = SplitDds::new(format, 8, 8);

    let result = merge_with(&dds, &Parts::default());

    assert!(
        matches!(result, Err(TextureError::UnsupportedFormat(_))),
        "{result:?}"
    );
}

#[test]
fn the_part_count_follows_from_the_header_tail() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 64, 32).split(4);
    let header = dds.header_bytes();
    let parsed = p4kx_texture::DdsHeader::parse(&header)?;

    assert_eq!(parsed.part_count(header.len() as u64)?, 4, "four parts");
    Ok(())
}
