// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The TXTP files of a real trigger name only files the export wrote.

use std::error::Error;
use std::fs;

use p4kx_assembly::audio::{ArchiveAudio, ExportOptions, SoundFormat, write_txtp};
use p4kx_extraction::{Archive, ConflictPolicy};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const TRIGGER: &str = "Play_QDrive_Spool";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn the_txtp_of_a_real_trigger_names_exported_files() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = audio
        .resolve_trigger(TRIGGER)
        .ok_or("the trigger resolves")?;
    let out = tempfile::tempdir()?;
    let options = ExportOptions {
        format: SoundFormat::Wav,
        policy: ConflictPolicy::Overwrite,
    };
    let triggers = [&sounds];
    let exported = audio.export_sounds(&triggers, out.path(), options)?;

    let report = write_txtp(&triggers, &exported, out.path(), ConflictPolicy::Overwrite)?;

    println!(
        "{} variants, {} files, {} TXTP written, {} skipped: {:?}",
        sounds.variants().len(),
        exported.sounds.len(),
        report.files.len(),
        report.skipped.len(),
        report.skipped
    );
    assert!(!report.files.is_empty(), "{:?}", report.skipped);
    for file in &report.files {
        for line in fs::read_to_string(&file.path)?.lines() {
            let line = line.trim_start();
            if line.starts_with('#') || line.starts_with("group =") {
                continue;
            }
            let name = line.split(" #").next().unwrap_or(line);
            assert!(out.path().join(name).is_file(), "{name} is written");
        }
    }
    Ok(())
}
