// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Value comparison of the `DataForge` Records `StarBreaker` wrote with the Records P4KX builds.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use p4kx_extraction::{Archive, DataForge, DataForgeEntry, TreeBuilder};
use p4kx_testkit::{real_archive_key, real_archive_path, reference_starbreaker_path};
use rayon::prelude::*;

use super::TestResult;
use super::starbreaker_json::record;
use super::tally::{Compared, Tally};
use super::value_tree::RecordDoc;
use super::value_tree_compare::compare_counting;
use super::walk::files_under;

pub(crate) const MAIN_RECORDS_ONLY: &str = "StarBreaker exports only the main Record of a file";

/// The Record P4KX builds for a GUID, in the value model.
fn record_doc<'f>(
    forge: &'f DataForge<'f>,
    builder: &mut TreeBuilder<'f>,
    guid: &str,
) -> Option<RecordDoc> {
    let found = forge.record_by_guid(guid.parse().ok()?)?;
    let tree = builder.build(&found).ok()?;
    Some(RecordDoc::from_tree(&tree))
}

fn compare_file<'f>(
    forge: &'f DataForge<'f>,
    builder: &mut TreeBuilder<'f>,
    file: &Path,
) -> (Compared, u64) {
    let different = |difference: String| (Compared::Different(difference), 0);
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => return different(format!("reference: {error}")),
    };
    let theirs = match record(&text) {
        Ok(theirs) => theirs,
        Err(error) => return different(format!("reference: {error}")),
    };
    let Ok(guid) = theirs.guid.parse() else {
        return different(format!("reference GUID {:?} is no GUID", theirs.guid));
    };
    let Some(found) = forge.record_by_guid(guid) else {
        return different(format!("no Record {} in the Archive", theirs.guid));
    };
    let ours = match builder.build(&found) {
        Ok(tree) => RecordDoc::from_tree(&tree),
        Err(error) => return different(format!("P4KX: {error}")),
    };
    compare_counting(&ours, &theirs, &mut |guid| record_doc(forge, builder, guid))
}

fn run<'f>(forge: &'f DataForge<'f>, root: &Path) -> Result<Tally, std::io::Error> {
    let paths = files_under(root)?;
    let files = AtomicU64::new(0);
    let mut tally = paths
        .par_iter()
        .filter(|file| {
            file.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map_init(TreeBuilder::new, |builder, file| {
            let mut tally = Tally::default();
            let (compared, embedded) = compare_file(forge, builder, file);
            files.fetch_add(1, Ordering::Relaxed);
            tally.record(&file.to_string_lossy(), compared);
            tally.embedded_records = embedded;
            tally
        })
        .reduce(Tally::default, Tally::merge);
    let unmatched = (forge.record_count() as u64).saturating_sub(files.into_inner());
    tally.missing_in_reference = unmatched;
    Ok(tally)
}

fn export_all<'f>(forge: &'f DataForge<'f>) -> (u64, u64) {
    let built = AtomicU64::new(0);
    let failed = AtomicU64::new(0);
    (0..forge.record_count())
        .into_par_iter()
        .for_each_init(TreeBuilder::new, |builder, index| {
            let result = u32::try_from(index)
                .ok()
                .and_then(|index| forge.record(index))
                .map(|found| builder.build(&found).is_ok());
            match result {
                Some(true) => built.fetch_add(1, Ordering::Relaxed),
                _ => failed.fetch_add(1, Ordering::Relaxed),
            };
        });
    (built.into_inner(), failed.into_inner())
}

#[test]
#[ignore = "real Archive and reference output: set P4KX_DATA_P4K and P4KX_REF_STARBREAKER and pass --ignored"]
fn every_record_starbreaker_wrote_has_the_same_values() -> TestResult {
    let Some(root) = reference_starbreaker_path() else {
        super::report("skipped: P4KX_REF_STARBREAKER is not set");
        return Ok(());
    };
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let started = Instant::now();
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    super::report(&format!(
        "opened {} ({} Records) in {:?}",
        entry.path(),
        forge.record_count(),
        started.elapsed()
    ));

    let started = Instant::now();
    let tally = run(&forge, &root)?;
    super::report(&format!(
        "StarBreaker comparison took {:?}",
        started.elapsed()
    ));
    if tally.missing_in_reference > 0 {
        super::report(&format!(
            "{} Records have no file in the reference ({MAIN_RECORDS_ONLY} unless --all)",
            tally.missing_in_reference
        ));
    }

    let started = Instant::now();
    let (built, failed) = export_all(&forge);
    super::report(&format!(
        "built the trees of all Records: {built} built, {failed} failed, in {:?}",
        started.elapsed()
    ));
    tally.finish("StarBreaker value comparison")?;
    if failed > 0 {
        return Err(format!("{failed} Records did not build").into());
    }
    Ok(())
}
