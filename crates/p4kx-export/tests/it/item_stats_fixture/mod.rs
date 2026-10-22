//! A synthetic Archive with one item of each stat kind and an item without stats.

mod structures;

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestValue,
};

pub(crate) const GUN: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const SHIELD: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const DRIVE: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const PLANT: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0004;
pub(crate) const COOLER: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0005;
pub(crate) const MISSILE: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0006;
pub(crate) const PLAIN: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0007;
pub(crate) const DANGLING_GUN: u128 = 0x1000_0000_0000_4000_8000_0000_0000_0008;
const AMMO: u128 = 0x2000_0000_0000_4000_8000_0000_0000_0001;
const MISSING_AMMO: u128 = 0x2000_0000_0000_4000_8000_0000_0000_00ff;

fn single(name: &str, value: f32) -> (String, TestValue) {
    (name.to_owned(), TestValue::Single(value))
}

fn instance(type_name: &str, values: Vec<(String, TestValue)>) -> TestValue {
    TestValue::Struct(
        values
            .into_iter()
            .fold(TestInstance::new(type_name), |instance, (name, value)| {
                instance.value(name, value)
            }),
    )
}

fn attach(kind: &str) -> TestValue {
    instance(
        "SAttachableComponentParams",
        vec![(
            "AttachDef".to_owned(),
            instance(
                "SItemDefinition",
                vec![("Type".to_owned(), TestValue::text(kind))],
            ),
        )],
    )
}

fn entity(guid: u128, class: &str, kind: &str, extra: Vec<TestValue>) -> TestRecord {
    let mut components = vec![attach(kind)];
    components.extend(extra);
    TestRecord::new(
        guid,
        "EntityClassDefinition",
        format!("EntityClassDefinition.{class}"),
        format!("libs/foundry/records/entities/scitem/ships/{class}.xml"),
    )
    .value("Components", TestValue::Array(components))
}

fn damage(physical: f32, energy: f32) -> TestValue {
    instance(
        "DamageInfo",
        vec![
            single("DamagePhysical", physical),
            single("DamageEnergy", energy),
        ],
    )
}

fn launch(pellets: i32, cost: i32) -> TestValue {
    instance(
        "SProjectileLauncher",
        vec![
            ("pellets".to_owned(), TestValue::Int32(pellets)),
            ("ammoCost".to_owned(), TestValue::Int32(cost)),
        ],
    )
}

fn rapid(rate: f32) -> TestValue {
    instance(
        "SWeaponActionFireRapidParams",
        vec![
            ("name".to_owned(), TestValue::text("rapid")),
            single("fireRate", rate),
            single("heatPerShot", 0.5),
            ("launchParams".to_owned(), launch(1, 1)),
        ],
    )
}

fn charged() -> TestValue {
    instance(
        "SWeaponActionFireChargedParams",
        vec![
            single("chargeTime", 1.5),
            single("overchargeTime", 3.0),
            single("cooldownTime", 0.25),
            ("weaponAction".to_owned(), rapid(120.0)),
        ],
    )
}

fn gun(guid: u128, class: &str, ammo: u128) -> TestRecord {
    entity(
        guid,
        class,
        "WeaponGun",
        vec![
            instance(
                "SCItemWeaponComponentParams",
                vec![(
                    "fireActions".to_owned(),
                    TestValue::Array(vec![rapid(600.0), charged()]),
                )],
            ),
            instance(
                "SAmmoContainerComponentParams",
                vec![
                    ("ammoParamsRecord".to_owned(), TestValue::Reference(ammo)),
                    ("maxAmmoCount".to_owned(), TestValue::Int32(200)),
                ],
            ),
        ],
    )
}

fn ammo() -> TestRecord {
    TestRecord::new(
        AMMO,
        "AmmoParams",
        "AmmoParams.Ammo_Test",
        "libs/foundry/records/ammoparams/vehicle/ammo_test.xml",
    )
    .value("speed", TestValue::Single(1100.0))
    .value("lifetime", TestValue::Single(2.5))
    .value(
        "projectileParams",
        instance(
            "BulletProjectileParams",
            vec![
                ("damage".to_owned(), damage(30.0, 2.0)),
                (
                    "detonationParams".to_owned(),
                    instance(
                        "ProjectileDetonationParams",
                        vec![(
                            "explosionParams".to_owned(),
                            instance(
                                "ExplosionParams",
                                vec![
                                    single("minRadius", 1.0),
                                    single("maxRadius", 4.0),
                                    ("damage".to_owned(), damage(7.0, 0.0)),
                                ],
                            ),
                        )],
                    ),
                ),
            ],
        ),
    )
}

fn side(resource: &str, unit: TestValue) -> TestValue {
    instance(
        "ItemResourceGeneration",
        vec![
            ("resource".to_owned(), TestValue::text(resource)),
            ("resourceAmountPerSecond".to_owned(), unit),
        ],
    )
}

fn signature(nominal: f32, decay: f32) -> TestValue {
    instance(
        "Signature",
        vec![
            single("nominalSignature", nominal),
            single("decayRate", decay),
        ],
    )
}

