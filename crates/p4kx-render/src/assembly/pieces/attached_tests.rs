// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    ArchiveBuilder, BONE_THRUSTER, bone_thruster_definition, bone_thruster_entries,
    bone_thruster_housing,
};

use super::pieces_of;

#[test]
fn a_bone_attachment_is_a_piece_below_its_part_at_the_part_world_times_its_local() {
    let dir = tempfile::tempdir().expect("a folder");
    let path = dir.path().join("Data.p4k");
    let definition =
        bone_thruster_definition(&bone_thruster_housing("tip", r"Objects\Test\housing.cgf"));
    bone_thruster_entries(&definition)
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)
        .expect("the Archive is written");
    let archive = Archive::open(&path).expect("the Archive opens");
    let entry = DataForgeEntry::read(&archive).expect("a DataForge");
    let forge = entry.open().expect("it opens");
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(BONE_THRUSTER).expect("the Entity");
    let tree = resolver.resolve(&record).expect("its Loadout");
    let models = ArchiveModels::new(&archive);
    let assembly =
        Assembly::build(&tree, &models, &AssemblyOptions::default()).expect("the Assembly");

    let pieces = pieces_of(&assembly);

    let [skin, housing] = pieces.as_slice() else {
        panic!("a skin and a housing: {}", pieces.len());
    };
    assert_eq!(skin.name, BONE_THRUSTER);
    assert_eq!(housing.name, format!("{BONE_THRUSTER}/Housing"));
    assert_eq!(housing.path, "Housing");
    assert_eq!(housing.parent, Some(0));
    assert!(housing.model.is_some() && !housing.interior);
    let part = assembly.root().expect("the part");
    let attached = part.attachments().first().expect("the attachment");
    assert_eq!(housing.world, part.world().then(attached.local()).rows);
}
