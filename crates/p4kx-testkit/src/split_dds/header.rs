use super::{SplitDds, TestTextureFormat};
use crate::bytes::put_u32;

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: u32 = 124;
const PIXEL_FORMAT_SIZE: u32 = 32;
const FLAGS: u32 = 0x0002_1007;
const PF_FOURCC: u32 = 0x4;
const PF_RGB: u32 = 0x40;
const PF_ALPHA_PIXELS: u32 = 0x1;
const PF_LUMINANCE: u32 = 0x2_0000;
const CAPS_TEXTURE: u32 = 0x1000;
const CAPS_COMPLEX_MIPMAP: u32 = 0x0040_0008;
const CAPS2_CUBEMAP_ALL_FACES: u32 = 0xFE00;
const CAPS2_VOLUME: u32 = 0x0020_0000;
const DIMENSION_TEXTURE2D: u32 = 3;
const DIMENSION_TEXTURE3D: u32 = 4;
const MISC_TEXTURECUBE: u32 = 0x4;

impl SplitDds {
    pub(super) fn dds_header(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header_len());
        if self.magic {
            out.extend_from_slice(MAGIC);
        }
        for value in [
            HEADER_SIZE,
            FLAGS,
            self.height,
            self.width,
            0,
            if self.depth > 1 { self.depth } else { 0 },
            self.mip_count,
        ] {
            put_u32(&mut out, value);
        }
        out.extend_from_slice(&[0; 44]);
        self.put_pixel_format(&mut out);
        let caps2 = if self.cubemap && !self.format.is_dx10() {
            CAPS2_CUBEMAP_ALL_FACES
        } else if self.depth > 1 {
            CAPS2_VOLUME
        } else {
            0
        };
        for value in [CAPS_TEXTURE | CAPS_COMPLEX_MIPMAP, caps2, 0, 0, 0] {
            put_u32(&mut out, value);
        }
        if let TestTextureFormat::Dx10Block { dxgi_format, .. }
        | TestTextureFormat::Dx10Pixel { dxgi_format, .. } = self.format
        {
            let dimension = if self.depth > 1 {
                DIMENSION_TEXTURE3D
            } else {
                DIMENSION_TEXTURE2D
            };
            let misc = if self.cubemap { MISC_TEXTURECUBE } else { 0 };
            for value in [dxgi_format, dimension, misc, self.array_size, 0] {
                put_u32(&mut out, value);
            }
        }
        out
    }

    fn put_pixel_format(&self, out: &mut Vec<u8>) {
        put_u32(out, PIXEL_FORMAT_SIZE);
        let (flags, four_cc, bits, masks) = match self.format {
            TestTextureFormat::LegacyDxt1 => (PF_FOURCC, *b"DXT1", 0, [0; 4]),
            TestTextureFormat::LegacyDxt5 => (PF_FOURCC, *b"DXT5", 0, [0; 4]),
            TestTextureFormat::LegacyBgra8 => (
                PF_RGB | PF_ALPHA_PIXELS,
                [0; 4],
                32,
                [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000],
            ),
            TestTextureFormat::LegacyBgr8 => (
                PF_RGB,
                [0; 4],
                24,
                [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0],
            ),
            TestTextureFormat::LegacyL8A8 => (
                PF_LUMINANCE | PF_ALPHA_PIXELS,
                [0; 4],
                16,
                [0x0000_00FF, 0, 0, 0x0000_FF00],
            ),
            TestTextureFormat::Dx10Block { .. } | TestTextureFormat::Dx10Pixel { .. } => {
                (PF_FOURCC, *b"DX10", 0, [0; 4])
            }
        };
        put_u32(out, flags);
        out.extend_from_slice(&four_cc);
        put_u32(out, bits);
        for mask in masks {
            put_u32(out, mask);
        }
    }
}
