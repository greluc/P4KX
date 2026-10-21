use crate::{TestInstance, TestRecord, TestValue};

/// The prefix of the Record name of every Entity.
const ENTITY_PREFIX: &str = "EntityClassDefinition.";

/// A synthetic Entity Record with its item ports and default Loadout.
///
/// [`TestEntity::record`] turns it into a [`TestRecord`] of a database built with
/// [`entity_schema`](crate::entity_schema).
#[derive(Debug, Clone, PartialEq)]
pub struct TestEntity {
    guid: u128,
    class_name: String,
    file_name: String,
    ports: Vec<TestPort>,
    loadout: Option<TestLoadout>,
    triggers: Vec<String>,
    sound_effects: Vec<(String, String)>,
    geometry: Option<(String, Option<String>)>,
    attach_type: Option<String>,
}

impl TestEntity {
    /// Creates an Entity with the class name `class_name`, whose Record is named
    /// `EntityClassDefinition.<class_name>`, without ports and Loadout.
    #[must_use]
    pub fn new(guid: u128, class_name: impl Into<String>, file_name: impl Into<String>) -> Self {
        Self {
            guid,
            class_name: class_name.into(),
            file_name: file_name.into(),
            ports: Vec::new(),
            loadout: None,
            triggers: Vec::new(),
            sound_effects: Vec::new(),
            geometry: None,
            attach_type: None,
        }
    }

    /// Gives the Entity an attachable component whose `AttachDef.Type` is `attach_type`, e.g.
    /// `NOITEM_Vehicle`; the database needs [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn attach_type(mut self, attach_type: impl Into<String>) -> Self {
        self.attach_type = Some(attach_type.into());
        self
    }

    /// Names the model `path` as the Entity's geometry and, if given, the material library
    /// `material` that replaces the one the model names; the database needs
    /// [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn geometry(mut self, path: impl Into<String>, material: Option<&str>) -> Self {
        self.geometry = Some((path.into(), material.map(str::to_owned)));
        self
    }

    /// Adds a reference to the audio trigger `name` in the Entity's audio component; the
    /// database needs [`audio_schema`](crate::audio_schema).
    #[must_use]
    pub fn audio_trigger(mut self, name: impl Into<String>) -> Self {
        self.triggers.push(name.into());
        self
    }

    /// Adds a sound trigger effect `effect` playing the audio trigger `trigger` to the Entity's
    /// audio component; the database needs [`audio_schema`](crate::audio_schema).
    #[must_use]
    pub fn sound_effect(mut self, effect: impl Into<String>, trigger: impl Into<String>) -> Self {
        self.sound_effects.push((effect.into(), trigger.into()));
        self
    }

    /// Adds an item port definition.
    #[must_use]
    pub fn port(mut self, port: TestPort) -> Self {
        self.ports.push(port);
        self
    }

    /// Sets the default Loadout.
    #[must_use]
    pub fn loadout(mut self, loadout: TestLoadout) -> Self {
        self.loadout = Some(loadout);
        self
    }

    /// Returns the Record.
    #[must_use]
    pub fn record(&self) -> TestRecord {
        let mut components = Vec::new();
        if let Some(loadout) = &self.loadout {
            components.push(TestValue::Struct(
                TestInstance::new("SEntityComponentDefaultLoadoutParams")
                    .value("loadout", loadout.value()),
            ));
        }
        if !self.ports.is_empty() {
            components.push(TestValue::Struct(
                TestInstance::new("SItemPortContainerComponentParams").value(
                    "Ports",
                    TestValue::Array(self.ports.iter().map(TestPort::value).collect()),
                ),
            ));
        }
        if !self.triggers.is_empty() || !self.sound_effects.is_empty() {
            components.push(self.audio_component());
        }
        if let Some((path, material)) = &self.geometry {
            components.push(geometry_component(path, material.as_deref()));
        }
        if let Some(attach_type) = &self.attach_type {
            components.push(TestValue::Struct(
                TestInstance::new("SAttachableComponentParams").value(
                    "AttachDef",
                    TestValue::Struct(
                        TestInstance::new("SItemDefinition")
                            .value("Type", TestValue::text(attach_type.clone())),
                    ),
                ),
            ));
        }
        TestRecord::new(
            self.guid,
            "EntityClassDefinition",
            format!("{ENTITY_PREFIX}{}", self.class_name),
            self.file_name.clone(),
        )
        .value("Category", TestValue::text("Default"))
        .value("Components", TestValue::Array(components))
    }

    fn audio_component(&self) -> TestValue {
        let audio = |trigger: &str| {
            TestValue::Struct(
                TestInstance::new("GlobalResourceAudio")
                    .value("audioTrigger", TestValue::text(trigger)),
            )
        };
        let effects = self
            .sound_effects
            .iter()
            .map(|(effect, trigger)| {
                TestValue::Struct(
                    TestInstance::new("SEntityEffectSystem_SoundTriggerEffect")
                        .value("name", TestValue::text(effect.clone()))
                        .value("audio", audio(trigger)),
                )
            })
            .collect();
        TestValue::Struct(
            TestInstance::new("SAudioTestComponentParams")
                .value(
                    "triggers",
                    TestValue::Array(self.triggers.iter().map(|name| audio(name)).collect()),
                )
                .value("triggerEffects", TestValue::Array(effects)),
        )
    }
}

