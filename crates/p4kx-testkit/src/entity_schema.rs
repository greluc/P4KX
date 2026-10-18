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
        .fold(builder, DataForgeBuilder::structure)
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
            .field("entityClassReference", TestType::Reference)
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
        TestStruct::new("SItemPortDefaultItemDef").field("entityClass", TestType::Reference),
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