fn resource_state(name: &str, deltas: Vec<TestValue>) -> TestValue {
    instance(
        "ItemResourceComponentParams",
        vec![(
            "states".to_owned(),
            TestValue::Array(vec![instance(
                "ItemResourceState",
                vec![
                    ("name".to_owned(), TestValue::text(name)),
                    (
                        "signatureParams".to_owned(),
                        instance(
                            "SignatureParams",
                            vec![
                                ("EMSignature".to_owned(), signature(500.0, 0.15)),
                                ("IRSignature".to_owned(), signature(80.0, 0.5)),
                            ],
                        ),
                    ),
                    ("deltas".to_owned(), TestValue::Array(deltas)),
                ],
            )]),
        )],
    )
}

fn plant_resources(units: f32) -> TestValue {
    resource_state(
        "Online",
        vec![instance(
            "ItemResourceDeltaGeneration",
            vec![(
                "generation".to_owned(),
                side(
                    "Power",
                    instance("SPowerSegmentResourceUnit", vec![single("units", units)]),
                ),
            )],
        )],
    )
}

fn cooler_resources(standard: f32) -> TestValue {
    resource_state(
        "Online",
        vec![instance(
            "ItemResourceDeltaConversion",
            vec![
                (
                    "consumption".to_owned(),
                    side(
                        "Power",
                        instance("SPowerSegmentResourceUnit", vec![single("units", 2.0)]),
                    ),
                ),
                (
                    "generation".to_owned(),
                    side(
                        "Coolant",
                        instance(
                            "SStandardResourceUnit",
                            vec![single("standardResourceUnits", standard)],
                        ),
                    ),
                ),
            ],
        )],
    )
}

fn shield() -> TestValue {
    instance(
        "SCItemShieldGeneratorParams",
        vec![
            single("MaxShieldHealth", 8000.0),
            single("MaxShieldRegen", 160.0),
            single("DecayRatio", 0.25),
            single("DownedRegenDelay", 5.0),
            single("DamagedRegenDelay", 2.5),
        ],
    )
}

fn drive() -> TestValue {
    instance(
        "SCItemQuantumDriveParams",
        vec![
            single("quantumFuelRequirement", 0.5),
            single("jumpRange", 5.0e10),
            single("disconnectRange", 20_000.0),
            (
                "params".to_owned(),
                instance(
                    "QuantumDriveParams",
                    vec![
                        single("driveSpeed", 2.0e8),
                        single("cooldownTime", 6.0),
                        single("spoolUpTime", 10.0),
                        single("stageOneAccelRate", 1000.0),
                        single("stageTwoAccelRate", 2000.0),
                        single("engageSpeed", 20.0),
                    ],
                ),
            ),
        ],
    )
}

fn missile() -> TestValue {
    instance(
        "SCItemMissileParams",
        vec![
            (
                "explosionParams".to_owned(),
                instance(
                    "ExplosionParams",
                    vec![
                        single("minRadius", 2.0),
                        single("maxRadius", 12.0),
                        ("damage".to_owned(), damage(1500.0, 0.0)),
                    ],
                ),
            ),
            (
                "targetingParams".to_owned(),
                instance(
                    "MissileTargetingParams",
                    vec![
                        ("trackingSignalType".to_owned(), TestValue::text("Infrared")),
                        single("lockTime", 2.0),
                        single("lockingAngle", 45.0),
                        single("lockRangeMin", 100.0),
                        single("lockRangeMax", 5000.0),
                    ],
                ),
            ),
            (
                "GCSParams".to_owned(),
                instance("MissileGcsParams", vec![single("linearSpeed", 300.0)]),
            ),
        ],
    )
}

fn connections() -> Vec<TestValue> {
    vec![
        instance(
            "EntityComponentPowerConnection",
            vec![
                single("PowerBase", 1.0),
                single("PowerDraw", 4.0),
                single("PowerToEM", 3.0),
                single("DecayRateOfEM", 0.5),
            ],
        ),
        instance(
            "EntityComponentHeatConnection",
            vec![
                single("ThermalEnergyBase", 2.0),
                single("ThermalEnergyDraw", 6.0),
                single("TemperatureToIR", 0.75),
                single("OverheatTemperature", 90.0),
            ],
        ),
        instance("SHealthComponentParams", vec![single("Health", 450.0)]),
    ]
}

fn database() -> DataForgeBuilder {
    let base = structures::builder()
        .record(ammo())
        .record(gun(GUN, "gun", AMMO))
        .record(gun(DANGLING_GUN, "dangling_gun", MISSING_AMMO))
        .record(entity(
            SHIELD,
            "shield",
            "Shield",
            [vec![shield()], connections()].concat(),
        ))
        .record(entity(DRIVE, "drive", "QuantumDrive", vec![drive()]))
        .record(entity(
            PLANT,
            "plant",
            "PowerPlant",
            vec![plant_resources(12.0)],
        ))
        .record(entity(
            COOLER,
            "cooler",
            "Cooler",
            vec![cooler_resources(40.0)],
        ))
        .record(entity(MISSILE, "missile", "Missile", vec![missile()]));
    base.record(entity(PLAIN, "plain", "Misc", connections()))
}

fn english() -> TestEntry {
    TestEntry::zstd(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new().build(),
    )
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(english())
        .write_to(&path)?;
    Ok(path)
}
