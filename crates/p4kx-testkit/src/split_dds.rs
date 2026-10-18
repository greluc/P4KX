use std::io;
use std::path::{Path, PathBuf};

use crate::TestEntry;
use crate::bytes::put_u32;

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: u32 = 124;
const PIXEL_FORMAT_SIZE: u32 = 32;
const FLAGS: u32 = 0x0002_1007;
const PF_FOURCC: u32 = 0x4;
const PF_RGB: u32 = 0x40;
const PF_ALPHA_PIXELS: u32 = 0x1;
const CAPS_TEXTURE: u32 = 0x1000;
const CAPS_COMPLEX_MIPMAP: u32 = 0x0040_0008;
const CAPS2_CUBEMAP_ALL_FACES: u32 = 0xFE00;
const CAPS2_VOLUME: u32 = 0x0020_0000;
const DIMENSION_TEXTURE2D: u32 = 3;
const DIMENSION_TEXTURE3D: u32 = 4;
const MISC_TEXTURECUBE: u32 = 0x4;

/// The pixel format of a synthetic DDS texture and how its header encodes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTextureFormat {
    /// BC1 in a legacy header with the `FourCC` `DXT1`.
    LegacyDxt1,
    /// BC3 in a legacy header with the `FourCC` `DXT5`.
    LegacyDxt5,
    /// 32-bit BGRA in a legacy header with RGB and alpha bit masks.
    LegacyBgra8,
    /// 24-bit BGR in a legacy header with RGB bit masks.
    LegacyBgr8,
    /// A DX10 header with the given DXGI format code of an uncompressed format.
    Dx10Pixel {
        /// The DXGI format code.
        dxgi_format: u32,
        /// Bytes per pixel.
        pixel_bytes: u32,
    },
    /// A DX10 header with the given DXGI format code and bytes per 4×4 block.
    Dx10Block {
        /// The DXGI format code.
        dxgi_format: u32,
        /// Bytes per 4×4 block.
        block_bytes: u32,
    },
}

impl TestTextureFormat {
    /// BC7 sRGB in a DX10 header.
    pub const BC7_SRGB: Self = Self::Dx10Block {
        dxgi_format: 99,
        block_bytes: 16,
    };

    const fn is_dx10(self) -> bool {
        matches!(self, Self::Dx10Block { .. } | Self::Dx10Pixel { .. })
    }

    fn mip_len(self, width: u32, height: u32) -> usize {
        let blocks = |side: u32| side.div_ceil(4) as usize;
        match self {
            Self::LegacyDxt1 => blocks(width) * blocks(height) * 8,
            Self::LegacyDxt5 => blocks(width) * blocks(height) * 16,
            Self::LegacyBgra8 => width as usize * height as usize * 4,
            Self::LegacyBgr8 => width as usize * height as usize * 3,
            Self::Dx10Pixel { pixel_bytes, .. } => {
                width as usize * height as usize * pixel_bytes as usize
            }
            Self::Dx10Block { block_bytes, .. } => {
                blocks(width) * blocks(height) * block_bytes as usize
            }
        }
    }
}

/// A synthetic DDS texture split the way the Archive stores it: a header Entry holding the DDS
/// header and the smallest mips, and one sibling `.dds.N` per larger mip, `.dds.1` holding the
/// smallest split mip.
///
/// Every mip is filled with a pattern unique to its level, so a merged file shows where each mip
/// landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitDds {
    format: TestTextureFormat,
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
    split_mips: u32,
    array_size: u32,
    cubemap: bool,
}

impl SplitDds {
    /// Creates a 2D texture with a full mip chain and no split parts.
    #[must_use]
    pub fn new(format: TestTextureFormat, width: u32, height: u32) -> Self {
        let largest = width.max(height).max(1);
        Self {
            format,
            width,
            height,
            depth: 1,
            mip_count: largest.bit_width(),
            split_mips: 0,
            array_size: 1,
            cubemap: false,
        }
    }

    /// Sets the number of mips in the header; the full chain is kept when larger.
    #[must_use]
    pub fn mips(mut self, count: u32) -> Self {
        self.mip_count = count.clamp(1, self.mip_count);
        self
    }

    /// Moves the `count` largest mips into sibling parts.
    #[must_use]
    pub fn split(mut self, count: u32) -> Self {
        self.split_mips = count.min(self.mip_count);
        self
    }

    /// Marks the texture as a volume of `depth` slices.
    #[must_use]
    pub const fn volume(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Marks the texture as a cubemap with all six faces.
    #[must_use]
    pub const fn cubemap(mut self) -> Self {
        self.cubemap = true;
        self
    }

    /// Sets the DX10 array size.
    #[must_use]
    pub const fn array(mut self, size: u32) -> Self {
        self.array_size = size;
        self
    }

    /// Returns the number of sibling parts.
    #[must_use]
    pub const fn part_count(&self) -> u32 {
        self.split_mips
    }

    /// Returns the length of the DDS header including the magic and any DX10 extension.
    #[must_use]
    pub const fn header_len(&self) -> usize {
        if self.format.is_dx10() { 148 } else { 128 }
    }

    /// Returns the data of mip `level`, all faces and slices.
    #[must_use]
    pub fn mip(&self, level: u32) -> Vec<u8> {
        let width = (self.width >> level).max(1);
        let height = (self.height >> level).max(1);
        let depth = (self.depth >> level).max(1);
        let faces = (if self.cubemap { 6 } else { 1 }) * self.array_size;
        let len = self.format.mip_len(width, height) * (depth * faces) as usize;
        let seed = level.wrapping_mul(37).wrapping_add(11);
        std::iter::successors(Some(seed), |value| Some(value.wrapping_add(7)))
            .map(|value| u8::try_from(value & 0xFF).unwrap_or_default())
            .take(len)
            .collect()
    }

    /// Returns the header Entry: the DDS header followed by the mips that are not split.
    #[must_use]
    pub fn header_bytes(&self) -> Vec<u8> {
        let mut out = self.dds_header();
        for level in self.split_mips..self.mip_count {
            out.extend_from_slice(&self.mip(level));
        }
        out
    }

    /// Returns the data of sibling part `index`, counted from 1 for the smallest split mip.
    #[must_use]
    pub fn part(&self, index: u32) -> Vec<u8> {
        self.mip(self.split_mips.saturating_sub(index))
    }

    /// Returns the header and its parts as Entries below `header_path`.
    #[must_use]
    pub fn entries(&self, header_path: &str) -> Vec<TestEntry> {
        let mut entries = vec![TestEntry::stored(header_path, self.header_bytes())];
        entries.extend(
            (1..=self.split_mips)
                .map(|index| TestEntry::zstd(format!("{header_path}.{index}"), self.part(index))),
        );
        entries
    }

    /// Writes the header file and its parts into `folder`, returning the header's path.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of writing a file.
    pub fn write_files(&self, folder: &Path, name: &str) -> io::Result<PathBuf> {
        let header = folder.join(name);
        std::fs::write(&header, self.header_bytes())?;
        for index in 1..=self.split_mips {
            std::fs::write(folder.join(format!("{name}.{index}")), self.part(index))?;
        }
        Ok(header)
    }

    fn dds_header(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.header_len());
        out.extend_from_slice(MAGIC);
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
