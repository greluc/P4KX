//! The nested field diff over two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them.

use std::error::Error;

use p4kx_comparison::{
    Archive, CompareOptions, DataForgeEntry, DataForges, Levels, RecordChange, Step, compare,
};
use p4kx_testkit::{real_archive_b_path, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_nested_field_diff_of_two_real_archives_is_deterministic_and_well_formed() -> TestResult {
    let (old, new) = (real_archive_path()?, real_archive_b_path()?);
    let (old, new) = rayon::join(|| Archive::open(old), || Archive::open(new));
    let (old, new) = (old?, new?);
    let (old_entry, new_entry) =
        rayon::join(|| DataForgeEntry::read(&old), || DataForgeEntry::read(&new));
    let (old_entry, new_entry) = (old_entry?, new_entry?);
    let (old_forge, new_forge) = (old_entry.open()?, new_entry.open()?);
    let forges = DataForges {
        old: &old_forge,
        new: &new_forge,
    };
    let options = CompareOptions {
        levels: Levels::RECORDS,
        ..CompareOptions::default()
    };

    let first = compare(&old, &new, Some(forges), &options)?;
    let second = compare(&old, &new, Some(forges), &options)?;

    assert!(
        serde_json::to_string(&first.report())? == serde_json::to_string(&second.report())?,
        "two runs give the same report"
    );
    let fields: Vec<_> = first
        .record_changes()
        .iter()
        .flat_map(RecordChange::fields)
        .collect();
    let malformed = fields.iter().find(|field| {
        let steps = field.path().steps();
        !matches!(steps.first(), Some(Step::Field(_)))
            || (field.old_value().is_none() && field.new_value().is_none())
    });
    assert!(
        malformed.is_none(),
        "every change starts at a field and has a side: {malformed:?}"
    );
    let nested = fields
        .iter()
        .filter(|field| field.path().steps().len() > 1)
        .count();
    assert!(
        first.record_counts().changed == 0 || nested > 0,
        "changed Records of two Builds hold nested changes: {} of {} field changes",
        nested,
        fields.len()
    );
    Ok(())
}
