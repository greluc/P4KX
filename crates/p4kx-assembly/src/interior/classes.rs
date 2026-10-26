// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{DataForge, Guid};

use crate::entities::is_entity;
use crate::spec::read_geometry;
use crate::{ForgePalettes, Geometry, PaletteProvider};

/// What an Interior asks of the DataForge: the geometry a class Record names and the tint
/// palettes.
pub(super) trait ClassLookup {
    fn geometry(&self, guid: Guid) -> Option<Geometry>;

    fn palettes(&self) -> &dyn PaletteProvider;
}

/// The DataForge an [`Interior`](crate::Interior) looks the class Records of its entities up in.
///
/// With it, entities that name only a class Record (`EntityClassGUID`), such as doors, panels and
/// cabinets, are placed as props with the geometry, material library and tint palette of that
/// Record, and the tint palettes the `.soc` file chooses are applied.
#[derive(Debug, Clone, Copy)]
pub struct InteriorForge<'f> {
    forge: &'f DataForge<'f>,
    palettes: ForgePalettes<'f>,
}

impl<'f> InteriorForge<'f> {
    /// Looks the class Records up in `forge`.
    #[must_use]
    pub const fn new(forge: &'f DataForge<'f>) -> Self {
        Self {
            forge,
            palettes: ForgePalettes::new(forge),
        }
    }
}

impl ClassLookup for InteriorForge<'_> {
    fn geometry(&self, guid: Guid) -> Option<Geometry> {
        let record = self.forge.record_by_guid(guid)?;
        if !is_entity(&record) {
            return None;
        }
        read_geometry(&record.tree().ok()?)
    }

    fn palettes(&self) -> &dyn PaletteProvider {
        &self.palettes
    }
}
