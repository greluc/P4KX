// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestRecord, TestStruct, TestType, TestValue};

pub(crate) fn geometry_component(path: &str, material: Option<&str>) -> TestValue {
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

pub(crate) fn vehicle_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("VehicleComponentParams")
            .parent("DataForgeComponentParams")
            .field("vehicleDefinition", TestType::String)
            .field("modification", TestType::String)
            .field(
                "landingSystem",
                TestType::ReferenceTo("VehicleLandingGearSystem".to_owned()),
            ),
        TestStruct::new("VehicleLandingGearSystem")
            .array("gears", TestType::Class("VehicleLandingGear".to_owned())),
        TestStruct::new("VehicleLandingGear")
            .field("bone", TestType::String)
            .field("mass", TestType::Single)
            .field(
                "geometry",
                TestType::Class("GlobalResourceGeometry".to_owned()),
            ),
    ]
}

pub(crate) fn vehicle_component(
    definition: &str,
    modification: &str,
    landing_system: Option<u128>,
) -> TestValue {
    let mut component = TestInstance::new("VehicleComponentParams")
        .value("vehicleDefinition", TestValue::text(definition))
        .value("modification", TestValue::text(modification));
    if let Some(guid) = landing_system {
        component = component.value("landingSystem", TestValue::Reference(guid));
    }
    TestValue::Struct(component)
}

/// Returns the Record of type `VehicleLandingGearSystem` named `name` whose gears move the bone
/// of each `(bone, geometry path, mass)`; a vehicle's `landingSystem` refers to it.
#[must_use]
pub fn landing_system_record(guid: u128, name: &str, gears: &[(&str, &str, f32)]) -> TestRecord {
    let gears = gears
        .iter()
        .map(|(bone, geometry, mass)| {
            TestValue::Struct(
                TestInstance::new("VehicleLandingGear")
                    .value("bone", TestValue::text(*bone))
                    .value("mass", TestValue::Single(*mass))
                    .value(
                        "geometry",
                        TestValue::Struct(
                            TestInstance::new("GlobalResourceGeometry")
                                .value("path", TestValue::text(*geometry)),
                        ),
                    ),
            )
        })
        .collect();
    TestRecord::new(
        guid,
        "VehicleLandingGearSystem",
        format!("VehicleLandingGearSystem.{name}"),
        format!("libs/foundry/records/entities/scitem/ships/landingsystem/{name}.xml"),
    )
    .value("gears", TestValue::Array(gears))
}
