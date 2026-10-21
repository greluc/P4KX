use crate::{DataForgeBuilder, TestStruct, TestType};

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
        .chain(attachable_structs())
        .fold(builder, DataForgeBuilder::structure)
}

fn attachable_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("SAttachableComponentParams")
            .parent("DataForgeComponentParams")
            .field("AttachDef", class("SItemDefinition")),
        TestStruct::new("SItemDefinition").field("Type", TestType::String),
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