fn geometry_component(path: &str, material: Option<&str>) -> TestValue {
    let resource = |type_name: &str, path: &str| {
        TestValue::Struct(TestInstance::new(type_name).value("path", TestValue::text(path)))
    };
    let mut data = TestInstance::new("SGeometryDataParams")
        .value("Geometry", resource("GlobalResourceGeometry", path));
    data = data.value(
        "Material",
        resource("GlobalResourceMaterial", material.unwrap_or_default()),
    );
    let node = TestInstance::new("SGeometryNodeParams").value("Geometry", TestValue::Struct(data));
    TestValue::Struct(
        TestInstance::new("SGeometryResourceParams").value("Geometry", TestValue::Struct(node)),
    )
}

/// A Loadout of a synthetic Entity or Loadout entry.
#[derive(Debug, Clone, PartialEq)]
pub enum TestLoadout {
    /// A manual Loadout with its entries in order.
    Manual(Vec<TestLoadoutEntry>),
    /// An XML Loadout naming the file that holds it.
    Xml(String),
}

impl TestLoadout {
    fn value(&self) -> TestValue {
        TestValue::Struct(match self {
            Self::Manual(entries) => TestInstance::new("SItemPortLoadoutManualParams").value(
                "entries",
                TestValue::Array(entries.iter().map(TestLoadoutEntry::value).collect()),
            ),
            Self::Xml(path) => TestInstance::new("SItemPortLoadoutXMLParams")
                .value("loadoutPath", TestValue::text(path.clone())),
        })
    }
}

/// One entry of a manual Loadout: a port and what is attached to it.
#[derive(Debug, Clone, PartialEq)]
pub struct TestLoadoutEntry {
    port: String,
    class_name: String,
    reference: Option<u128>,
    loadout: Option<TestLoadout>,
}

impl TestLoadoutEntry {
    /// Creates an entry for the port `port` with nothing attached.
    #[must_use]
    pub fn empty(port: impl Into<String>) -> Self {
        Self {
            port: port.into(),
            class_name: String::new(),
            reference: None,
            loadout: None,
        }
    }

    /// Creates an entry attaching the Entity of class name `class_name`.
    #[must_use]
    pub fn class(port: impl Into<String>, class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            ..Self::empty(port)
        }
    }

    /// Creates an entry attaching the Entity by a Reference to `guid`.
    #[must_use]
    pub fn reference(port: impl Into<String>, guid: u128) -> Self {
        Self {
            reference: Some(guid),
            ..Self::empty(port)
        }
    }

    /// Sets the Loadout the entry gives the attached Entity.
    #[must_use]
    pub fn loadout(mut self, loadout: TestLoadout) -> Self {
        self.loadout = Some(loadout);
        self
    }

    fn value(&self) -> TestValue {
        let mut instance = TestInstance::new("SItemPortLoadoutEntryParams")
            .value("itemPortName", TestValue::text(self.port.clone()))
            .value("entityClassName", TestValue::text(self.class_name.clone()));
        if let Some(guid) = self.reference {
            instance = instance.value("entityClassReference", TestValue::Reference(guid));
        }
        if let Some(loadout) = &self.loadout {
            instance = instance.value("loadout", loadout.value());
        }
        TestValue::Struct(instance)
    }
}

