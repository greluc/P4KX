// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use super::BodyVariant;
use crate::spec::EntitySpec;
use crate::{Geometry, GeometryVariant};

/// Why a piece has no geometry for a body variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unfit {
    /// The Entity's Record names no geometry at all.
    NoGeometry,
    /// The Record names geometry per body, but none for this variant.
    NoVariant,
}

/// An Entity worn on a body, with the geometry its Record names: the main geometry and the
/// sub-geometry per body tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wearable {
    pub(crate) guid: Guid,
    pub(crate) record_path: String,
    pub(crate) class_name: String,
    pub(crate) item_type: String,
    pub(crate) geometry: Option<Geometry>,
    pub(crate) variants: Vec<GeometryVariant>,
}

impl Wearable {
    pub(crate) fn from_spec(spec: &EntitySpec) -> Self {
        Self {
            guid: spec.guid,
            record_path: spec.record_path.clone(),
            class_name: crate::entities::class_name_of(&spec.name)
                .unwrap_or(&spec.name)
                .to_owned(),
            item_type: spec.attach_type.clone(),
            geometry: spec.geometry.clone(),
            variants: spec.variants.clone(),
        }
    }

    /// Returns the GUID of the Entity's Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the record path of the Entity.
    #[must_use]
    pub fn record_path(&self) -> &str {
        &self.record_path
    }

    /// Returns the class name of the Entity.
    #[must_use]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// Returns the Entity's `AttachDef.Type`, e.g. `Char_Armor_Torso`; empty when it has none.
    #[must_use]
    pub fn item_type(&self) -> &str {
        &self.item_type
    }

    /// Returns the main geometry of the Record, which is not the one worn on a body.
    #[must_use]
    pub const fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref()
    }

    /// Returns the sub-geometry of the Record with its tags.
    #[must_use]
    pub fn variants(&self) -> &[GeometryVariant] {
        &self.variants
    }

    /// Returns the geometry worn on `body`: the sub-geometry tagged with the variant's tag, else
    /// the main geometry when the Record names no geometry per body at all.
    pub(crate) fn geometry_for(&self, body: BodyVariant) -> Result<&Geometry, Unfit> {
        if let Some(found) = self.variants.iter().find(|found| found.has_tag(body.tag())) {
            return Ok(&found.geometry);
        }
        let per_body = self.variants.iter().any(|found| {
            BodyVariant::ALL
                .iter()
                .any(|other| found.has_tag(other.tag()))
        });
        if per_body {
            return Err(Unfit::NoVariant);
        }
        self.geometry.as_ref().ok_or(Unfit::NoGeometry)
    }
}
