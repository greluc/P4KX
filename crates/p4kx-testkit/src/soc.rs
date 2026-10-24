// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::{put_u16, put_u32};
use crate::{CryXmlByteOrder, CryXmlSignature, TestChunkFile, TestElement, TestSocEntity};

const ENTITIES: u32 = 0x0004;
const NAME_LEN: usize = 256;
const OBJECT_LEN: usize = 208;
const MODEL_INDEX_AT: usize = 60;
const TRANSFORM_AT: usize = 64;
const PALETTE_AT: usize = 172;
const INCLUDED_OBJECTS: u32 = 0x0010;
const SOC_HEADER: u32 = 0x0002;

/// The identity as the rows of a 3×4 matrix.
pub const SOC_IDENTITY: [[f64; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

#[derive(Debug, Clone)]
struct TestSocObject {
    kind: u32,
    model: u16,
    rows: [[f64; 4]; 3],
    palette: u8,
}

/// A synthetic `.soc` file: a `CrCh` chunk file with an `IncludedObjects` chunk in the layout
/// of real socpaks (a table of 256-byte model paths, one 208-byte record per placed model).
#[derive(Debug, Clone, Default)]
pub struct TestSoc {
    models: Vec<String>,
    palettes: Vec<String>,
    objects: Vec<TestSocObject>,
    entities: Vec<TestSocEntity>,
}

impl TestSoc {
    /// Adds `entity` to the entity chunk (`CryXmlB`, type `0x0004`).
    #[must_use]
    pub fn entity(mut self, entity: TestSocEntity) -> Self {
        self.entities.push(entity);
        self
    }

    /// Returns the bytes of the entity chunk, a `CryXmlB` document.
    #[must_use]
    pub fn entities_chunk(&self) -> Vec<u8> {
        self.entities
            .iter()
            .fold(TestElement::new("Entities"), |root, entity| {
                root.child(entity.element())
            })
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    }

    /// Creates a `.soc` file that places nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `path` to the model table; the models are numbered from 0 in the order added.
    #[must_use]
    pub fn model(mut self, path: impl Into<String>) -> Self {
        self.models.push(path.into());
        self
    }

    /// Adds `path` to the tint palette table.
    #[must_use]
    pub fn palette(mut self, path: impl Into<String>) -> Self {
        self.palettes.push(path.into());
        self
    }

    /// Places the model number `model` with the 3×4 row-major matrix `rows`.
    #[must_use]
    pub fn object(self, model: u16, rows: [[f64; 4]; 3]) -> Self {
        self.object_with(model, rows, 0)
    }

    /// Places the model number `model` with `rows`, tinted by the palette number `palette`.
    #[must_use]
    pub fn object_with(mut self, model: u16, rows: [[f64; 4]; 3], palette: u8) -> Self {
        self.objects.push(TestSocObject {
            kind: 1,
            model,
            rows,
            palette,
        });
        self
    }

    /// Adds a record of a kind other than a placed model, which ends the object list.
    #[must_use]
    pub fn object_of_kind(mut self, kind: u32) -> Self {
        self.objects.push(TestSocObject {
            kind,
            model: 0,
            rows: SOC_IDENTITY,
            palette: 0,
        });
        self
    }

    /// Returns the bytes of the `IncludedObjects` chunk.
    #[must_use]
    pub fn chunk(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u32(&mut out, 0);
        put_u32(
            &mut out,
            u32::try_from(self.models.len()).unwrap_or(u32::MAX),
        );
        for model in &self.models {
            put_name(&mut out, model);
        }
        put_u16(&mut out, 0);
        put_u16(
            &mut out,
            u16::try_from(self.palettes.len()).unwrap_or(u16::MAX),
        );
        for palette in &self.palettes {
            put_name(&mut out, palette);
        }
        put_u32(
            &mut out,
            u32::try_from(self.objects.len()).unwrap_or(u32::MAX),
        );
        for bound in [f32::MIN, f32::MIN, f32::MIN, f32::MAX, f32::MAX, f32::MAX] {
            out.extend_from_slice(&bound.to_le_bytes());
        }
        put_u32(
            &mut out,
            u32::try_from(self.objects.len() * OBJECT_LEN).unwrap_or(u32::MAX),
        );
        for object in &self.objects {
            out.extend_from_slice(&record(object));
        }
        out
    }

    /// Returns the bytes of the file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let file = TestChunkFile::crch()
            .chunk(SOC_HEADER, 5, vec![0; 4])
            .chunk(INCLUDED_OBJECTS, 1, self.chunk());
        if self.entities.is_empty() {
            file.bytes()
        } else {
            file.chunk(ENTITIES, 3, self.entities_chunk()).bytes()
        }
    }
}

/// A synthetic object container description, the `<name>.xml` file of a socpak.
#[derive(Debug, Clone, Default)]
pub struct TestObjectContainer {
    name: String,
    children: Vec<TestElement>,
    streams: Vec<String>,
}

impl TestObjectContainer {
    /// Creates a description of the container `name` without nested containers or `.soc` files.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Nests the container `name` at `position` turned by the quaternion `rotation`
    /// (`[w, x, y, z]`).
    #[must_use]
    pub fn child(mut self, name: &str, position: [f64; 3], rotation: [f64; 4]) -> Self {
        let join = |values: &[f64]| {
            values
                .iter()
                .map(f64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        self.children.push(
            TestElement::new("Child")
                .attribute("name", name)
                .attribute("class", "ObjectContainerInstance")
                .attribute("entityName", name)
                .attribute("pos", join(&position))
                .attribute("rot", join(&rotation))
                .attribute("external", "1"),
        );
        self
    }

    /// Nests a container with the attributes `attributes` as they are, for malformed ones.
    #[must_use]
    pub fn raw_child(mut self, attributes: &[(&str, &str)]) -> Self {
        self.children.push(
            attributes
                .iter()
                .fold(TestElement::new("Child"), |element, (name, value)| {
                    element.attribute(*name, *value)
                }),
        );
        self
    }

    /// Streams the `.soc` file `name`.
    #[must_use]
    pub fn stream(mut self, name: impl Into<String>) -> Self {
        self.streams.push(name.into());
        self
    }

    fn element(&self) -> TestElement {
        let mut root = TestElement::new("ObjectContainer").attribute("name", &self.name);
        if !self.children.is_empty() {
            root = root.child(self.children.iter().cloned().fold(
                TestElement::new("ChildObjectContainers"),
                TestElement::child,
            ));
        }
        if !self.streams.is_empty() {
            root = root.child(
                self.streams
                    .iter()
                    .fold(TestElement::new("StreamingOCs"), |streams, name| {
                        streams.child(TestElement::new("OC").attribute("name", name))
                    }),
            );
        }
        root
    }

    /// Returns the description as text XML.
    #[must_use]
    pub fn xml(&self) -> String {
        self.element().to_xml()
    }

    /// Returns the description as little-endian `CryXmlB`.
    #[must_use]
    pub fn cryxml(&self) -> Vec<u8> {
        self.element()
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    }
}

fn put_name(out: &mut Vec<u8>, name: &str) {
    let mut field = [0_u8; NAME_LEN];
    let bytes = name.as_bytes();
    let len = bytes.len().min(NAME_LEN - 1);
    if let (Some(target), Some(source)) = (field.get_mut(..len), bytes.get(..len)) {
        target.copy_from_slice(source);
    }
    out.extend_from_slice(&field);
}

fn record(object: &TestSocObject) -> [u8; OBJECT_LEN] {
    let mut record = [0_u8; OBJECT_LEN];
    let mut set = |at: usize, bytes: &[u8]| {
        if let Some(target) = record.get_mut(at..at + bytes.len()) {
            target.copy_from_slice(bytes);
        }
    };
    set(0, &object.kind.to_le_bytes());
    set(MODEL_INDEX_AT, &object.model.to_le_bytes());
    for (position, value) in object.rows.iter().flatten().enumerate() {
        set(TRANSFORM_AT + position * 8, &value.to_le_bytes());
    }
    set(PALETTE_AT, &[object.palette]);
    record
}
