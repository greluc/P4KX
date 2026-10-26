// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};

fn join(values: &[f64]) -> String {
    values
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// An object of a synthetic prefab.
#[derive(Debug, Clone)]
pub struct TestPrefabObject {
    kind: &'static str,
    name: String,
    position: Option<String>,
    rotation: Option<String>,
    scale: Option<String>,
    attributes: Vec<(String, String)>,
    children: Vec<TestElement>,
    members: Vec<Self>,
}

impl TestPrefabObject {
    fn placed(kind: &'static str, name: &str, position: [f64; 3], rotation: [f64; 4]) -> Self {
        Self {
            kind,
            name: name.to_owned(),
            position: Some(join(&position)),
            rotation: Some(join(&rotation)),
            scale: None,
            attributes: Vec::new(),
            children: Vec::new(),
            members: Vec::new(),
        }
    }

    fn with(mut self, name: &str, value: impl Into<String>) -> Self {
        self.attributes.retain(|(existing, _)| existing != name);
        self.attributes.push((name.to_owned(), value.into()));
        self
    }

    /// Creates a brush of the model `model` at `position` turned by the quaternion `rotation`
    /// (`[w, x, y, z]`).
    #[must_use]
    pub fn brush(name: &str, model: &str, position: [f64; 3], rotation: [f64; 4]) -> Self {
        Self::placed("Brush", name, position, rotation).with("Prefab", model)
    }

    /// Creates a geometry entity of the model `model`.
    #[must_use]
    pub fn geometry_entity(
        name: &str,
        model: &str,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Self {
        Self::placed("GeomEntity", name, position, rotation)
            .with("EntityClass", "GeomEntity")
            .with("Geometry", model)
    }

    /// Creates an entity of the class `class` whose geometry resource names `model`.
    #[must_use]
    pub fn entity(
        name: &str,
        class: &str,
        model: &str,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Self {
        let mut object =
            Self::placed("Entity", name, position, rotation).with("EntityClass", class);
        let innermost = TestElement::new("Geometry").attribute("path", model);
        let resource = TestElement::new("EntityGeometryResource").child(
            TestElement::new("Geometry").child(TestElement::new("Geometry").child(innermost)),
        );
        object
            .children
            .push(TestElement::new("PropertiesDataCore").child(resource));
        object
    }

    /// Creates an object of the type `kind`, e.g. a `TagPoint`, that names no geometry.
    #[must_use]
    pub fn marker(kind: &'static str, name: &str, position: [f64; 3]) -> Self {
        Self::placed(kind, name, position, [1.0, 0.0, 0.0, 0.0])
    }

    /// Creates an object that instances the prefab `prefab` of the library `library`.
    #[must_use]
    pub fn nested(
        name: &str,
        library: &str,
        prefab: &str,
        position: [f64; 3],
        rotation: [f64; 4],
    ) -> Self {
        Self::placed("Prefab", name, position, rotation)
            .with("PrefabGUID", prefab_guid(library, prefab))
            .with("PrefabName", format!("{library}.{prefab}"))
    }

    /// Creates a group at `position` turned by `rotation` that holds `members`, whose
    /// transforms are relative to it.
    #[must_use]
    pub fn group(name: &str, position: [f64; 3], rotation: [f64; 4], members: Vec<Self>) -> Self {
        let mut object = Self::placed("Group", name, position, rotation);
        object.members = members;
        object
    }

    /// Sets the `Scale` of the object.
    #[must_use]
    pub fn scaled(mut self, scale: [f64; 3]) -> Self {
        self.scale = Some(join(&scale));
        self
    }

    /// Sets the `Rotate` attribute to `text`, which need not be a quaternion.
    #[must_use]
    pub fn rotate_text(mut self, text: &str) -> Self {
        self.rotation = Some(text.to_owned());
        self
    }

    /// Leaves out the `Pos` and `Rotate` attributes.
    #[must_use]
    pub fn without_transform(mut self) -> Self {
        self.position = None;
        self.rotation = None;
        self
    }

    /// Adds the attribute `name` with `value`.
    #[must_use]
    pub fn attribute(self, name: &str, value: &str) -> Self {
        self.with(name, value)
    }

    fn id(&self) -> String {
        format!("{{{}}}", self.name.to_ascii_uppercase())
    }

    fn element(&self, parent: Option<&str>) -> TestElement {
        let mut element = TestElement::new("Object")
            .attribute("Type", self.kind)
            .attribute("Name", &self.name)
            .attribute("Id", self.id());
        if let Some(parent) = parent {
            element = element.attribute("Parent", parent);
        }
        for (value, name) in [
            (&self.position, "Pos"),
            (&self.rotation, "Rotate"),
            (&self.scale, "Scale"),
        ] {
            if let Some(value) = value {
                element = element.attribute(name, value);
            }
        }
        for (name, value) in &self.attributes {
            element = element.attribute(name, value);
        }
        element = self
            .children
            .iter()
            .cloned()
            .fold(element, TestElement::child);
        if self.members.is_empty() {
            return element;
        }
        let id = self.id();
        element.child(
            self.members
                .iter()
                .fold(TestElement::new("Objects"), |all, member| {
                    all.child(member.element(Some(&id)))
                }),
        )
    }
}

/// The `PrefabGUID` that [`TestPrefabObject::nested`] names for a prefab.
#[must_use]
pub fn prefab_guid(library: &str, prefab: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in format!(
        "{}.{}",
        library.to_ascii_lowercase(),
        prefab.to_ascii_lowercase()
    )
    .bytes()
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{{{:08X}-0000-0000-0000-000000000000}}", hash & 0xffff_ffff)
}

/// A synthetic prefab library: a `PrefabsLibrary` XML file.
#[derive(Debug, Clone)]
pub struct TestPrefabLibrary {
    name: String,
    prefabs: Vec<(String, Vec<TestPrefabObject>)>,
}

impl TestPrefabLibrary {
    /// Creates a library named `name` without prefabs.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prefabs: Vec::new(),
        }
    }

    /// Adds a prefab with `objects`.
    #[must_use]
    pub fn prefab(mut self, name: impl Into<String>, objects: Vec<TestPrefabObject>) -> Self {
        self.prefabs.push((name.into(), objects));
        self
    }

    fn element(&self) -> TestElement {
        self.prefabs.iter().fold(
            TestElement::new("PrefabsLibrary").attribute("Name", &self.name),
            |library, (name, objects)| {
                let objects = objects
                    .iter()
                    .fold(TestElement::new("Objects"), |all, object| {
                        all.child(object.element(None))
                    });
                library.child(
                    TestElement::new("Prefab")
                        .attribute("Name", name)
                        .attribute("Library", &self.name)
                        .attribute("Id", prefab_guid(&self.name, name))
                        .child(objects),
                )
            },
        )
    }

    /// Returns the library as text XML.
    #[must_use]
    pub fn xml(&self) -> String {
        self.element().to_xml()
    }

    /// Returns the library as `CryXmlB`.
    #[must_use]
    pub fn cryxml(&self) -> Vec<u8> {
        self.element()
            .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
    }

    /// Returns the Entry of the library below `Data\Prefabs`, stored as `CryXml`.
    #[must_use]
    pub fn entry(&self) -> TestEntry {
        TestEntry::stored(
            format!("Data\\Prefabs\\{}.xml", self.name.replace('/', "\\")),
            self.cryxml(),
        )
    }
}
