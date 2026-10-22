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

fn singles_component(name: &str, fields: &[&str]) -> TestStruct {
    fields.iter().fold(component(name), |structure, field| {
        structure.field(*field, TestType::Single)
    })
}

fn weapons() -> Vec<TestStruct> {
    [
        TestStruct::new("EntityClassDefinition")
            .array("Components", pointer("DataForgeComponentParams")),
        TestStruct::new("DataForgeComponentParams"),
        component("SAttachableComponentParams").field("AttachDef", class("SItemDefinition")),
        TestStruct::new("SItemDefinition").field("Type", TestType::Enum("EItemType".to_owned())),
        component("SCItemWeaponComponentParams")
            .array("fireActions", pointer("SWeaponActionParams")),
        TestStruct::new("SWeaponActionParams"),
        TestStruct::new("SWeaponActionFireRapidParams")
            .parent("SWeaponActionParams")
            .field("name", TestType::String)
            .field("fireRate", TestType::Single)
            .field("heatPerShot", TestType::Single)
            .field("launchParams", class("SProjectileLauncher")),
        TestStruct::new("SWeaponActionFireChargedParams")
            .parent("SWeaponActionParams")
            .field("chargeTime", TestType::Single)
            .field("overchargeTime", TestType::Single)
            .field("cooldownTime", TestType::Single)
            .field("weaponAction", pointer("SWeaponActionParams")),
        TestStruct::new("SProjectileLauncher")
            .field("pellets", TestType::Int32)
            .field("ammoCost", TestType::Int32),
        component("SAmmoContainerComponentParams")
            .field("ammoParamsRecord", TestType::Reference)
            .field("maxAmmoCount", TestType::Int32),
        TestStruct::new("AmmoParams")
            .field("speed", TestType::Single)
            .field("lifetime", TestType::Single)
            .field("projectileParams", pointer("BulletProjectileParams")),
        TestStruct::new("BulletProjectileParams")
            .field("damage", class("DamageInfo"))
            .field("detonationParams", pointer("ProjectileDetonationParams")),
        TestStruct::new("ProjectileDetonationParams")
            .field("explosionParams", class("ExplosionParams")),
        TestStruct::new("ExplosionParams")
            .field("minRadius", TestType::Single)
            .field("maxRadius", TestType::Single)
            .field("damage", class("DamageInfo")),
        singles("DamageInfo", &["DamagePhysical", "DamageEnergy"]),
    ]
    .into()
}

fn components() -> Vec<TestStruct> {
    vec![
        singles_component(
            "SCItemShieldGeneratorParams",
            &[
                "MaxShieldHealth",
                "MaxShieldRegen",
                "DecayRatio",
                "DownedRegenDelay",
                "DamagedRegenDelay",
            ],
        ),
        singles_component(
            "SCItemQuantumDriveParams",
            &["quantumFuelRequirement", "jumpRange", "disconnectRange"],
        )
        .field("params", class("QuantumDriveParams")),
        singles(
            "QuantumDriveParams",
            &[
                "driveSpeed",
                "cooldownTime",
                "spoolUpTime",
                "stageOneAccelRate",
                "stageTwoAccelRate",
                "engageSpeed",
            ],
        ),
        component("ItemResourceComponentParams").array("states", class("ItemResourceState")),
        TestStruct::new("ItemResourceState")
            .field("name", TestType::String)
            .field("signatureParams", class("SignatureParams"))
            .array("deltas", pointer("ItemResourceDelta")),
        TestStruct::new("SignatureParams")
            .field("EMSignature", class("Signature"))
            .field("IRSignature", class("Signature")),
        singles("Signature", &["nominalSignature", "decayRate"]),
        TestStruct::new("ItemResourceDelta"),
        TestStruct::new("ItemResourceDeltaGeneration")
            .parent("ItemResourceDelta")
            .field("generation", class("ItemResourceGeneration")),
        TestStruct::new("ItemResourceDeltaConversion")
            .parent("ItemResourceDelta")
            .field("consumption", class("ItemResourceGeneration"))
            .field("generation", class("ItemResourceGeneration")),
        singles("SStandardResourceUnit", &["standardResourceUnits"]),
        TestStruct::new("ItemResourceGeneration")
            .field("resource", TestType::String)
            .field(
                "resourceAmountPerSecond",
                class("SPowerSegmentResourceUnit"),
            ),
        singles("SPowerSegmentResourceUnit", &["units"]),
    ]
}

fn others() -> Vec<TestStruct> {
    vec![
        component("SCItemMissileParams")
            .field("explosionParams", class("ExplosionParams"))
            .field("targetingParams", class("MissileTargetingParams"))
            .field("GCSParams", class("MissileGcsParams")),
        singles("MissileGcsParams", &["linearSpeed"]),
        singles(
            "MissileTargetingParams",
            &["lockTime", "lockingAngle", "lockRangeMin", "lockRangeMax"],
        )
        .field("trackingSignalType", TestType::String),
        singles_component(
            "EntityComponentPowerConnection",
            &["PowerBase", "PowerDraw", "PowerToEM", "DecayRateOfEM"],
        ),
        singles_component(
            "EntityComponentHeatConnection",
            &[
                "ThermalEnergyBase",
                "ThermalEnergyDraw",
                "TemperatureToIR",
                "OverheatTemperature",
            ],
        ),
        singles_component("SHealthComponentParams", &["Health"]),
    ]
}

pub(super) fn builder() -> DataForgeBuilder {
    weapons()
        .into_iter()
        .chain(components())
        .chain(others())
        .fold(
            DataForgeBuilder::new().version(8).enumeration(
                "EItemType",
                [
                    "WeaponGun",
                    "Shield",
                    "QuantumDrive",
                    "PowerPlant",
                    "Cooler",
                    "Missile",
                    "Misc",
                ],
            ),
            DataForgeBuilder::structure,
        )
}
