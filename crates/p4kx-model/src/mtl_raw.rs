// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::mtl_xml::XmlElement;
use crate::{LayerSurface, MaterialError, PaletteSlot, TintEntry};

/// The deepest nesting of elements the reader keeps in a [`RawElement`].
pub const MAX_RAW_DEPTH: usize = 8;

/// The most elements the reader keeps in the raw elements of one material.
pub const MAX_RAW_ELEMENTS: usize = 4096;

/// The most material layers one material may hold.
pub const MAX_LAYERS: usize = 64;

/// An attribute list as the file gives it: names and values in file order.
pub type Attributes = Vec<(String, String)>;

/// An XML element of a material as the file has it, for parts of the format the reader does not
/// interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawElement {
    /// The element's name.
    pub name: String,
    /// The element's attributes in file order.
    pub attributes: Attributes,
    /// The element's child elements in file order.
    pub children: Vec<Self>,
}

/// One layer of a material's `MatLayers` list: another material blended into the surface.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MaterialLayer {
    /// All attributes of the `Layer` element in file order, e.g. `Name`, `Path`, `TintColor`,
    /// `PaletteTint`, `UVTiling`, `GlossMult`.
    pub attributes: Attributes,
    /// The surface values of the layer's own `.mtl` file; `None` until
    /// [`MaterialStatus::load`](crate::MaterialStatus::load) has read it, and when it is missing.
    pub surface: Option<LayerSurface>,
    /// The palette entry a [`TintPalette`](crate::TintPalette) gave the layer, for a layer with a
    /// `PaletteTint` from 1 to 3.
    pub palette_entry: Option<TintEntry>,
}

impl MaterialLayer {
    /// Returns the value of the attribute `name`, ignoring case.
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Returns the palette entry the layer takes: that of its `PaletteTint` from 1 to 3.
    #[must_use]
    pub fn palette_slot(&self) -> Option<PaletteSlot> {
        let number = self.attribute("PaletteTint")?.trim().parse::<u32>().ok()?;
        PaletteSlot::of_layer(number)
    }
}

pub(crate) struct Budget(usize);

impl Budget {
    pub(crate) const fn new() -> Self {
        Self(MAX_RAW_ELEMENTS)
    }

    fn take(&mut self) -> Result<(), MaterialError> {
        self.0 = self.0.checked_sub(1).ok_or(MaterialError::TooMany {
            what: "raw elements in one material",
            limit: MAX_RAW_ELEMENTS,
        })?;
        Ok(())
    }
}

pub(crate) fn attributes<'a>(element: impl XmlElement<'a>) -> Attributes {
    element
        .attributes()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

pub(crate) fn raw<'a>(
    element: impl XmlElement<'a>,
    depth: usize,
    budget: &mut Budget,
) -> Result<RawElement, MaterialError> {
    budget.take()?;
    let children = if depth < MAX_RAW_DEPTH {
        element
            .children()
            .map(|child| raw(child, depth + 1, budget))
            .collect::<Result<_, _>>()?
    } else {
        Vec::new()
    };
    Ok(RawElement {
        name: element.name().to_owned(),
        attributes: attributes(element),
        children,
    })
}

pub(crate) fn layers<'a>(
    element: impl XmlElement<'a>,
    budget: &mut Budget,
) -> Result<Vec<MaterialLayer>, MaterialError> {
    let mut layers = Vec::new();
    let lists = element
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case("MatLayers"));
    for layer in lists
        .flat_map(XmlElement::children)
        .filter(|child| child.name().eq_ignore_ascii_case("Layer"))
    {
        if layers.len() == MAX_LAYERS {
            return Err(MaterialError::TooMany {
                what: "layers in one material",
                limit: MAX_LAYERS,
            });
        }
        budget.take()?;
        layers.push(MaterialLayer {
            attributes: attributes(layer),
            surface: None,
            palette_entry: None,
        });
    }
    Ok(layers)
}

pub(crate) fn public_params<'a>(element: impl XmlElement<'a>) -> Attributes {
    element
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case("PublicParams"))
        .flat_map(|params| attributes(params))
        .collect()
}

pub(crate) fn other_elements<'a>(
    element: impl XmlElement<'a>,
    budget: &mut Budget,
) -> Result<Vec<RawElement>, MaterialError> {
    const INTERPRETED: [&str; 4] = ["SubMaterials", "Textures", "MatLayers", "PublicParams"];
    element
        .children()
        .filter(|child| {
            !INTERPRETED
                .iter()
                .any(|known| child.name().eq_ignore_ascii_case(known))
        })
        .map(|child| raw(child, 1, budget))
        .collect()
}
