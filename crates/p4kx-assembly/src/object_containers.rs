// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use crate::read::{struct_field, struct_items, text, xyz};

const VEHICLE_PARAMS: &str = "VehicleComponentParams";
const OBJECT_CONTAINER_PARAMS: &str = "SObjectContainerComponentParams";

/// Which component of an Entity's Record names an object container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerSource {
    /// `VehicleComponentParams.objectContainers`: placed at a bone of the Entity's model.
    Vehicle,
    /// `SObjectContainerComponentParams.objectContainer`: placed at the Entity's origin.
    Item,
}

impl ContainerSource {
    /// Returns the stable code of the source, e.g. `vehicle`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Vehicle => "vehicle",
            Self::Item => "item",
        }
    }
}

/// A socpak container an Entity's Record names, with where it sits relative to the Entity.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectContainerRef {
    pub(crate) file: String,
    pub(crate) bone: String,
    pub(crate) position: [f32; 3],
    pub(crate) rotation: [f32; 3],
    pub(crate) source: ContainerSource,
}

impl ObjectContainerRef {
    /// Returns the container's file as the Record spells it, a path below `Data`, e.g.
    /// `objectcontainers\ships\rsi\constellation\base_int_front_main.socpak`.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Returns the bone of the Entity's model the container is placed at; empty for none.
    #[must_use]
    pub fn bone(&self) -> &str {
        &self.bone
    }

    /// Returns the offset from the bone in metres.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    /// Returns the rotation of the offset as `Ang3` in degrees.
    #[must_use]
    pub const fn rotation(&self) -> [f32; 3] {
        self.rotation
    }

    /// Returns the component the container comes from.
    #[must_use]
    pub const fn source(&self) -> ContainerSource {
        self.source
    }
}

/// Reads the object containers of the components of an Entity's Record, the vehicle's first.
pub(crate) fn read(components: &[&StructValue<'_>]) -> Vec<ObjectContainerRef> {
    let mut found = Vec::new();
    for component in components {
        match component.type_name() {
            VEHICLE_PARAMS => {
                for container in struct_items(component, "objectContainers") {
                    let file = text(container, "fileName").trim();
                    if file.is_empty() {
                        continue;
                    }
                    let offset = struct_field(container, "Offset");
                    found.push(ObjectContainerRef {
                        file: file.to_owned(),
                        bone: text(container, "boneName").trim().to_owned(),
                        position: xyz(offset.and_then(|o| struct_field(o, "Position"))),
                        rotation: xyz(offset.and_then(|o| struct_field(o, "Rotation"))),
                        source: ContainerSource::Vehicle,
                    });
                }
            }
            OBJECT_CONTAINER_PARAMS => {
                let file = text(component, "objectContainer").trim();
                if !file.is_empty() {
                    found.push(ObjectContainerRef {
                        file: file.to_owned(),
                        bone: String::new(),
                        position: [0.0; 3],
                        rotation: [0.0; 3],
                        source: ContainerSource::Item,
                    });
                }
            }
            _ => {}
        }
    }
    found
}
