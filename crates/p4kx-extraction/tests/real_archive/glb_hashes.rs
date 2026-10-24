// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! CRC-32 of the GLB of a sample of real models, to compare two builds of the model reader; runs
//! with `--ignored` when `P4KX_DATA_P4K` names an Archive. The lines are printed
//! (`--nocapture`) and, when `P4KX_HASH_OUT` names a file, written there.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;

use p4kx_extraction::{Archive, Entry, GlbOptions, ModelSource, PathIndex};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: [(&str, usize); 4] = [(".cgf", 600), (".cga", 300), (".skin", 600), (".chr", 100)];

fn line(index: &PathIndex<'_>, entry: &Entry, position: usize, dir: &std::path::Path) -> String {
    let out = dir.join(format!("{position}.glb"));
    let result = ModelSource::from_index(index, entry.path())
        .map_err(|error| format!("{error:?}"))
        .and_then(|source| {
            source
                .write_glb(&GlbOptions::default(), &out)
                .map_err(|error| format!("{error:?}"))
        })
        .and_then(|_| fs::read(&out).map_err(|error| error.to_string()));
    let _ignored = fs::remove_file(&out);
    match result {
        Ok(bytes) => format!(
            "{}\t{:08x}\t{}",
            entry.path(),
            crc32fast::hash(&bytes),
            bytes.len()
        ),
        Err(error) => format!(
            "{}\tERROR\t{}",
            entry.path(),
            error.chars().take(60).collect::<String>()
        ),
    }
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the hashes are the measurement; the test prints them"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_glb_of_a_sample_of_models_is_hashed() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = PathIndex::new(&archive);
    let mut sample: Vec<&Entry> = Vec::new();
    for (extension, count) in SAMPLES {
        let candidates: Vec<&Entry> = archive
            .entries()
            .iter()
            .filter(|entry| entry.path().to_ascii_lowercase().ends_with(extension))
            .collect();
        let stride = (candidates.len() / count).max(1);
        sample.extend(candidates.into_iter().step_by(stride).take(count));
    }
    let dir = tempfile::tempdir()?;
    let mut lines: Vec<String> = sample
        .par_iter()
        .enumerate()
        .map(|(position, entry)| line(&index, entry, position, dir.path()))
        .collect();
    lines.sort();
    let mut text = String::new();
    for line in &lines {
        let _written = writeln!(text, "{line}");
    }
    println!("{text}");
    if let Some(out) = std::env::var_os("P4KX_HASH_OUT") {
        fs::write(out, &text)?;
    }
    assert!(!lines.is_empty(), "the Archive holds models");
    Ok(())
}
