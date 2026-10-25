// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layered clothing of Miles Eckhart: its materials have layers with diffuse maps, so none is
//! reported as drawn in its layer colours.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry, draws_layer_diffuse};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const NPC: &str = "NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_layered_clothing_of_miles_eckhart_has_layers_with_diffuse_maps() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(NPC)?;
    let tree = resolver.resolve(&record)?;
    let head = resolver.npc_head(&tree).ok_or("no head")?;
    let mut pieces = resolver.worn(&tree);
    pieces.extend(head.pieces().iter().cloned());
    let options = AssemblyOptions::default().with_lod(0);

    let armour = Armour::build(head.variant(), &pieces, &models, &options)?;

    let drawn = armour
        .parts()
        .iter()
        .filter_map(|part| part.materials().library())
        .flat_map(|library| &library.materials)
        .filter(|material| draws_layer_diffuse(material))
        .count();
    assert!(drawn > 0, "the clothing has layered materials with maps");
    assert!(
        armour.layer_issues().is_empty(),
        "{:?}",
        armour.layer_issues()
    );
    Ok(())
}
