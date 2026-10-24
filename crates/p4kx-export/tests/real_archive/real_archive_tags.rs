// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tag tree of a real Archive, reached through `p4kx-export`.

use std::error::Error;
use std::fs;

use p4kx_export::{
    Guid, NDJSON_FILE, RecordExportOptions, RecordForm, TagResolver, TagTree, write_record_export,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIP_GUID: &str = "0079c5d5-1678-4f8c-85ba-18ca8f642af6";
const SHIP_TAG: &str = "5f12dd31-4ff9-41a5-b5e2-916270351ba9";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_tag_tree_of_a_real_archive_resolves_the_tags_of_a_known_ship() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let tags = TagTree::build(&forge);

    assert!(
        tags.len() > 10_000,
        "the tag tree holds {} tags",
        tags.len()
    );
    assert!(
        tags.roots().count() > 10,
        "the TagDatabase lists many roots"
    );
    let ship = tags.get(SHIP_TAG.parse()?).ok_or("no ship tag")?;
    assert_eq!(ship.path(), "Missions.VehicleType.Ship", "full tag path");
    assert_eq!(ship.name(), "Ship", "the name alone");
    assert_eq!(
        ship.parent().map(|parent| parent.path()),
        Some("Missions.VehicleType"),
        "parent"
    );
    assert!(
        ship.children()
            .any(|child| child.path() == "Missions.VehicleType.Ship.CombatShip"),
        "children"
    );
    assert_eq!(
        tags.tag_path(SHIP_TAG.parse::<Guid>()?),
        Some("Missions.VehicleType.Ship"),
        "the resolver of the Record export"
    );

    let record = forge
        .record_by_guid(SHIP_GUID.parse()?)
        .ok_or("no known ship")?;
    let dir = tempfile::tempdir()?;
    let options = RecordExportOptions {
        tags: Some(&tags),
        ..RecordExportOptions::default()
    };
    let summary = write_record_export(&forge, &[record], RecordForm::Ndjson, dir.path(), &options);
    assert!(summary.is_complete(), "{:?}", summary.failures().first());
    let line = fs::read_to_string(dir.path().join(NDJSON_FILE))?;
    for path in ["Ship.Series.Avenger", "Ship.Model.Avenger_Titan"] {
        assert!(
            line.contains(&format!("\"tag\":\"{path}\"")),
            "the Record carries the tag {path}"
        );
    }
    Ok(())
}
