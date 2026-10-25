// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TestPort;
use crate::entity_cargo::{container_component, physics_component};
use crate::entity_loadout::TestLoadout;
use crate::entity_schema::attachable_component;
use crate::entity_vehicle::{
    ContainerPlacement, GeometryVariant, MaterialChoice, geometry_component,
    item_container_component, vehicle_component,
};
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
    material_choices: Vec<MaterialChoice>,
    vehicle: Option<(String, String)>,
    landing_system: Option<u128>,
    attach_type: Option<String>,
    sub_type: Option<String>,
    size: Option<i32>,
    tags: Option<String>,
    mass: Option<f32>,
    container: Option<u128>,
    extra: Vec<TestValue>,
    object_containers: Vec<ContainerPlacement>,
    item_container: Option<String>,
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
            material_choices: Vec::new(),
            vehicle: None,
            landing_system: None,
            attach_type: None,
            sub_type: None,
            size: None,
            tags: None,
            mass: None,
            container: None,
            extra: Vec::new(),
            object_containers: Vec::new(),
            item_container: None,
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

    /// Adds the material library `material` for the node of the material tree tagged `tags`, e.g.
    /// `hairVar_Brown>Male` for the node `Male` below `hairVar_Brown`; the database needs
    /// [`entity_schema`](crate::entity_schema).
    #[must_use]
    pub fn material_choice(mut self, tags: impl Into<String>, material: impl Into<String>) -> Self {
        self.material_choices.push((tags.into(), material.into()));
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

    /// Adds an object container to the Entity's vehicle component: the socpak `file`, the `bone`
    /// of the model it sits at (empty for none) and the offset from it, `position` in metres and
    /// `rotation` as `Ang3` in degrees. The component is written with an empty vehicle definition
    /// when [`Self::vehicle`] is not called.
    #[must_use]
    pub fn object_container(
        mut self,
        file: impl Into<String>,
        bone: impl Into<String>,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Self {
        self.object_containers
            .push((file.into(), bone.into(), position, rotation));
        self
    }

    /// Gives the Entity an `SObjectContainerComponentParams` naming the socpak `file`.
    #[must_use]
    pub fn item_container(mut self, file: impl Into<String>) -> Self {
        self.item_container = Some(file.into());
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
    #[expect(
        clippy::too_many_lines,
        reason = "one component per optional part, in the order of the game data"
    )]
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
        if self.geometry.is_some()
            || !self.geometry_variants.is_empty()
            || !self.material_choices.is_empty()
        {
            let (path, material) = self.geometry.clone().unwrap_or_default();
            components.push(geometry_component(
                &path,
                material.as_deref(),
                &self.geometry_variants,
                self.palette.as_ref(),
                &self.material_choices,
            ));
        }
        if self.vehicle.is_some() || !self.object_containers.is_empty() {
            let (definition, modification) = self.vehicle.clone().unwrap_or_default();
            components.push(vehicle_component(
                &definition,
                &modification,
                self.landing_system,
                &self.object_containers,
            ));
        }
        components.extend(self.item_container.as_deref().map(item_container_component));
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