/// An item port definition of a synthetic Entity.
#[derive(Debug, Clone, PartialEq)]
pub struct TestPort {
    name: String,
    sizes: (u32, u32),
    types: Vec<(String, Vec<String>)>,
    helper: Option<(String, [f32; 3], [f32; 3])>,
    default_item: Option<u128>,
}

impl TestPort {
    /// Creates a port of size 1 without types, helper and default item.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sizes: (1, 1),
            types: Vec::new(),
            helper: None,
            default_item: None,
        }
    }

    /// Sets the smallest and largest item size the port takes.
    #[must_use]
    pub const fn sizes(mut self, min: u32, max: u32) -> Self {
        self.sizes = (min, max);
        self
    }

    /// Adds an item type the port takes, with its sub-types.
    #[must_use]
    pub fn item_type<S: Into<String>>(
        mut self,
        name: impl Into<String>,
        sub_types: impl IntoIterator<Item = S>,
    ) -> Self {
        self.types
            .push((name.into(), sub_types.into_iter().map(Into::into).collect()));
        self
    }

    /// Attaches the port to the bone helper `name`, offset by `position` and `rotation`.
    #[must_use]
    pub fn helper(
        mut self,
        name: impl Into<String>,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Self {
        self.helper = Some((name.into(), position, rotation));
        self
    }

    /// Sets the Entity the port holds when no Loadout names one.
    #[must_use]
    pub const fn default_item(mut self, guid: u128) -> Self {
        self.default_item = Some(guid);
        self
    }

    fn value(&self) -> TestValue {
        let mut instance = TestInstance::new("SItemPortDef")
            .value("Name", TestValue::text(self.name.clone()))
            .value("DisplayName", TestValue::text("@LOC_EMPTY"))
            .value("Flags", TestValue::text(""))
            .value("MinSize", TestValue::UInt32(self.sizes.0))
            .value("MaxSize", TestValue::UInt32(self.sizes.1))
            .value(
                "Types",
                TestValue::Array(
                    self.types
                        .iter()
                        .map(|(name, sub_types)| {
                            TestValue::Struct(
                                TestInstance::new("SItemPortDefTypes")
                                    .value("Type", TestValue::text(name.clone()))
                                    .value(
                                        "SubTypes",
                                        TestValue::Array(
                                            sub_types
                                                .iter()
                                                .cloned()
                                                .map(TestValue::Text)
                                                .collect(),
                                        ),
                                    ),
                            )
                        })
                        .collect(),
                ),
            );
        if let Some(guid) = self.default_item {
            instance = instance.value(
                "defaultItem",
                TestValue::Struct(
                    TestInstance::new("SItemPortDefaultItemDef")
                        .value("entityClass", TestValue::Reference(guid)),
                ),
            );
        }
        if let Some((name, position, rotation)) = &self.helper {
            instance = instance.value(
                "AttachmentImplementation",
                helper(name, *position, *rotation),
            );
        }
        TestValue::Struct(instance)
    }
}

fn helper(name: &str, position: [f32; 3], rotation: [f32; 3]) -> TestValue {
    let xyz = |type_name: &str, [x, y, z]: [f32; 3]| {
        TestValue::Struct(
            TestInstance::new(type_name)
                .value("x", TestValue::Single(x))
                .value("y", TestValue::Single(y))
                .value("z", TestValue::Single(z)),
        )
    };
    let offset = TestInstance::new("QuatT")
        .value("Rotation", xyz("Ang3", rotation))
        .value("Position", xyz("Vec3", position));
    let helper = TestInstance::new("SItemPortDefHelper")
        .value("Name", TestValue::text(name))
        .value("ItemOffsetHelperName", TestValue::text(""))
        .value("Offset", TestValue::Struct(offset));
    let node =
        TestInstance::new("SItemPortDefHelperNode").value("Helper", TestValue::Struct(helper));
    TestValue::Struct(
        TestInstance::new("SItemPortDefAttachmentImplementationBone")
            .value("Helper", TestValue::Struct(node)),
    )
}
