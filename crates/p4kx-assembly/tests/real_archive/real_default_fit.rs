// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default items of every real ship and vehicle against the ports that hold them. The data
//! does not keep its own port definitions (ADR-0076), so the test measures and holds the counts
//! per kind of mismatch under ceilings a regression of the check would break.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, Attachment, EntityCatalogue, EntityKind, EntityNode, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

/// The counts of LIVE at the time of ADR-0076 plus a margin of about 8 per cent.
const CEILINGS: [(&str, usize); 4] = [
    ("item_sub_type", 100),
    ("item_type", 1430),
    ("too_large", 165),
    ("too_small", 120),
];

#[derive(Default)]
struct Census {
    checked: usize,
    by_kind: BTreeMap<&'static str, usize>,
}

impl Census {
    fn walk(&mut self, node: &EntityNode) {
        for port in node.ports() {
            if let Attachment::Entity(child) = port.attachment() {
                if port.constraints().is_some() {
                    self.checked += 1;
                    if let Err(reason) = port.accepts(&child.item_facts()) {
                        *self.by_kind.entry(reason.kind()).or_default() += 1;
                    }
                }
                self.walk(child);
            }
            for nested in port.nested() {
                if let Attachment::Entity(child) = nested.attachment() {
                    self.walk(child);
                }
            }
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts of checked and rejected items are the measurement"
)]
fn default_items_stay_within_the_known_mismatches_of_their_ports() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let catalogue = EntityCatalogue::build(&forge, &|| false).ok_or("catalogue stopped")?;
    let mut census = Census::default();
    let mut vehicles = 0_usize;
    for summary in catalogue.entries() {
        if !matches!(summary.kind(), EntityKind::Ship | EntityKind::Vehicle) {
            continue;
        }
        let Ok(record) = resolver.entities().find(summary.class_name()) else {
            continue;
        };
        let Ok(tree) = resolver.resolve(&record) else {
            continue;
        };
        vehicles += 1;
        census.walk(&tree);
    }
    println!(
        "{vehicles} ships and vehicles, {} default items checked, rejected by kind {:?}",
        census.checked, census.by_kind
    );
    assert!(vehicles >= 100, "the fleet was walked");
    assert!(census.checked > 10_000, "default items were checked");
    for kind in census.by_kind.keys() {
        assert!(
            CEILINGS.iter().any(|(known, _)| known == kind),
            "a new kind of mismatch: {kind}"
        );
    }
    for (kind, ceiling) in CEILINGS {
        let count = census.by_kind.get(kind).copied().unwrap_or(0);
        assert!(
            count <= ceiling,
            "{kind}: {count} default items rejected, above the ceiling {ceiling}"
        );
    }
    Ok(())
}
