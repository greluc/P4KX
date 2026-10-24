// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use crate::read::{struct_field, struct_items, text};

/// The geometry an Entity Record names (`SGeometryResourceParams`): the model file and the
/// material library that replaces the library the model names itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub(crate) path: String,
    pub(crate) material: Option<String>,
}

/// A geometry an Entity Record names for the tags of a sub-node (`SubGeometry`), e.g. `Male` or
/// `Female` for the body a piece of armour is worn on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryVariant {
    pub(crate) tags: Vec<String>,
    pub(crate) geometry: Geometry,
}

impl GeometryVariant {
    /// Returns the tags of the sub-node, split at white space.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns the geometry of the sub-node.
    #[must_use]
    pub const fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// Returns whether the sub-node carries the tag `tag`, ignoring ASCII case.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|own| own.eq_ignore_ascii_case(tag))
    }
}

impl Geometry {
    /// Returns the model file as the Record spells it, a path below `Data`, e.g.
    /// `Objects\Spaceships\Ships\AEGS\Gladius\AEGS_Gladius.cga`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the material library the Entity chooses, a path below `Data`, or `None` when the
    /// Record names none and the model's own library applies.
    #[must_use]
    pub fn material(&self) -> Option<&str> {
        self.material.as_deref()
    }

    /// Reads the main geometry of a `SGeometryResourceParams` component.
    pub(crate) fn read(component: &StructValue<'_>) -> Option<Self> {
        Self::of_node(struct_field(component, "Geometry")?)
    }

    /// Reads the geometry of the sub-nodes of the main node of a `SGeometryResourceParams`
    /// component that name a model, with their tags.
    pub(crate) fn read_variants(component: &StructValue<'_>) -> Vec<GeometryVariant> {
        let Some(node) = struct_field(component, "Geometry") else {
            return Vec::new();
        };
        struct_items(node, "SubGeometry")
            .filter_map(|sub| {
                Some(GeometryVariant {
                    tags: text(sub, "Tags")
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect(),
                    geometry: Self::of_node(sub)?,
                })
            })
            .collect()
    }

    fn of_node(node: &StructValue<'_>) -> Option<Self> {
        let data = struct_field(node, "Geometry")?;
        let path = struct_field(data, "Geometry").map(|resource| text(resource, "path"))?;
        if path.is_empty() {
            return None;
        }
        let material = struct_field(data, "Material")
            .map(|resource| text(resource, "path"))
            .filter(|path| !path.is_empty());
        Some(Self {
            path: path.to_owned(),
            material: material.map(str::to_owned),
        })
    }
}
