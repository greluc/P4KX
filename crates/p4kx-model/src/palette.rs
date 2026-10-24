// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::Material;

/// Which colour of a [`TintPalette`] a material takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaletteSlot {
    /// The primary entry, `PaletteTint="1"` in a material layer.
    Primary,
    /// The secondary entry, `PaletteTint="2"`.
    Secondary,
    /// The tertiary entry, `PaletteTint="3"`.
    Tertiary,
    /// The glass colour, taken by a glass material that uses the tint palette.
    Glass,
}

impl PaletteSlot {
    /// Returns the slot a layer's `PaletteTint` number selects, or `None` for 0 and any number
    /// above 3.
    #[must_use]
    pub const fn of_layer(number: u32) -> Option<Self> {
        match number {
            1 => Some(Self::Primary),
            2 => Some(Self::Secondary),
            3 => Some(Self::Tertiary),
            _ => None,
        }
    }
}

/// One tinted surface of a [`TintPalette`]; colours are linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TintEntry {
    /// The colour of the paint.
    pub tint: [f32; 3],
    /// The specular colour.
    pub specular: [f32; 3],
    /// The glossiness from 0 to 255.
    pub glossiness: f32,
}

impl Default for TintEntry {
    fn default() -> Self {
        Self {
            tint: [1.0; 3],
            specular: [0.04; 3],
            glossiness: 128.0,
        }
    }
}

/// The colours a livery gives the materials that take a palette (`TintPalette` of a
/// `TintPaletteTree` Record); colours are linear.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TintPalette {
    /// The palette's name.
    pub name: String,
    /// The primary entry.
    pub primary: TintEntry,
    /// The secondary entry.
    pub secondary: TintEntry,
    /// The tertiary entry.
    pub tertiary: TintEntry,
    /// The colour of the tinted glass.
    pub glass: [f32; 3],
}

impl TintPalette {
    /// Creates a palette from its entries and glass colour.
    #[must_use]
    pub fn new(name: impl Into<String>, entries: [TintEntry; 3], glass: [f32; 3]) -> Self {
        let [primary, secondary, tertiary] = entries;
        Self {
            name: name.into(),
            primary,
            secondary,
            tertiary,
            glass,
        }
    }

    /// Returns the entry of `slot`, or `None` for [`PaletteSlot::Glass`].
    #[must_use]
    pub const fn entry(&self, slot: PaletteSlot) -> Option<&TintEntry> {
        match slot {
            PaletteSlot::Primary => Some(&self.primary),
            PaletteSlot::Secondary => Some(&self.secondary),
            PaletteSlot::Tertiary => Some(&self.tertiary),
            PaletteSlot::Glass => None,
        }
    }

    /// Returns `material` drawn with this palette, or `None` when the material takes none.
    ///
    /// A painted material gets the entry's colour as its diffuse colour, the entry's specular
    /// colour and its glossiness as the shininess; a glass material gets the glass colour as its
    /// diffuse colour. The result is [`Material::tinted`].
    #[must_use]
    pub fn apply(&self, material: &Material) -> Option<Material> {
        let slot = material.palette?;
        let mut tinted = material.clone();
        match self.entry(slot) {
            Some(entry) => {
                tinted.diffuse = entry.tint;
                tinted.specular = entry.specular;
                tinted.shininess = entry.glossiness;
            }
            None => tinted.diffuse = self.glass,
        }
        tinted.tinted = true;
        Some(tinted)
    }
}

/// Converts an 8-bit sRGB channel to a linear value from 0 to 1.
#[must_use]
pub fn srgb8_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}
