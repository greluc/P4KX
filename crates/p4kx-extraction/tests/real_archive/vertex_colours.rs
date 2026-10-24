// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The byte order of the vertex colours, checked against the colour words in the file names of
//! the models of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_extraction::model::LodSelection;
use p4kx_extraction::{Archive, Entry, ModelSource, PathIndex};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SUFFIXES: [&str; 3] = [".cgf", ".cga", ".skin"];
const RED_WORDS: [&str; 3] = ["red", "orange", "yellow"];
const BLUE_WORDS: [&str; 1] = ["blue"];
const TINTED_DIFFERENCE: u8 = 64;
const MIN_TINTED_SHARE: f64 = 0.25;
const DOMINANCE: u64 = 2;
const MIN_FILES: u64 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tint {
    Red,
    Blue,
}

#[derive(Default)]
struct Counts {
    red: u64,
    blue: u64,
}

fn tint_of(path: &str, index: &PathIndex<'_>) -> Option<Tint> {
    let model = ModelSource::from_index_with(index, path, LodSelection::Only(0)).ok()?;
    let (mut red, mut blue, mut vertices) = (0_u64, 0_u64, 0_u64);
    for mesh in model.model().lods.iter().flat_map(|lod| &lod.meshes) {
        vertices += mesh.positions.len() as u64;
        for &[r, _, b, _] in mesh.colours.iter().flatten() {
            if r.abs_diff(b) > TINTED_DIFFERENCE {
                if r > b {
                    red += 1;
                } else {
                    blue += 1;
                }
            }
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
    let tinted = (red + blue) as f64 / vertices.max(1) as f64;
    (tinted > MIN_TINTED_SHARE).then_some(if red > blue { Tint::Red } else { Tint::Blue })
}

fn words_of(path: &str) -> Vec<&'static str> {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let tokens: Vec<&str> = name.split(|c: char| !c.is_ascii_alphabetic()).collect();
    RED_WORDS
        .iter()
        .chain(&BLUE_WORDS)
        .copied()
        .filter(|word| tokens.contains(word))
        .collect()
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the counts per colour word are the finding; the test prints them"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn models_named_after_a_colour_have_it_under_the_bgra_order() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let index = PathIndex::new(&archive);
    let named: Vec<(&Entry, Vec<&'static str>)> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().to_ascii_lowercase();
            SUFFIXES.iter().any(|suffix| path.ends_with(suffix))
        })
        .map(|entry| (entry, words_of(entry.path())))
        .filter(|(_, words)| !words.is_empty())
        .collect();
    let counts = named
        .par_iter()
        .filter_map(|(entry, words)| Some((words, tint_of(entry.path(), &index)?)))
        .fold(
            BTreeMap::<&str, Counts>::new,
            |mut counts, (words, tint)| {
                for &word in words {
                    let slot = counts.entry(word).or_default();
                    match tint {
                        Tint::Red => slot.red += 1,
                        Tint::Blue => slot.blue += 1,
                    }
                }
                counts
            },
        )
        .reduce(BTreeMap::new, |mut left, right| {
            for (word, other) in right {
                let slot = left.entry(word).or_default();
                slot.red += other.red;
                slot.blue += other.blue;
            }
            left
        });

    for (word, Counts { red, blue }) in &counts {
        println!("{word}: {red} mostly red, {blue} mostly blue");
    }
    let of = |word: &str| counts.get(word).map_or((0, 0), |c| (c.red, c.blue));
    for word in BLUE_WORDS {
        let (red, blue) = of(word);
        assert!(
            blue >= MIN_FILES && blue >= DOMINANCE * red,
            "{word}: {red} mostly red, {blue} mostly blue"
        );
    }
    for word in RED_WORDS {
        let (red, blue) = of(word);
        assert!(
            red >= MIN_FILES && red >= DOMINANCE * blue,
            "{word}: {red} mostly red, {blue} mostly blue"
        );
    }
    Ok(())
}
