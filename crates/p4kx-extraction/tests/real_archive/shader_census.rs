// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The shaders of the materials of a real Archive and which of them read the vertex colours; runs
//! with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;

use p4kx_extraction::model::MaterialLibrary;
use p4kx_extraction::{Archive, Entry};
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

/// Counts of one shader: materials, those with `PER_VERTEX_TINT`, those that read the colours.
#[derive(Default, Clone, Copy)]
struct Count {
    materials: usize,
    tinted: usize,
    reading: usize,
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the census table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_glass_shaders_and_the_materials_that_read_the_vertex_colours() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let mtl: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".mtl"))
        .collect();

    let counts = mtl
        .par_iter()
        .fold(BTreeMap::<String, Count>::new, |mut counts, entry| {
            let mut bytes = Vec::new();
            let read = archive
                .open_entry(entry)
                .ok()
                .and_then(|mut reader| reader.read_to_end(&mut bytes).ok());
            if let (Some(_), Ok(library)) = (read, MaterialLibrary::parse(entry.path(), &bytes)) {
                for material in &library.materials {
                    let count = counts
                        .entry(material.shader.to_ascii_lowercase())
                        .or_default();
                    count.materials += 1;
                    count.tinted +=
                        usize::from(material.string_gen_mask.contains("PER_VERTEX_TINT"));
                    count.reading += usize::from(material.reads_vertex_colours());
                }
            }
            counts
        })
        .reduce(BTreeMap::new, |mut all, part| {
            for (shader, count) in part {
                let total: &mut Count = all.entry(shader).or_default();
                total.materials += count.materials;
                total.tinted += count.tinted;
                total.reading += count.reading;
            }
            all
        });

    for (shader, count) in &counts {
        println!(
            "{shader:>24}: {} materials, {} PER_VERTEX_TINT, {} read the vertex colours",
            count.materials, count.tinted, count.reading
        );
    }
    let glass = counts.get("glasspbr").copied().unwrap_or_default();
    assert!(glass.materials > 0, "the Archive has GlassPBR materials");
    assert_eq!(
        glass.tinted, glass.reading,
        "tinted glass reads the colours"
    );
    assert!(
        !counts.contains_key("glass"),
        "no material has the shader Glass"
    );
    assert!(
        counts.values().map(|count| count.reading).sum::<usize>() > 0,
        "some materials read the vertex colours"
    );
    Ok(())
}
