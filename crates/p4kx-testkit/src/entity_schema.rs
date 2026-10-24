// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::entity_cargo::cargo_structs;
use crate::entity_vehicle::vehicle_structs;
use crate::{DataForgeBuilder, TestInstance, TestStruct, TestType, TestValue};

/// Adds the structs of Entities, their default Loadouts and their item ports to `builder`.
///
/// They are named and shaped as in the game data: `EntityClassDefinition` with a `Components`
/// array, `SEntityComponentDefaultLoadoutParams`, the manual and XML Loadouts, and
/// `SItemPortContainerComponentParams` with `SItemPortDef` ports.
#[must_use]
pub fn entity_schema(builder: DataForgeBuilder) -> DataForgeBuilder {
    loadout_structs()
        .into_iter()
        .chain(port_structs())
        .chain(geometry_structs())
        .chain(vehicle_structs())
        .chain(attachable_structs())
        .chain(cargo_structs())
        .fold(builder, DataForgeBuilder::structure)
}

fn attachable_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("SAttachableComponentParams")
            .parent("DataForgeComponentParams")
            .field("AttachDef", class("SItemDefinition")),
        TestStruct::new("SItemDefinition")
            .field("Type", TestType::String)
            .field("SubType", TestType::String)
            .field("Size", TestType::Int32)
            .field("Tags", TestType::String),
    ]
}

fn geometry_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("SGeometryResourceParams")
            .parent("DataForgeComponentParams")
            .field("Geometry", class("SGeometryNodeParams")),
        TestStruct::new("SGeometryNodeParams").field("Geometry", class("SGeometryDataParams")),
        TestStruct::new("SGeometryDataParams")
            .field("Geometry", class("GlobalResourceGeometry"))
            .field("Material", class("GlobalResourceMaterial")),
        TestStruct::new("GlobalResourceGeometry").field("path", TestType::String),
        TestStruct::new("GlobalResourceMaterial").field("path", TestType::String),
    ]
}

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

fn loadout_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("EntityClassDefinition")
            .field("Category", TestType::String)
            .array("Components", pointer("DataForgeComponentParams")),
        TestStruct::new("DataForgeComponentParams"),
        TestStruct::new("SEntityComponentDefaultLoadoutParams")
            .parent("DataForgeComponentParams")
            .field("loadout", pointer("SItemPortLoadoutBaseParams")),
        TestStruct::new("SItemPortLoadoutBaseParams")
            .field("SkipInventoryItemsOnMissionEntities", TestType::Bool),
        TestStruct::new("SItemPortLoadoutManualParams")
            .parent("SItemPortLoadoutBaseParams")
            .array("entries", class("SItemPortLoadoutEntryParams")),
        TestStruct::new("SItemPortLoadoutXMLParams")
            .parent("SItemPortLoadoutBaseParams")
            .field("loadoutPath", TestType::String),
        TestStruct::new("SItemPortLoadoutEntryParams")
            .field("itemPortName", TestType::String)
            .field("entityClassName", TestType::String)
            .field(
                "entityClassReference",
                TestType::ReferenceTo("EntityClassDefinition".to_owned()),
            )
            .field("loadout", pointer("SItemPortLoadoutBaseParams")),
    ]
}

fn port_structs() -> Vec<TestStruct> {
    let xyz = |name: &str| {
        TestStruct::new(name)
            .field("x", TestType::Single)
            .field("y", TestType::Single)
            .field("z", TestType::Single)
    };
    vec![
        TestStruct::new("SItemPortContainerComponentParams")
            .parent("DataForgeComponentParams")
            .array("Ports", class("SItemPortDef")),
        TestStruct::new("SItemPortDef")
            .field("Name", TestType::String)
            .field("DisplayName", TestType::Locale)
            .field("Flags", TestType::String)
            .field("MinSize", TestType::UInt32)
            .field("MaxSize", TestType::UInt32)
            .field("PortTags", TestType::String)
            .field("RequiredPortTags", TestType::String)
            .field("defaultItem", class("SItemPortDefaultItemDef"))
            .array("Types", class("SItemPortDefTypes"))
            .field(
                "AttachmentImplementation",
                pointer("SItemPortDefAttachmentImplementationBase"),
            ),
        TestStruct::new("SItemPortDefaultItemDef").field(
            "entityClass",
            TestType::ReferenceTo("EntityClassDefinition".to_owned()),
        ),
        TestStruct::new("SItemPortDefTypes")
            .field("Type", TestType::String)
            .array("SubTypes", TestType::String),
        TestStruct::new("SItemPortDefAttachmentImplementationBase"),
        TestStruct::new("SItemPortDefAttachmentImplementationBone")
            .parent("SItemPortDefAttachmentImplementationBase")
            .field("Helper", class("SItemPortDefHelperNode")),
        TestStruct::new("SItemPortDefHelperNode").field("Helper", class("SItemPortDefHelper")),
        TestStruct::new("SItemPortDefHelper")
            .field("Name", TestType::String)
            .field("ItemOffsetHelperName", TestType::String)
            .field("Offset", class("QuatT")),
        TestStruct::new("QuatT")
            .field("Rotation", class("Ang3"))
            .field("Position", class("Vec3")),
        xyz("Ang3"),
        xyz("Vec3"),
    ]
}

pub(crate) fn attachable_component(
    attach_type: &str,
    sub_type: Option<&str>,
    size: Option<i32>,
    tags: Option<&str>,
) -> TestValue {
    let mut definition =
        TestInstance::new("SItemDefinition").value("Type", TestValue::text(attach_type));
    if let Some(sub_type) = sub_type {
        definition = definition.value("SubType", TestValue::text(sub_type));
    }
    if let Some(size) = size {
        definition = definition.value("Size", TestValue::Int32(size));
    }
    if let Some(tags) = tags {
        definition = definition.value("Tags", TestValue::text(tags));
    }
    TestValue::Struct(
        TestInstance::new("SAttachableComponentParams")
            .value("AttachDef", TestValue::Struct(definition)),
    )
}
