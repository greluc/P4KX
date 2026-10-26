// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `IncludedObjects` tables of every `.soc` file in the socpaks follow the record layouts
//! that are read.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_extraction::Archive;
use p4kx_model::{IncludedObjects, IncludedObjectsError};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default, Debug)]
struct Survey {
    files: usize,
    tables: usize,
    errors: Vec<String>,
    objects: usize,
    skipped: usize,
    invalid: usize,
    unsupported: BTreeMap<u32, usize>,
}

impl Survey {
    fn add(&mut self, name: &str, read: Result<Option<IncludedObjects>, IncludedObjectsError>) {
        self.files += 1;
        match read {
            Ok(Some(table)) => {
                self.tables += 1;
                self.objects += table.objects.len();
                self.skipped += table.skipped;
                self.invalid += table.invalid;
                if let Some(kind) = table.unsupported {
                    *self.unsupported.entry(kind).or_default() += 1;
                }
            }
            Ok(None) => {}
            Err(error) => self.errors.push(format!("{name}: {error}")),
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the PR body")]
fn every_included_objects_table_of_the_archive_is_read() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let survey = Mutex::new(Survey::default());
    let socpaks: Vec<_> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".socpak"))
        .collect();
    socpaks.par_iter().for_each(|entry| {
        let Ok(root) = archive.open_container(entry) else {
            return;
        };
        for inner in root.entries() {
            if !inner.path().to_ascii_lowercase().ends_with(".soc") {
                continue;
            }
            let mut bytes = Vec::new();
            let Ok(mut reader) = root.open_entry(inner) else {
                continue;
            };
            if reader.read_to_end(&mut bytes).is_err() {
                continue;
            }
            let read = IncludedObjects::read(&bytes);
            if let Ok(mut survey) = survey.lock() {
                survey.add(&format!("{} {}", entry.path(), inner.path()), read);
            }
        }
    });
    let survey = survey.into_inner()?;
    println!("{survey:#?}");
    assert!(survey.tables > 5000, "{} tables", survey.tables);
    assert!(survey.errors.is_empty(), "{:?}", survey.errors);
    assert!(survey.skipped > 0, "records of kind 16 are skipped");
    assert!(
        survey.unsupported.values().sum::<usize>() * 100 < survey.tables,
        "{:?} of {} tables end at a kind of unknown length",
        survey.unsupported,
        survey.tables
    );
    assert!(
        survey.invalid * 1000 < survey.objects,
        "{} of {} objects are invalid",
        survey.invalid,
        survey.objects
    );
    Ok(())
}
