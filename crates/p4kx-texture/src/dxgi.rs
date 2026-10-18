use crate::BlockLayout;

const fn pixel(bytes: u32) -> BlockLayout {
    BlockLayout {
        block_width: 1,
        block_height: 1,
        bytes_per_block: bytes,
    }
}

const fn block(bytes: u32) -> BlockLayout {
    BlockLayout {
        block_width: 4,
        block_height: 4,
        bytes_per_block: bytes,
    }
}

/// The DXGI formats this version knows by name and size.
const FORMATS: &[(u32, &str, BlockLayout)] = &[
    (1, "R32G32B32A32_TYPELESS", pixel(16)),
    (2, "R32G32B32A32_FLOAT", pixel(16)),
    (3, "R32G32B32A32_UINT", pixel(16)),
    (4, "R32G32B32A32_SINT", pixel(16)),
    (5, "R32G32B32_TYPELESS", pixel(12)),
    (6, "R32G32B32_FLOAT", pixel(12)),
    (7, "R32G32B32_UINT", pixel(12)),
    (8, "R32G32B32_SINT", pixel(12)),
    (9, "R16G16B16A16_TYPELESS", pixel(8)),
    (10, "R16G16B16A16_FLOAT", pixel(8)),
    (11, "R16G16B16A16_UNORM", pixel(8)),
    (12, "R16G16B16A16_UINT", pixel(8)),
    (13, "R16G16B16A16_SNORM", pixel(8)),
    (14, "R16G16B16A16_SINT", pixel(8)),
    (15, "R32G32_TYPELESS", pixel(8)),
    (16, "R32G32_FLOAT", pixel(8)),
    (17, "R32G32_UINT", pixel(8)),
    (18, "R32G32_SINT", pixel(8)),
    (23, "R10G10B10A2_TYPELESS", pixel(4)),
    (24, "R10G10B10A2_UNORM", pixel(4)),
    (25, "R10G10B10A2_UINT", pixel(4)),
    (26, "R11G11B10_FLOAT", pixel(4)),
    (27, "R8G8B8A8_TYPELESS", pixel(4)),
    (28, "R8G8B8A8_UNORM", pixel(4)),
    (29, "R8G8B8A8_UNORM_SRGB", pixel(4)),
    (30, "R8G8B8A8_UINT", pixel(4)),
    (31, "R8G8B8A8_SNORM", pixel(4)),
    (32, "R8G8B8A8_SINT", pixel(4)),
    (33, "R16G16_TYPELESS", pixel(4)),
    (34, "R16G16_FLOAT", pixel(4)),
    (35, "R16G16_UNORM", pixel(4)),
    (36, "R16G16_UINT", pixel(4)),
    (37, "R16G16_SNORM", pixel(4)),
    (38, "R16G16_SINT", pixel(4)),
    (39, "R32_TYPELESS", pixel(4)),
    (41, "R32_FLOAT", pixel(4)),
    (42, "R32_UINT", pixel(4)),
    (43, "R32_SINT", pixel(4)),
    (48, "R8G8_TYPELESS", pixel(2)),
    (49, "R8G8_UNORM", pixel(2)),
    (50, "R8G8_UINT", pixel(2)),
    (51, "R8G8_SNORM", pixel(2)),
    (52, "R8G8_SINT", pixel(2)),
    (53, "R16_TYPELESS", pixel(2)),
    (54, "R16_FLOAT", pixel(2)),
    (56, "R16_UNORM", pixel(2)),
    (57, "R16_UINT", pixel(2)),
    (58, "R16_SNORM", pixel(2)),
    (59, "R16_SINT", pixel(2)),
    (60, "R8_TYPELESS", pixel(1)),
    (61, "R8_UNORM", pixel(1)),
    (62, "R8_UINT", pixel(1)),
    (63, "R8_SNORM", pixel(1)),
    (64, "R8_SINT", pixel(1)),
    (65, "A8_UNORM", pixel(1)),
    (67, "R9G9B9E5_SHAREDEXP", pixel(4)),
    (70, "BC1_TYPELESS", block(8)),
    (71, "BC1_UNORM", block(8)),
    (72, "BC1_UNORM_SRGB", block(8)),
    (73, "BC2_TYPELESS", block(16)),
    (74, "BC2_UNORM", block(16)),
    (75, "BC2_UNORM_SRGB", block(16)),
    (76, "BC3_TYPELESS", block(16)),
    (77, "BC3_UNORM", block(16)),
    (78, "BC3_UNORM_SRGB", block(16)),
    (79, "BC4_TYPELESS", block(8)),
    (80, "BC4_UNORM", block(8)),
    (81, "BC4_SNORM", block(8)),
    (82, "BC5_TYPELESS", block(16)),
    (83, "BC5_UNORM", block(16)),
    (84, "BC5_SNORM", block(16)),
    (85, "B5G6R5_UNORM", pixel(2)),
    (86, "B5G5R5A1_UNORM", pixel(2)),
    (87, "B8G8R8A8_UNORM", pixel(4)),
    (88, "B8G8R8X8_UNORM", pixel(4)),
    (90, "B8G8R8A8_TYPELESS", pixel(4)),
    (91, "B8G8R8A8_UNORM_SRGB", pixel(4)),
    (92, "B8G8R8X8_TYPELESS", pixel(4)),
    (93, "B8G8R8X8_UNORM_SRGB", pixel(4)),
    (94, "BC6H_TYPELESS", block(16)),
    (95, "BC6H_UF16", block(16)),
    (96, "BC6H_SF16", block(16)),
    (97, "BC7_TYPELESS", block(16)),
    (98, "BC7_UNORM", block(16)),
    (99, "BC7_UNORM_SRGB", block(16)),
    (115, "B4G4R4A4_UNORM", pixel(2)),
];

/// A DXGI format code as a DX10 header stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DxgiFormat(pub u32);

impl DxgiFormat {
    /// BC1, as legacy `DXT1` headers store it.
    pub const BC1_UNORM: Self = Self(71);
    /// BC2, as legacy `DXT2` and `DXT3` headers store it.
    pub const BC2_UNORM: Self = Self(74);
    /// BC3, as legacy `DXT4` and `DXT5` headers store it.
    pub const BC3_UNORM: Self = Self(77);
    /// BC4 unsigned, as legacy `ATI1` and `BC4U` headers store it.
    pub const BC4_UNORM: Self = Self(80);
    /// BC4 signed, as legacy `BC4S` headers store it.
    pub const BC4_SNORM: Self = Self(81);
    /// BC5 unsigned, as legacy `ATI2` and `BC5U` headers store it.
    pub const BC5_UNORM: Self = Self(83);
    /// BC5 signed, as legacy `BC5S` headers store it.
    pub const BC5_SNORM: Self = Self(84);

    /// Returns the DXGI name, e.g. `BC7_UNORM_SRGB`, or `None` for a code this version does not
    /// know.
    #[must_use]
    pub fn name(self) -> Option<&'static str> {
        lookup(self.0).map(|(name, _)| name)
    }

    /// Returns how pixels are stored, or `None` for a code this version does not know.
    #[must_use]
    pub fn layout(self) -> Option<BlockLayout> {
        lookup(self.0).map(|(_, layout)| layout)
    }
}

fn lookup(code: u32) -> Option<(&'static str, BlockLayout)> {
    FORMATS
        .iter()
        .find(|(known, ..)| *known == code)
        .map(|&(_, name, layout)| (name, layout))
}
