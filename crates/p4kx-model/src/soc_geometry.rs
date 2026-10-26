// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::mtl_xml::XmlElement;
use crate::soc_entities::{SocPalette, child};

/// The geometry an entity names itself: `PropertiesDataCore/EntityGeometryResource` and the three
/// nested `Geometry` elements.
pub(crate) struct EntityGeometry<'a> {
    pub(crate) path: &'a str,
    pub(crate) material: Option<&'a str>,
    pub(crate) palette: Option<SocPalette>,
}

pub(crate) fn read_geometry<'a, E: XmlElement<'a>>(entity: E) -> Option<EntityGeometry<'a>> {
    let mut node = child(entity, "PropertiesDataCore")?;
    node = child(node, "EntityGeometryResource")?;
    node = child(node, "Geometry")?;
    let data = child(node, "Geometry")?;
    let path = child(data, "Geometry")?
        .attribute("path")
        .filter(|path| !path.is_empty())?;
    let material = child(data, "Material")
        .and_then(|material| material.attribute("path"))
        .filter(|path| !path.is_empty());
    let palette = child(data, "Palette").and_then(|reference| {
        let root = reference.attribute("RootRecord")?.trim();
        let unset = root.chars().all(|c| c == '0' || c == '-');
        (!unset).then(|| SocPalette {
            root: root.to_owned(),
            child: reference
                .attribute("ChildPath")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        })
    });
    Some(EntityGeometry {
        path,
        material,
        palette,
    })
}
