// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Guid, StructValue, Value, ValueError};

use crate::read::{struct_field, struct_items, text};

/// The deepest nesting of sub-geometry nodes that is read.
const MAX_NESTED_SUB_GEOMETRY: usize = 4;

/// The tint palette an Entity's geometry names (`Palette.TintPaletteRef`): the `TintPaletteTree`
/// Record and the child of its tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteRef {
    pub(crate) guid: Guid,
    pub(crate) child_path: String,
}

impl PaletteRef {
    /// Returns the GUID of the `TintPaletteTree` Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the path of the child palette below the Record's root, or an empty string for the
    /// root.
    #[must_use]
    pub fn child_path(&self) -> &str {
        &self.child_path
    }
}

/// The geometry an Entity Record names (`SGeometryResourceParams`): the model file, the material
/// library that replaces the library the model names itself and the tint palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    pub(crate) path: String,
    pub(crate) material: Option<String>,
    pub(crate) palette: Option<PaletteRef>,
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

    /// Returns the tint palette the Entity chooses, or `None` when the Record names none.
    #[must_use]
    pub const fn palette(&self) -> Option<&PaletteRef> {
        self.palette.as_ref()
    }

    /// Reads the main geometry of a `SGeometryResourceParams` component.
    pub(crate) fn read(component: &StructValue<'_>) -> Option<Self> {
        Self::of_node(struct_field(component, "Geometry")?)
    }

    /// Reads the geometry of the sub-nodes of the main node of a `SGeometryResourceParams`
    /// component that name a model, with their tags: the children of the main node first, then
    /// the sub-nodes nested in them, which carry the tags of the nodes they are nested in as well
    /// as their own.
    pub(crate) fn read_variants(component: &StructValue<'_>) -> Vec<GeometryVariant> {
        let Some(node) = struct_field(component, "Geometry") else {
            return Vec::new();
        };
        let mut variants = Vec::new();
        let mut level = vec![(node, Vec::<String>::new())];
        for _ in 0..MAX_NESTED_SUB_GEOMETRY {
            let mut next = Vec::new();
            for (parent, inherited) in &level {
                for sub in struct_items(parent, "SubGeometry") {
                    let mut tags = inherited.clone();
                    tags.extend(text(sub, "Tags").split_whitespace().map(str::to_owned));
                    if let Some(geometry) = Self::of_node(sub) {
                        variants.push(GeometryVariant {
                            tags: tags.clone(),
                            geometry,
                        });
                    }
                    next.push((sub, tags));
                }
            }
            if next.is_empty() {
                break;
            }
            level = next;
        }
        variants
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
            palette: read_palette(data),
        })
    }
}

fn read_palette(data: &StructValue<'_>) -> Option<PaletteRef> {
    let reference = struct_field(data, "Palette")?;
    let guid = match reference.field("RootRecord")? {
        Value::Reference(link) => link.guid(),
        Value::Error(ValueError::DanglingReference { guid }) => *guid,
        _ => return None,
    };
    Some(PaletteRef {
        guid,
        child_path: text(reference, "ChildPath").trim().to_owned(),
    })
}
