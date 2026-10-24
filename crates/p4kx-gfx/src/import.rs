// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::GfxError;
use crate::geometry::Rect;
use crate::tag::Tag;

/// The most assets a movie imports, as many as there are IDs.
pub const MAX_IMPORTS: usize = 1 << 16;

/// The assets a movie imports from one other movie, of an `ImportAssets` or `ImportAssets2` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The URL of the exporting movie as the tag states it.
    pub url: String,
    /// The assets taken from it, each under the ID it has in this movie.
    pub assets: Vec<ImportedAsset>,
}

/// One asset imported from another movie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAsset {
    /// The ID the asset has in the importing movie.
    pub id: u16,
    /// The name the exporting movie exports it under.
    pub name: String,
}

/// Reads an `ImportAssets` (57) or `ImportAssets2` (71) tag, adding its assets to `total`.
pub(crate) fn read_import(tag: &Tag<'_>, total: &mut usize) -> Result<Import, GfxError> {
    let mut reader = tag.reader();
    let url = reader.c_string()?.into_owned();
    if tag.code == 71 {
        reader.u16()?;
    }
    let count = usize::from(reader.u16()?);
    let mut assets = Vec::with_capacity(count.min(reader.remaining() / 3));
    for _ in 0..count {
        *total += 1;
        if *total > MAX_IMPORTS {
            return Err(GfxError::TooManyImports { limit: MAX_IMPORTS });
        }
        assets.push(ImportedAsset {
            id: reader.u16()?,
            name: reader.c_string()?.into_owned(),
        });
    }
    Ok(Import { url, assets })
}

/// A name a movie binds to a class, of a `SymbolClass` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolClass {
    /// The ID of the definition; 0 is the main timeline.
    pub id: u16,
    /// The class name.
    pub name: String,
}

/// Reads the names of a `SymbolClass` tag.
pub(crate) fn read_symbol_classes(tag: &Tag<'_>) -> Result<Vec<SymbolClass>, GfxError> {
    let mut reader = tag.reader();
    let count = usize::from(reader.u16()?);
    let mut symbols = Vec::with_capacity(count.min(reader.remaining() / 3));
    for _ in 0..count {
        symbols.push(SymbolClass {
            id: reader.u16()?,
            name: reader.c_string()?.into_owned(),
        });
    }
    Ok(symbols)
}

/// The nine-slice grid of a sprite or button, of a `DefineScalingGrid` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalingGrid {
    /// The ID of the definition.
    pub id: u16,
    /// The centre rectangle that scales; the border around it keeps its size.
    pub splitter: Rect,
}

/// Reads a `DefineScalingGrid` tag.
pub(crate) fn read_scaling_grid(tag: &Tag<'_>) -> Result<ScalingGrid, GfxError> {
    let mut reader = tag.reader();
    Ok(ScalingGrid {
        id: reader.u16()?,
        splitter: reader.rect()?,
    })
}
