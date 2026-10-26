// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default items of real ships fit the sub-types of the ports that hold them.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, Attachment, Incompatibility, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 10] = [
    "AEGS_Gladius",
    "AEGS_Sabre",
    "ANVL_Hornet_F7A_Mk1",
    "ANVL_Carrack",
    "DRAK_Cutlass_Black",
    "MISC_Freelancer",
    "ORIG_325a",
    "RSI_Constellation_Andromeda",
    "RSI_Polaris",
    "AEGS_Idris_P",
];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts of checked and rejected items are the measurement"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the ships that counts and reports"
)]
fn default_items_fit_the_sub_types_of_their_ports() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let (mut ships, mut checked, mut with_sub_types) = (0_usize, 0_usize, 0_usize);
    let mut sub_type_rejections = Vec::new();
    let mut other_rejections = 0_usize;
    for name in SHIPS {
        let Ok(record) = resolver.entities().find(name) else {
            continue;
        };
        let tree = resolver.resolve(&record)?;
        ships += 1;
        for (path, port) in tree.walk() {
            let Attachment::Entity(child) = port.attachment() else {
                continue;
            };
            let Some(constraints) = port.constraints() else {
                continue;
            };
            let Ok(item) = resolver.entities().find(&child.guid().to_string()) else {
                continue;
            };
            let facts = resolver.item_facts(&item)?;
            checked += 1;
            if constraints
                .types()
                .iter()
                .any(|accepted| !accepted.sub_types().is_empty())
            {
                with_sub_types += 1;
            }
            match port.accepts(&facts) {
                Ok(()) => {}
                Err(reason @ Incompatibility::SubType { .. }) => {
                    sub_type_rejections
                        .push(format!("{name}: {path} = {}: {reason}", child.class_name()));
                }
                Err(reason) => {
                    println!("other: {name}: {path} = {}: {reason}", child.class_name());
                    other_rejections += 1;
                }
            }
        }
    }
    println!(
        "{ships} ships, {checked} default items checked, {with_sub_types} on ports with \
         sub-types, {} sub-type rejections, {other_rejections} other rejections",
        sub_type_rejections.len()
    );
    for line in &sub_type_rejections {
        println!("{line}");
    }
    assert!(ships >= 3, "at least three of the ships exist");
    assert!(checked > 0, "default items were checked");
    assert!(
        sub_type_rejections.is_empty(),
        "default items rejected by sub-type: {sub_type_rejections:#?}"
    );
    Ok(())
}
