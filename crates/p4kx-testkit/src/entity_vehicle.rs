use crate::{TestInstance, TestStruct, TestType, TestValue};

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
            .field("modification", TestType::String),
        TestStruct::new("VehicleLandingGearSystem")
            .parent("DataForgeComponentParams")
            .array("gears", TestType::Class("VehicleLandingGear".to_owned())),
        TestStruct::new("VehicleLandingGear")
            .field("bone", TestType::String)
            .field(
                "geometry",
                TestType::Class("GlobalResourceGeometry".to_owned()),
            ),
    ]
}

pub(crate) fn vehicle_component(definition: &str, modification: &str) -> TestValue {
    TestValue::Struct(
        TestInstance::new("VehicleComponentParams")
            .value("vehicleDefinition", TestValue::text(definition))
            .value("modification", TestValue::text(modification)),
    )
}

pub(crate) fn landing_gear_component(gears: &[(String, String)]) -> TestValue {
    let gears = gears
        .iter()
        .map(|(bone, geometry)| {
            TestValue::Struct(
                TestInstance::new("VehicleLandingGear")
                    .value("bone", TestValue::text(bone.clone()))
                    .value(
                        "geometry",
                        TestValue::Struct(
                            TestInstance::new("GlobalResourceGeometry")
                                .value("path", TestValue::text(geometry.clone())),
                        ),
                    ),
            )
        })
        .collect();
    TestValue::Struct(
        TestInstance::new("VehicleLandingGearSystem").value("gears", TestValue::Array(gears)),
    )
}
