//! Parsing legacy and DX10 DDS headers.

use p4kx_testkit::{SplitDds, TestTextureFormat};
use p4kx_texture::{DdsHeader, TextureError, TextureFormat};

fn parse(dds: &SplitDds) -> Result<DdsHeader, TextureError> {
    DdsHeader::parse(&dds.header_bytes())
}

#[test]
fn a_legacy_dxt1_header_is_bc1() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::LegacyDxt1, 256, 128))?;

    assert_eq!(header.format().to_string(), "BC1_UNORM", "DXT1 is BC1");
    assert_eq!((header.width(), header.height()), (256, 128), "size");
    assert_eq!(header.depth(), 1, "a 2D texture has depth 1");
    assert_eq!(header.mip_count(), 9, "full chain of a 256 texture");
    assert_eq!(header.array_size(), 1, "no array");
    assert_eq!(header.faces(), 1, "no cubemap");
    assert!(!header.is_cubemap(), "not a cubemap");
    assert_eq!(header.header_len(), 128, "magic and legacy header");
    assert_eq!(header.mip_size(0), Some(64 * 32 * 8), "BC1 blocks of mip 0");
    assert_eq!(header.mip_size(8), Some(8), "a 1x1 mip is one block");
    Ok(())
}

#[test]
fn a_legacy_dxt5_header_is_bc3() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::LegacyDxt5, 64, 64).mips(3))?;

    assert_eq!(header.format().to_string(), "BC3_UNORM", "DXT5 is BC3");
    assert_eq!(header.mip_count(), 3, "the header's mip count");
    Ok(())
}

#[test]
fn a_legacy_header_with_bit_masks_is_bgra8() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::LegacyBgra8, 16, 8))?;

    assert_eq!(
        header.format().to_string(),
        "B8G8R8A8_UNORM",
        "masks name BGRA"
    );
    assert_eq!(header.mip_size(0), Some(16 * 8 * 4), "4 bytes per pixel");
    assert_eq!(header.mip_size(4), Some(4), "the last mip is one pixel");
    Ok(())
}

#[test]
fn a_dx10_header_carries_its_dxgi_format() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::BC7_SRGB, 4096, 4096).mips(11))?;

    assert_eq!(header.format().to_string(), "BC7_UNORM_SRGB", "DXGI 99");
    assert_eq!(header.header_len(), 148, "magic, header and DX10 extension");
    assert_eq!(header.mip_count(), 11, "mip count");
    assert_eq!(header.mip_size(0), Some(16 << 20), "one byte per pixel");
    Ok(())
}

#[test]
fn a_legacy_cubemap_has_six_faces() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::LegacyDxt1, 32, 32).cubemap())?;

    assert!(header.is_cubemap(), "cubemap");
    assert_eq!(header.faces(), 6, "all six faces");
    assert_eq!(
        header.mip_size(0),
        Some(6 * 64 * 8),
        "a mip holds every face"
    );
    Ok(())
}

#[test]
fn a_dx10_cubemap_array_has_six_faces_per_element() -> Result<(), TextureError> {
    let dds = SplitDds::new(TestTextureFormat::BC7_SRGB, 16, 16)
        .cubemap()
        .array(2);
    let header = parse(&dds)?;

    assert_eq!(header.faces(), 6, "cubemap faces");
    assert_eq!(header.array_size(), 2, "array size");
    assert_eq!(
        header.mip_size(0),
        Some(12 * 16 * 16),
        "every face of every element"
    );
    Ok(())
}

#[test]
fn a_volume_texture_has_its_depth() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8).volume(4))?;

    assert_eq!(header.depth(), 4, "volume depth");
    assert_eq!(
        header.mip_size(1),
        Some(2 * 16),
        "the depth halves per mip too"
    );
    Ok(())
}

#[test]
fn an_unknown_dxgi_format_is_named_by_its_code() -> Result<(), TextureError> {
    let format = TestTextureFormat::Dx10Block {
        dxgi_format: 200,
        block_bytes: 16,
    };
    let header = parse(&SplitDds::new(format, 8, 8))?;

    assert_eq!(header.format().to_string(), "DXGI format 200", "unnamed");
    assert!(matches!(header.format(), TextureFormat::Dxgi(_)), "DXGI");
    assert_eq!(header.mip_size(0), None, "size unknown");
    Ok(())
}

#[test]
fn a_header_without_magic_parses_like_an_alpha_tail() -> Result<(), TextureError> {
    let bytes = SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8).header_bytes();
    let header = DdsHeader::parse(bytes.get(4..).unwrap_or_default())?;

    assert_eq!(header.header_len(), 144, "no magic");
    assert_eq!(header.width(), 8, "size");
    Ok(())
}

#[test]
fn other_data_is_not_a_dds_header() {
    let result = DdsHeader::parse(b"PNG\x0d\x0a\x1a\x0a and the rest of a picture file .....");

    assert!(matches!(result, Err(TextureError::NotDds)), "{result:?}");
}

#[test]
fn a_cut_header_is_truncated() {
    let bytes = SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8).header_bytes();
    let result = DdsHeader::parse(bytes.get(..140).unwrap_or_default());

    assert!(
        matches!(
            result,
            Err(TextureError::TruncatedHeader {
                len: 140,
                needed: 148
            })
        ),
        "{result:?}"
    );
}

#[test]
fn more_mips_than_the_size_allows_are_invalid() {
    let mut bytes = SplitDds::new(TestTextureFormat::LegacyDxt1, 8, 8).header_bytes();
    if let Some(mip_count) = bytes.get_mut(28..32) {
        mip_count.copy_from_slice(&40_u32.to_le_bytes());
    }

    let result = DdsHeader::parse(&bytes);

    assert!(
        matches!(result, Err(TextureError::InvalidHeader(_))),
        "{result:?}"
    );
}

#[test]
fn a_size_that_overflows_is_invalid() {
    let mut bytes = SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8)
        .mips(1)
        .header_bytes();
    for range in [12..16, 16..20] {
        if let Some(side) = bytes.get_mut(range) {
            side.copy_from_slice(&u32::MAX.to_le_bytes());
        }
    }
    if let Some(array_size) = bytes.get_mut(140..144) {
        array_size.copy_from_slice(&u32::MAX.to_le_bytes());
    }

    let result = DdsHeader::parse(&bytes);

    assert!(
        matches!(result, Err(TextureError::InvalidHeader(_))),
        "{result:?}"
    );
}

#[test]
fn a_legacy_luminance_alpha_header_is_l8a8() -> Result<(), TextureError> {
    let header = parse(&SplitDds::new(TestTextureFormat::LegacyL8A8, 8, 8))?;

    assert_eq!(header.format(), TextureFormat::L8A8, "luminance and alpha");
    assert_eq!(header.format().to_string(), "L8A8_UNORM", "its name");
    assert_eq!(header.mip_size(0), Some(128), "two bytes per pixel");
    Ok(())
}
