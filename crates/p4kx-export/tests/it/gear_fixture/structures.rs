use p4kx_testkit::{DataForgeBuilder, TestStruct, TestType};

fn class(name: &str) -> TestType {
    TestType::Class(name.to_owned())
}

fn pointer(name: &str) -> TestType {
    TestType::StrongPointer(name.to_owned())
}

fn component(name: &str) -> TestStruct {
    TestStruct::new(name).parent("DataForgeComponentParams")
}

fn singles(name: &str, fields: &[&str]) -> TestStruct {
    fields
        .iter()
        .fold(TestStruct::new(name), |structure, field| {
            structure.field(*field, TestType::Single)
        })
}

fn worn() -> Vec<TestStruct> {
    vec![
        TestStruct::new("EntityClassDefinition")
            .array("Components", pointer("DataForgeComponentParams")),
        TestStruct::new("DataForgeComponentParams"),
        component("SAttachableComponentParams").field("AttachDef", class("SItemDefinition")),
        TestStruct::new("SItemDefinition").field("Type", TestType::Enum("EItemType".to_owned())),
        component("SCItemSuitArmorParams")
            .field("integrityMilestoneToBreak", TestType::Single)
            .field("damageResistance", TestType::Reference),
        TestStruct::new("DamageResistanceMacro")
            .field("damageResistance", class("DamageResistance")),
        TestStruct::new("DamageResistance")
            .field("IgnoreMeleeDamage", TestType::Bool)
            .field("PhysicalResistance", class("DamageResistanceEntry"))
            .field("EnergyResistance", class("DamageResistanceEntry")),
        singles(
            "DamageResistanceEntry",
            &["Multiplier", "Threshold", "DamageCap", "DamageCapLimit"],
        ),
        component("SCItemClothingParams")
            .field("TemperatureResistance", class("ClothingTemperature"))
            .field("RadiationResistance", class("ClothingRadiation"))
            .field("Flight", class("ClothingFlight")),
        singles("ClothingTemperature", &["MinResistance", "MaxResistance"]),
        singles(
            "ClothingRadiation",
            &["MaximumRadiationCapacity", "RadiationDissipationRate"],
        ),
        singles("ClothingFlight", &["gForceResistance"]),
        component("SCItemRadarComponentParams")
            .field("aimAssist", class("RadarAimAssist"))
            .field("pingProperties", class("RadarPing")),
        singles(
            "RadarAimAssist",
            &[
                "distanceMinAssignment",
                "distanceMaxAssignment",
                "outsideRangeBufferDistance",
            ],
        ),
        singles("RadarPing", &["cooldownTime"]),
    ]
}

fn weapons() -> Vec<TestStruct> {
    vec![
        component("SCItemWeaponComponentParams")
            .array("fireActions", pointer("SWeaponActionParams")),
        TestStruct::new("SWeaponActionParams"),
        singles(
            "SWeaponActionFireTractorBeamParams",
            &[
                "minForce",
                "maxForce",
                "minDistance",
                "maxDistance",
                "fullStrengthDistance",
                "maxVolume",
                "maxAngle",
            ],
        )
        .parent("SWeaponActionParams"),
        TestStruct::new("SWeaponActionWrapParams")
            .parent("SWeaponActionParams")
            .field("weaponAction", pointer("SWeaponActionParams")),
        TestStruct::new("SWeaponActionParallelParams")
            .parent("SWeaponActionParams")
            .array("weaponActions", pointer("SWeaponActionParams")),
        TestStruct::new("SWeaponActionFireBeamParams")
            .parent("SWeaponActionParams")
            .field("damagePerSecond", class("DamageInfo")),
        singles("DamageInfo", &["DamagePhysical", "DamageEnergy"]),
        component("EntityComponentAttachableModifierParams")
            .array("modifiers", pointer("ItemModifiersParams")),
        TestStruct::new("ItemModifiersParams"),
        TestStruct::new("ItemWeaponModifiersParams")
            .parent("ItemModifiersParams")
            .field("weaponModifier", pointer("SWeaponModifierParams")),
        TestStruct::new("SWeaponModifierParams").field("weaponStats", class("SWeaponStats")),
        TestStruct::new("SWeaponStats")
            .field("fireRateMultiplier", TestType::Single)
            .field("salvageModifier", pointer("SSalvageModifier"))
            .field("tractorModifier", pointer("STractorModifier")),
        singles(
            "SSalvageModifier",
            &[
                "salvageSpeedMultiplier",
                "radiusMultiplier",
                "extractionEfficiency",
            ],
        ),
        singles(
            "STractorModifier",
            &[
                "forceMultiplier",
                "fullStrengthDistMultiplier",
                "maxDistMultiplier",
                "maxVolumeMultiplier",
            ],
        ),
    ]
}

pub(super) fn database() -> DataForgeBuilder {
    worn().into_iter().chain(weapons()).fold(
        DataForgeBuilder::new().version(8).enumeration(
            "EItemType",
            [
                "Char_Armor_Torso",
                "Char_Clothing_Hat",
                "Radar",
                "TractorBeam",
                "SalvageModifier",
                "WeaponGun",
            ],
        ),
        DataForgeBuilder::structure,
    )
}
