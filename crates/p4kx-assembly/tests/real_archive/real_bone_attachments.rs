// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The thruster housings of two ships of a real Archive are drawn at their bones.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, IssueKind, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 2] = ["ORIG_M50", "ANVL_Gladiator"];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_housing_of_a_main_thruster_is_drawn_at_its_bone() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    for name in SHIPS {
        let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
        let record = resolver.entities().find(name)?;
        let tree = resolver.resolve(&record)?;

        let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default())?;

        let housings: Vec<_> = assembly
            .parts()
            .iter()
            .flat_map(p4kx_assembly::Part::attachments)
            .filter(|attached| attached.name() == "Housing")
            .collect();
        assert_eq!(housings.len(), 2, "{name}: the left and the right thruster");
        assert!(
            housings.iter().all(|housing| housing.is_bone_found()),
            "{name}: the bone of each housing is in the skeleton"
        );
        assert!(
            assembly.issues().iter().all(|issue| !matches!(
                issue.kind(),
                IssueKind::AttachmentsLeftOut { .. } | IssueKind::HelperNotFound { .. }
            )),
            "{name}: {:?}",
            assembly.issues()
        );
    }
    Ok(())
}
