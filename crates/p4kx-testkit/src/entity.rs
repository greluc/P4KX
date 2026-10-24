// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TestPort;
use crate::entity_cargo::{container_component, physics_component};
use crate::entity_schema::attachable_component;
use crate::entity_vehicle::{GeometryVariant, geometry_component, vehicle_component};
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
    palette: Option<(u128, String)>,
    geometry_variants: Vec<GeometryVariant>,
    vehicle: Option<(String, String)>,
    landing_system: Option<u128>,
    attach_type: Option<String>,
    sub_type: Option<String>,
    size: Option<i32>,
    tags: Option<String>,
    mass: Option<f32>,
    container: Option<u128>,
    extra: Vec<TestValue>,
}

impl TestEntity {
    /// Adds a component of a struct the database declares, after the components the other
    /// methods add.
    #[must_use]
    pub fn component(mut self, component: TestValue) -> Self {
        self.extra.push(component);
        self
    }

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
            palette: None,
            geometry_variants: Vec::new(),
            vehicle: None,
            landing_system: None,
            attach_type: None,
            sub_type: None,
            size: None,
            tags: None,
            mass: None,
            container: None,
            extra: Vec::new(),
        }
    }

    /// Sets the Entity's `AttachDef.Size`; without [`Self::attach_type`] the type is empty.
    #[must_use]
    pub const fn size(mut self, size: i32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the Entity's `AttachDef.SubType`, e.g. `Gun`; without [`Self::attach_type`] the type
    /// is empty.
    #[must_use]
    pub fn sub_type(mut self, sub_type: impl Into<String>) -> Self {
        self.sub_type = Some(sub_type.into());
        self
    }

    /// Sets the Entity's `AttachDef.Tags`, words separated by white space; the database needs
    /// [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn tags(mut self, tags: impl Into<String>) -> Self {
        self.tags = Some(tags.into());
        self
    }

    /// Gives the Entity a rigid physics controller of `mass` kilograms.
    #[must_use]
    pub const fn physics_mass(mut self, mass: f32) -> Self {
        self.mass = Some(mass);
        self
    }

    /// Makes the Entity an inventory container referring to the `InventoryContainer` Record
    /// `guid`, see [`open_container_record`](crate::open_container_record).
    #[must_use]
    pub const fn container(mut self, guid: u128) -> Self {
        self.container = Some(guid);
        self
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

    /// Makes the geometry name the tint palette Record `guid` (see
    /// [`tint_palette_record`](crate::tint_palette_record)) and its child `child` (empty for the
    /// root); needs [`Self::geometry`].
    #[must_use]
    pub fn palette(mut self, guid: u128, child: impl Into<String>) -> Self {
        self.palette = Some((guid, child.into()));
        self
    }

    /// Adds a sub-geometry tagged `tags`, words separated by white space, such as `Male`, naming
    /// the model `path` and, if given, the material library `material`; without
    /// [`Self::geometry`] the main geometry names no model. The database needs
    /// [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn geometry_variant(
        mut self,
        tags: impl Into<String>,
        path: impl Into<String>,
        material: Option<&str>,
    ) -> Self {
        self.geometry_variants
            .push((tags.into(), path.into(), material.map(str::to_owned)));
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

    /// Gives the Entity a vehicle component naming the vehicle implementation file `definition`
    /// and the Modification `modification` (empty for none); the database needs
    /// [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn vehicle(
        mut self,
        definition: impl Into<String>,
        modification: impl Into<String>,
    ) -> Self {
        self.vehicle = Some((definition.into(), modification.into()));
        self
    }

    /// Makes the Entity's vehicle component refer to the landing gear Record `guid`, see
    /// [`landing_system_record`](crate::landing_system_record).
    #[must_use]
    pub const fn landing_system(mut self, guid: u128) -> Self {
        self.landing_system = Some(guid);
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
        if self.geometry.is_some() || !self.geometry_variants.is_empty() {
            let (path, material) = self.geometry.clone().unwrap_or_default();
            components.push(geometry_component(
                &path,
                material.as_deref(),
                &self.geometry_variants,
                self.palette.as_ref(),
            ));
        }
        components.extend(self.vehicle.as_ref().map(|(definition, modification)| {
            vehicle_component(definition, modification, self.landing_system)
        }));
        if self.attach_type.is_some()
            || self.sub_type.is_some()
            || self.size.is_some()
            || self.tags.is_some()
        {
            components.push(attachable_component(
                self.attach_type.as_deref().unwrap_or_default(),
                self.sub_type.as_deref(),
                self.size,
                self.tags.as_deref(),
            ));
        }
        components.extend(self.mass.map(physics_component));
        components.extend(self.container.map(container_component));
        components.extend(self.extra.iter().cloned());
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
