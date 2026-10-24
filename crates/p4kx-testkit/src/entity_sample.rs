// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{DataForgeBuilder, TestEntity, TestLoadout, TestLoadoutEntry, TestPort, entity_schema};

/// The GUID of the sample ship [`sample_ship`] holds.
pub const SAMPLE_SHIP: u128 = 0x0079_c5d5_1678_4f8c_85ba_18ca_8f64_2af6;

/// The GUID of the gimbal mount at the sample ship's nose.
pub const SAMPLE_MOUNT: u128 = 0xea49_d194_0c0b_43e1_82fb_d72f_35c1_af05;

/// The GUID of the gun the sample ship's Loadout puts into the mount.
pub const SAMPLE_GUN: u128 = 0xdf89_e9ca_8a11_477c_8abd_f483_7a27_7dbe;

/// The GUID the sample ship's Loadout refers to although no Record has it.
pub const SAMPLE_MISSING: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

/// Returns a synthetic DataForge with a ship Entity whose default Loadout nests.
///
/// The ship holds a seat and a dashboard by class name, the dashboard with its own Loadout of
/// two screens of which the ship overrides one, a gimbal mount by Reference with the gun the
/// ship's Loadout gives it, an empty paint port, a defined port without entry, a dangling
/// Reference and an unknown class name.
#[must_use]
pub fn sample_ship() -> DataForgeBuilder {
    std::iter::once(ship())
        .chain(parts())
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
}

fn ship() -> TestEntity {
    TestEntity::new(
        SAMPLE_SHIP,
        "TEST_Ship",
        "libs/foundry/records/entities/spaceships/test_ship.xml",
    )
    .port(
        TestPort::new("hardpoint_weapon_nose")
            .sizes(2, 3)
            .item_type("WeaponGun", ["Gun"])
            .helper("hp_nose", [0.0, 4.5, -0.25], [0.0, 0.0, 90.0]),
    )
    .port(TestPort::new("hardpoint_relay").sizes(0, 1))
    .loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::empty("hardpoint_paint"),
        TestLoadoutEntry::class("hardpoint_seat_pilot", "TEST_Seat"),
        TestLoadoutEntry::class("hardpoint_dashboard", "TEST_Dashboard").loadout(
            TestLoadout::Manual(vec![TestLoadoutEntry::class("Radar", "TEST_Radar_Screen")]),
        ),
        TestLoadoutEntry::reference("hardpoint_weapon_nose", SAMPLE_MOUNT).loadout(
            TestLoadout::Manual(vec![TestLoadoutEntry::reference(
                "hardpoint_class_2",
                SAMPLE_GUN,
            )]),
        ),
        TestLoadoutEntry::reference("hardpoint_cargo", SAMPLE_MISSING),
        TestLoadoutEntry::class("hardpoint_door", "TEST_No_Such_Door"),
    ]))
}

fn parts() -> [TestEntity; 6] {
    let dashboard = TestEntity::new(
        0x5f12_dd31_4ff9_41a5_b5e2_9162_7035_1ba9,
        "TEST_Dashboard",
        "libs/foundry/records/entities/scitem/ships/test_dashboard.xml",
    )
    .loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::class("Display_HUD", "TEST_Screen"),
        TestLoadoutEntry::class("Radar", "TEST_Screen"),
    ]));
    let mount = TestEntity::new(
        SAMPLE_MOUNT,
        "TEST_Mount_Gimbal_S3",
        "libs/foundry/records/entities/scitem/ships/weapon_mounts/test_mount_gimbal_s3.xml",
    )
    .port(
        TestPort::new("hardpoint_class_2")
            .sizes(3, 3)
            .item_type("WeaponGun", ["Gun"])
            .helper("hardpoint_class_2", [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
    );
    [
        dashboard,
        mount,
        TestEntity::new(
            SAMPLE_GUN,
            "TEST_Gun_S3",
            "libs/foundry/records/entities/scitem/ships/weapons/test_gun_s3.xml",
        ),
        TestEntity::new(
            0x95b8_cab3_5230_46f9_92e6_8e02_1978_f543,
            "TEST_Seat",
            "libs/foundry/records/entities/scitem/ships/test_seat.xml",
        ),
        TestEntity::new(
            0x6374_bb62_de4e_410a_96a3_a115_6dce_4802,
            "TEST_Screen",
            "libs/foundry/records/entities/scitem/ships/test_screen.xml",
        ),
        TestEntity::new(
            0x5034_43c0_b6d1_4539_b493_6881_1733_4ff1,
            "TEST_Radar_Screen",
            "libs/foundry/records/entities/scitem/ships/test_radar_screen.xml",
        ),
    ]
}
