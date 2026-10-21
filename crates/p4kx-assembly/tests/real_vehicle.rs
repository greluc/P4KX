//! Vehicle implementations and their Modifications in a real Archive.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ENTITY_TYPE, EntityNode, LoadoutResolver, PortVisibility, VehicleImplementation,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SPACESHIPS: &str = "libs/foundry/records/entities/spaceships/";

#[derive(Debug, Default)]
struct Tally {
    with_implementation: usize,
    with_modification: usize,
    changed: Vec<String>,
}

fn ports_changed(
    files: &ArchiveFiles<'_>,
    tree: &EntityNode,
    modified: &VehicleImplementation,
) -> Result<usize, Box<dyn Error>> {
    let base = VehicleImplementation::read(files, tree.class_name(), modified.path(), None)?;
    Ok(base
        .item_ports()
        .filter(|part| {
            let now = modified.port_visibility(part.name());
            now != PortVisibility::Unspecified && now != base.port_visibility(part.name())
        })
        .count())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_modification_changes_the_port_visibility_of_a_real_ship() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let ships: Vec<_> = forge
        .records()
        .filter(|record| {
            record
                .record_path()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .starts_with(SPACESHIPS)
                && record.type_name().is_ok_and(|name| name == ENTITY_TYPE)
        })
        .collect();
    assert!(!ships.is_empty(), "no spaceship Entities");
    let mut tally = Tally::default();
    for ship in ships {
        let tree = resolver.resolve(&ship)?;
        let Some(modified) = tree.vehicle() else {
            continue;
        };
        tally.with_implementation += 1;
        let Some(modification) = modified.modification() else {
            continue;
        };
        tally.with_modification += 1;
        let differing = ports_changed(&files, &tree, modified)?;
        if differing > 0 {
            tally.changed.push(format!(
                "{} ({modification}): {differing} ports",
                tree.class_name()
            ));
        }
    }
    assert!(tally.with_implementation > 50, "{tally:?}");
    assert!(tally.with_modification > 0, "{tally:?}");
    assert!(
        !tally.changed.is_empty(),
        "no Modification changes the visibility of a port: {tally:?}"
    );
    Ok(())
}
