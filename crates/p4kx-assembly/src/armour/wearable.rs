// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::Guid;

use super::{BodyVariant, ZoneChunk, ZoneList};
use crate::spec::EntitySpec;
use crate::{Geometry, GeometryVariant, MaterialVariant};

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
    pub(crate) tags: Vec<String>,
    pub(crate) material_variants: Vec<MaterialVariant>,
    pub(crate) zones: ZoneList,
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
            tags: spec.attach_tags.clone(),
            material_variants: spec.material_variants.clone(),
            zones: spec.zones.clone(),
        }
    }

    /// Returns the zones of the body the Entity covers when it is worn, from the Record's
    /// `SCItemClothingParams`.
    #[must_use]
    pub fn zones(&self) -> &[ZoneChunk] {
        &self.zones.chunks
    }

    /// Returns the tags of the Entity's `AttachDef`, e.g. `hairVar_Brown` for a hair colour.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns the material libraries the Record chooses by tags.
    #[must_use]
    pub fn material_variants(&self) -> &[MaterialVariant] {
        &self.material_variants
    }

    /// Returns the geometry worn on `body` like [`Self::geometry_for`], but the sub-geometry
    /// tagged with one of `selecting` comes first: a port's flags select the sub-geometry of the
    /// item attached to it.
    pub(crate) fn geometry_selected(
        &self,
        body: BodyVariant,
        selecting: &[String],
    ) -> Result<&Geometry, Unfit> {
        let chosen = self
            .variants
            .iter()
            .find(|found| selecting.iter().any(|tag| found.has_tag(tag)));
        chosen.map_or_else(|| self.geometry_for(body), |found| Ok(&found.geometry))
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

    /// Returns whether `geometry` is the main geometry of the Record and not a sub-geometry.
    pub(crate) fn is_main(&self, geometry: &Geometry) -> bool {
        self.geometry
            .as_ref()
            .is_some_and(|main| std::ptr::eq(main, geometry))
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
