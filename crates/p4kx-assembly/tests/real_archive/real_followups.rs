// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Character definitions, vehicle-only ports and helper issues over ships of a real Archive.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, IssueKind, LoadoutResolver, PortSource,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 10] = [
    "AEGS_Gladius",
    "AEGS_Avenger_Titan",
    "ANVL_Hornet_F7C",
    "RSI_Constellation_Andromeda",
    "TMBL_Cyclone",
    "ORIG_M50",
    "ORIG_350r",
    "ANVL_Gladiator",
    "DRAK_Cutter",
    "KRIG_P72_Archimedes",
];

#[derive(Default)]
struct Census {
    ships: usize,
    cdf_parts: usize,
    cdf_modelled: usize,
    cdf_unreadable: usize,
    attachment_notes: usize,
    empty_ports: usize,
    helper_without_parent_model: usize,
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the census of character definitions and empty ports is the measurement"
)]
fn character_definitions_and_vehicle_only_ports_of_documented_ships() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut census = Census::default();
    for name in SHIPS {
        let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
        let Ok(record) = resolver.entities().find(name) else {
            println!("{name}: no such Entity");
            continue;
        };
        let tree = resolver.resolve(&record)?;
        let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;
        let before = census.cdf_parts;
        tally(&tree, &assembly, &mut census);
        println!(
            "{name}: {} parts, {} with models, {} character definitions",
            assembly.parts().len(),
            assembly.modelled_parts(),
            census.cdf_parts - before
        );
    }
    println!(
        "{} ships: {} cdf parts, {} modelled, {} unreadable, {} attachment notes, {} empty ports, {} helper issues under a part without model",
        census.ships,
        census.cdf_parts,
        census.cdf_modelled,
        census.cdf_unreadable,
        census.attachment_notes,
        census.empty_ports,
        census.helper_without_parent_model
    );
    assert!(census.ships >= 5, "{} ships found", census.ships);
    assert_eq!(census.cdf_unreadable, 0, "character definitions are read");
    assert!(census.cdf_parts > 0, "some ship has a character definition");
    assert_eq!(census.cdf_modelled, census.cdf_parts);
    assert_eq!(
        census.helper_without_parent_model, 0,
        "no helper issue repeats a missing parent model"
    );
    Ok(())
}

fn tally(tree: &p4kx_assembly::EntityNode, assembly: &Assembly, census: &mut Census) {
    census.ships += 1;
    census.empty_ports += tree
        .walk()
        .iter()
        .filter(|(_, port)| port.source() == PortSource::Empty)
        .count();
    for part in assembly.parts() {
        if part
            .geometry()
            .is_some_and(|geometry| geometry.to_ascii_lowercase().ends_with(".cdf"))
        {
            census.cdf_parts += 1;
            census.cdf_modelled += usize::from(part.model().is_some());
        }
    }
    for issue in assembly.issues() {
        match issue.kind() {
            IssueKind::GeometryUnreadable { path, .. }
                if path.to_ascii_lowercase().ends_with(".cdf") =>
            {
                census.cdf_unreadable += 1;
            }
            IssueKind::AttachmentsLeftOut { .. } => census.attachment_notes += 1,
            IssueKind::HelperNotFound { .. } => {
                let parent = assembly
                    .part_at(issue.path())
                    .and_then(p4kx_assembly::Part::parent)
                    .and_then(|index| assembly.parts().get(index));
                census.helper_without_parent_model +=
                    usize::from(parent.is_some_and(|part| part.model().is_none()));
            }
            _ => {}
        }
    }
}
