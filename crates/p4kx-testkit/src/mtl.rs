// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::cryxml::{CryXmlByteOrder, CryXmlSignature, TestElement};

/// One material of a synthetic `.mtl` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMtlMaterial {
    attributes: Vec<(String, String)>,
    textures: Vec<(String, String)>,
    layers: Vec<(String, String)>,
    public_params: Vec<(String, String)>,
}

impl TestMtlMaterial {
    /// Creates a material named `name` with the shader `Illum` and no other attribute.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            attributes: vec![
                ("Name".to_owned(), name.into()),
                ("Shader".to_owned(), "Illum".to_owned()),
            ],
            textures: Vec::new(),
            layers: Vec::new(),
            public_params: Vec::new(),
        }
    }

    /// Sets the shader parameter `name` of the material's `PublicParams` to `value`, as written:
    /// a number or colour such as `0.5` or `1,0.5,0.25`.
    #[must_use]
    pub fn public_param(mut self, name: &str, value: &str) -> Self {
        self.public_params.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Adds a material layer named `name` whose `PaletteTint` attribute is `palette_tint`, as
    /// written (`1` to `3` take a palette entry, `0` none).
    #[must_use]
    pub fn layer(mut self, name: &str, palette_tint: &str) -> Self {
        self.layers.push((name.to_owned(), palette_tint.to_owned()));
        self
    }

    /// Sets an attribute to any value, even a malformed one.
    #[must_use]
    pub fn attribute(mut self, name: &str, value: impl Into<String>) -> Self {
        let value = value.into();
        match self.attributes.iter_mut().find(|(known, _)| known == name) {
            Some((_, current)) => *current = value,
            None => self.attributes.push((name.to_owned(), value)),
        }
        self
    }

    /// Sets the `Diffuse` colour.
    #[must_use]
    pub fn diffuse(self, colour: [f32; 3]) -> Self {
        self.attribute("Diffuse", vector(colour))
    }

    /// Sets the `Specular` colour.
    #[must_use]
    pub fn specular(self, colour: [f32; 3]) -> Self {
        self.attribute("Specular", vector(colour))
    }

    /// Sets the `Emissive` colour.
    #[must_use]
    pub fn emissive(self, colour: [f32; 3]) -> Self {
        self.attribute("Emissive", vector(colour))
    }

    /// Sets `Glow`.
    #[must_use]
    pub fn glow(self, glow: f32) -> Self {
        self.attribute("Glow", glow.to_string())
    }

    /// Sets `Opacity`.
    #[must_use]
    pub fn opacity(self, opacity: f32) -> Self {
        self.attribute("Opacity", opacity.to_string())
    }

    /// Sets `Shininess`.
    #[must_use]
    pub fn shininess(self, shininess: f32) -> Self {
        self.attribute("Shininess", shininess.to_string())
    }

    /// Sets `AlphaTest`.
    #[must_use]
    pub fn alpha_test(self, threshold: f32) -> Self {
        self.attribute("AlphaTest", threshold.to_string())
    }

    /// Sets `MtlFlags`, as the signed 32-bit number the game's files write.
    #[must_use]
    pub fn flags(self, flags: u32) -> Self {
        self.attribute(
            "MtlFlags",
            i32::from_ne_bytes(flags.to_ne_bytes()).to_string(),
        )
    }

    /// Sets `SurfaceType`.
    #[must_use]
    pub fn surface_type(self, surface: &str) -> Self {
        self.attribute("SurfaceType", surface)
    }

    /// Sets `StringGenMask`.
    #[must_use]
    pub fn string_gen_mask(self, mask: &str) -> Self {
        self.attribute("StringGenMask", mask)
    }

    /// Sets `Shader`.
    #[must_use]
    pub fn shader(self, shader: &str) -> Self {
        self.attribute("Shader", shader)
    }

    /// Adds a texture to the slot `map`, e.g. `TexSlot1`.
    #[must_use]
    pub fn texture(mut self, map: &str, file: &str) -> Self {
        self.textures.push((map.to_owned(), file.to_owned()));
        self
    }

    fn to_element(&self) -> TestElement {
        let mut element = TestElement::new("Material");
        for (name, value) in &self.attributes {
            element = element.attribute(name, value);
        }
        let mut textures = TestElement::new("Textures");
        for (map, file) in &self.textures {
            textures = textures.child(
                TestElement::new("Texture")
                    .attribute("Map", map)
                    .attribute("File", file),
            );
        }
        let mut element = element.child(textures);
        if !self.public_params.is_empty() {
            let mut params = TestElement::new("PublicParams");
            for (name, value) in &self.public_params {
                params = params.attribute(name, value);
            }
            element = element.child(params);
        }
        if self.layers.is_empty() {
            return element;
        }
        let mut layers = TestElement::new("MatLayers");
        for (name, palette_tint) in &self.layers {
            layers = layers.child(
                TestElement::new("Layer")
                    .attribute("Name", name)
                    .attribute("PaletteTint", palette_tint),
            );
        }
        element.child(layers)
    }
}

fn vector([x, y, z]: [f32; 3]) -> String {
    format!("{x},{y},{z}")
}

/// A synthetic `.mtl` file: one material for every material ID, or a list of sub-materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMtl {
    root: TestElement,
}

impl TestMtl {
    /// Creates a file that is one material.
    #[must_use]
    pub fn single(material: &TestMtlMaterial) -> Self {
        Self {
            root: material.to_element(),
        }
    }

    /// Creates a file of sub-materials; the material ID is the position in `materials`.
    #[must_use]
    pub fn multi(materials: &[TestMtlMaterial]) -> Self {
        let mut list = TestElement::new("SubMaterials");
        for material in materials {
            list = list.child(material.to_element());
        }
        Self {
            root: TestElement::new("Material")
                .attribute("MtlFlags", "256")
                .child(list),
        }
    }

    /// Names the tint palette the file uses by default.
    #[must_use]
    pub fn with_default_palette(mut self, palette: &str) -> Self {
        self.root = self.root.attribute("DefaultPalette", palette);
        self
    }

    /// Returns the file as text XML.
    #[must_use]
    pub fn text_xml(&self) -> Vec<u8> {
        self.root.to_xml().into_bytes()
    }

    /// Returns the file as little-endian `CryXmlB`, the game's form.
    #[must_use]
    pub fn cryxml(&self) -> Vec<u8> {
        self.cryxml_as(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    }

    /// Returns the file as `CryXml` with the given signature and byte order.
    #[must_use]
    pub fn cryxml_as(&self, signature: CryXmlSignature, byte_order: CryXmlByteOrder) -> Vec<u8> {
        self.root.to_cryxml(signature, byte_order)
    }
}
