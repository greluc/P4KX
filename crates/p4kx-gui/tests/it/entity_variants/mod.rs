// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the variant tests of the Entities tab: a synthetic Archive whose rifle has two
//! colour variants, beside a pistol, a ship and a second family of variants.

use egui_kittest::Harness;
use p4kx_gui::{EntitiesStatus, EntityRole, Language, Model};
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestEntry, TestPort, assembly_entries, entity_schema,
};

use crate::entity_picking::opened_sized;
use crate::waiting::wait_for_model;

type Window = (tempfile::TempDir, Harness<'static, Model>);

/// Opens the synthetic Archive in a window of `size` on the Entities tab and waits until the
/// Entities are listed and their variants folded.
pub(crate) fn opened(
    language: Language,
    size: [f32; 2],
) -> Result<Window, Box<dyn std::error::Error>> {
    let (dir, mut harness) = opened_sized(entries(), language, size)?;
    wait_for_model(&mut harness, "the variants to be grouped", |model| {
        !model.entities().is_grouping()
    })?;
    Ok((dir, harness))
}

const FOLDER: &str = "libs/foundry/records/entities";

fn gun(guid: u128, class: &str, size: i32, model: &str, material: &str) -> TestEntity {
    TestEntity::new(
        guid,
        class,
        format!("{FOLDER}/scitem/ships/weapons/{}.xml", class.to_lowercase()),
    )
    .attach_type("WeaponGun")
    .sub_type("Gun")
    .size(size)
    .geometry(model, Some(material))
}

fn entities() -> Vec<TestEntity> {
    vec![
        TestEntity::new(
            0x1,
            "ACME_Ship",
            format!("{FOLDER}/spaceships/acme_ship.xml"),
        )
        .geometry("Objects/Test/ship.cgf", None)
        .port(TestPort::new("hardpoint_relay").helper(
            "hardpoint_relay",
            [0.0; 3],
            [0.0; 3],
        )),
        gun(0x10, "ACME_Rifle", 2, "Objects/Test/rifle.cgf", "base.mtl"),
        gun(
            0x11,
            "ACME_Rifle_Blue",
            2,
            "Objects/Test/rifle.cgf",
            "blue.mtl",
        ),
        gun(
            0x12,
            "ACME_Rifle_Red",
            2,
            "Objects/Test/rifle.cgf",
            "red.mtl",
        ),
        gun(
            0x13,
            "ACME_Pistol",
            1,
            "Objects/Test/pistol.cgf",
            "base.mtl",
        ),
        gun(
            0x14,
            "ACME_Cannon",
            4,
            "Objects/Test/cannon.cgf",
            "base.mtl",
        ),
        gun(0x15, "ACME_Dart", 4, "Objects/Test/cannon.cgf", "dark.mtl"),
    ]
}

/// Returns the Entries of the synthetic Archive.
pub(crate) fn entries() -> Vec<TestEntry> {
    let database = entities()
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
        .build();
    let mut entries = vec![TestEntry::zstd("Data\\Game2.dcb", database)];
    entries.extend(assembly_entries().into_iter().skip(1));
    entries
}

/// Returns the class name and the place in its group of every row of the picker.
pub(crate) fn rows(model: &Model) -> Vec<(String, EntityRole)> {
    match model.entities().status() {
        EntitiesStatus::Ready(list) => (0..list.shown_len())
            .filter_map(|at| list.row(at))
            .map(|row| (row.entity().class_name().to_owned(), row.role()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns the base of a group of `variants`, folded.
pub(crate) const fn folded(variants: usize) -> EntityRole {
    EntityRole::Base {
        variants,
        expanded: false,
    }
}

/// Returns the base of a group of `variants`, with its variants listed.
pub(crate) const fn open(variants: usize) -> EntityRole {
    EntityRole::Base {
        variants,
        expanded: true,
    }
}
