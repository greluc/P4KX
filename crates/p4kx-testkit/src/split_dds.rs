// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod header;

use std::io;
use std::path::{Path, PathBuf};

use crate::TestEntry;

const CUBE_FACES: u32 = 6;
const ALPHA_SEED: u32 = 0x55;

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
    /// 8-bit luminance and alpha in a legacy header with luminance and alpha bit masks.
    LegacyL8A8,
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

    /// 8-bit single-channel `R8_UNORM` in a DX10 header.
    pub const R8: Self = Self::Dx10Pixel {
        dxgi_format: 61,
        pixel_bytes: 1,
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
            Self::LegacyL8A8 => width as usize * height as usize * 2,
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
/// A texture with several images per mip, a cubemap's faces or an array's elements, keeps each
/// image's smallest mips together in the header Entry, while a part holds one mip of every image,
/// one after the other. An alpha tail is a second split texture of the same size: its header Entry
/// `.dds.a` has no `DDS ` magic and its parts are `.dds.1a` … `.dds.Na`.
///
/// Every mip of every image is filled with a pattern unique to its image and level, so a merged
/// file shows where each one landed.
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
    magic: bool,
    seed: u32,
    alpha: Option<TestTextureFormat>,
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
            magic: true,
            seed: 0,
            alpha: None,
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

    /// Adds an alpha tail in `format` with the texture's size, mips and split.
    #[must_use]
    pub const fn alpha(mut self, format: TestTextureFormat) -> Self {
        self.alpha = Some(format);
        self
    }

    /// Returns the alpha tail as a split texture of its own, or `None` when there is none.
    #[must_use]
    pub fn alpha_tail(&self) -> Option<Self> {
        self.alpha.map(|format| Self {
            format,
            magic: false,
            seed: ALPHA_SEED,
            alpha: None,
            ..self.clone()
        })
    }

    /// Returns the number of sibling parts.
    #[must_use]
    pub const fn part_count(&self) -> u32 {
        self.split_mips
    }

    /// Returns the number of images per mip: cubemap faces times array elements.
    #[must_use]
    pub const fn surfaces(&self) -> u32 {
        (if self.cubemap { CUBE_FACES } else { 1 }) * self.array_size
    }

    /// Returns the length of the DDS header including any magic and DX10 extension.
    #[must_use]
    pub const fn header_len(&self) -> usize {
        let magic = if self.magic { 4 } else { 0 };
        magic + if self.format.is_dx10() { 144 } else { 124 }
    }

    /// Returns the data of mip `level` of image `surface`, all slices.
    #[must_use]
    pub fn surface_mip(&self, surface: u32, level: u32) -> Vec<u8> {
        let width = (self.width >> level).max(1);
        let height = (self.height >> level).max(1);
        let depth = (self.depth >> level).max(1);
        let len = self.format.mip_len(width, height) * depth as usize;
        let seed = level
            .wrapping_mul(37)
            .wrapping_add(11)
            .wrapping_add(surface.wrapping_mul(101))
            .wrapping_add(self.seed);
        std::iter::successors(Some(seed), |value| Some(value.wrapping_add(7)))
            .map(|value| u8::try_from(value & 0xFF).unwrap_or_default())
            .take(len)
            .collect()
    }

    /// Returns the data of mip `level` of every image, one image after the other, as a part
    /// holds it.
    #[must_use]
    pub fn mip(&self, level: u32) -> Vec<u8> {
        (0..self.surfaces())
            .flat_map(|surface| self.surface_mip(surface, level))
            .collect()
    }

    /// Returns the header Entry: the DDS header followed, image by image, by the mips that are
    /// not split.
    #[must_use]
    pub fn header_bytes(&self) -> Vec<u8> {
        self.with_mips(self.split_mips)
    }

    /// Returns the texture as one standard DDS file: the header followed, image by image, by
    /// every mip from the largest to the smallest.
    #[must_use]
    pub fn merged_bytes(&self) -> Vec<u8> {
        self.with_mips(0)
    }

    /// Returns the data of sibling part `index`, counted from 1 for the smallest split mip.
    #[must_use]
    pub fn part(&self, index: u32) -> Vec<u8> {
        self.mip(self.split_mips.saturating_sub(index))
    }

    /// Returns the header, its parts and any alpha tail as Entries below `header_path`.
    #[must_use]
    pub fn entries(&self, header_path: &str) -> Vec<TestEntry> {
        let mut entries = vec![TestEntry::stored(header_path, self.header_bytes())];
        entries.extend(
            (1..=self.split_mips)
                .map(|index| TestEntry::zstd(format!("{header_path}.{index}"), self.part(index))),
        );
        if let Some(alpha) = self.alpha_tail() {
            entries.push(TestEntry::stored(
                format!("{header_path}.a"),
                alpha.header_bytes(),
            ));
            entries.extend((1..=alpha.split_mips).map(|index| {
                TestEntry::zstd(format!("{header_path}.{index}a"), alpha.part(index))
            }));
        }
        entries
    }

    /// Writes the header file, its parts and any alpha tail into `folder`, returning the header's
    /// path.
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
        if let Some(alpha) = self.alpha_tail() {
            std::fs::write(folder.join(format!("{name}.a")), alpha.header_bytes())?;
            for index in 1..=alpha.split_mips {
                std::fs::write(folder.join(format!("{name}.{index}a")), alpha.part(index))?;
            }
        }
        Ok(header)
    }

    fn with_mips(&self, first: u32) -> Vec<u8> {
        let mut out = self.dds_header();
        for surface in 0..self.surfaces() {
            for level in first..self.mip_count {
                out.extend_from_slice(&self.surface_mip(surface, level));
            }
        }
        out
    }
}
