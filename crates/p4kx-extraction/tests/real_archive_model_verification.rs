//! M7 verification against a real Archive: models of every extension in both containers export
//! to valid GLB without a panic, the chunks the models skip are counted per type, and known
//! models have plausible sizes in metres. Runs with `--ignored` when `P4KX_DATA_P4K` names an
//! Archive; the tables are printed (`--nocapture`) and the GLB files go to the `TMP` folder only.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use p4kx_extraction::model::ChunkUsage;
use p4kx_extraction::{Archive, Entry, GlbOptions, ModelSource, PathIndex};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: [(&str, usize); 5] = [
    (".cgf", 300),
    (".cga", 200),
    (".skin", 300),
    (".chr", 100),
    (".vvg", 100),
];
const REQUIRED_IVO: [&str; 4] = [".cgf", ".cga", ".skin", ".chr"];
const KNOWN_MODELS: [&str; 6] = [
    "aegs_gladius.cga",
    "anvl_hornet_f7a_mk2.cga",
    "orig_300i.cga",
    "drak_cutlass_black.cga",
    "anvl_carrack_no_colide.vvg",
    "anvl_carrack.cga",
];
const MIN_LONGEST_SIDE: f32 = 0.5;
const MAX_LONGEST_SIDE: f32 = 1500.0;

#[derive(Default)]
struct Tally {
    exported: BTreeMap<(String, &'static str), usize>,
    failures: Vec<String>,
    chunks: BTreeMap<(String, &'static str), (usize, String)>,
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        for (key, count) in other.exported {
            *self.exported.entry(key).or_default() += count;
        }
        for (key, (count, example)) in other.chunks {
            let slot = self.chunks.entry(key).or_insert((0, example));
            slot.0 += count;
        }
        self.failures.extend(other.failures);
        self
    }
}

fn container_of(archive: &Archive, entry: &Entry) -> Option<&'static str> {
    let mut signature = [0_u8; 4];
    archive
        .open_entry(entry)
        .ok()?
        .read_exact(&mut signature)
        .ok()?;
    match &signature {
        b"#ivo" => Some("#ivo"),
        b"CrCh" => Some("CrCh"),
        _ => None,
    }
}

fn extension_of<'a>(path: &str, extensions: impl IntoIterator<Item = &'a str>) -> &'a str {
    let lower = path.to_ascii_lowercase();
    extensions
        .into_iter()
        .find(|extension| lower.ends_with(extension))
        .unwrap_or("")
}

fn convert(
    archive: &Archive,
    index: &PathIndex<'_>,
    entry: &Entry,
    out: &Path,
    tally: &mut Tally,
) -> Result<(), String> {
    let container = container_of(archive, entry).ok_or("neither #ivo nor CrCh")?;
    let source =
        ModelSource::from_index(index, entry.path()).map_err(|error| format!("{error}"))?;
    source
        .write_glb(&GlbOptions::default(), out)
        .map_err(|error| format!("{error}"))?;
    let bytes = std::fs::read(out).map_err(|error| error.to_string())?;
    check_glb(&bytes)?;
    let extension = extension_of(entry.path(), SAMPLES.map(|(extension, _)| extension));
    *tally
        .exported
        .entry((extension.to_owned(), container))
        .or_default() += 1;
    for chunk in &source.model().chunks {
        let usage = match chunk.usage {
            ChunkUsage::Read => continue,
            ChunkUsage::Skipped => "skipped",
            ChunkUsage::Unknown => "unknown",
        };
        let slot = tally
            .chunks
            .entry((
                format!("{} {}", chunk.chunk_type, chunk.chunk_type.hex_id()),
                usage,
            ))
            .or_insert_with(|| (0, entry.path().to_owned()));
        slot.0 += 1;
    }
    Ok(())
}

#[expect(clippy::print_stdout, reason = "the tables are the measurement")]
fn report(tally: &Tally) {
    println!("\nexported models per extension and container:");
    for ((extension, container), count) in &tally.exported {
        println!("{extension:>6} {container:>5} {count:>6}");
    }
    println!("\nchunks the models did not read (type, usage, files, first example):");
    for ((chunk, usage), (count, example)) in &tally.chunks {
        println!("{chunk:<40} {usage:<8} {count:>6}  {example}");
    }
    println!("\n{} failures", tally.failures.len());
    for failure in &tally.failures {
        println!("FAIL {failure}");
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn models_of_every_extension_export_valid_glb_and_skipped_chunk_types_are_counted() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let out = tempfile::tempdir()?;
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

    let tally = sample
        .par_iter()
        .enumerate()
        .map(|(position, entry)| {
            let mut tally = Tally::default();
            let glb = out.path().join(format!("{position}.glb"));
            let result = catch_unwind(AssertUnwindSafe(|| {
                convert(&archive, &index, entry, &glb, &mut tally)
            }));
            let _ignored = std::fs::remove_file(&glb);
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tally.failures.push(format!("{}: {error}", entry.path())),
                Err(_) => tally.failures.push(format!("{}: PANIC", entry.path())),
            }
            tally
        })
        .reduce(Tally::default, Tally::merge);

    report(&tally);

    assert!(tally.failures.is_empty(), "failed: {:#?}", tally.failures);
    for extension in REQUIRED_IVO {
        assert!(
            tally
                .exported
                .get(&(extension.to_owned(), "#ivo"))
                .is_some_and(|count| *count > 0),
            "no {extension} model in the #ivo container exported"
        );
    }
    assert!(
        tally
            .exported
            .keys()
            .any(|(_, container)| *container == "CrCh"),
        "no model in the CrCh container exported"
    );
    Ok(())
}

#[test]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn known_models_have_plausible_dimensions_in_metres() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let mut found = 0_usize;
    for entry in archive.entries() {
        let lower = entry.path().to_ascii_lowercase();
        if !KNOWN_MODELS.iter().any(|name| lower.ends_with(name)) {
            continue;
        }
        let source = ModelSource::from_index(&index, entry.path())?;
        let model = source.model();
        let lod = model.lods.first().ok_or("LOD0")?;
        let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
        for position in lod.meshes.iter().flat_map(|mesh| &mesh.positions) {
            for (axis, value) in position.iter().enumerate() {
                if let (Some(low), Some(high)) = (low.get_mut(axis), high.get_mut(axis)) {
                    *low = low.min(*value);
                    *high = high.max(*value);
                }
            }
        }
        let size: [f32; 3] = std::array::from_fn(|axis| {
            high.get(axis).copied().unwrap_or(0.0) - low.get(axis).copied().unwrap_or(0.0)
        });
        let longest = size.into_iter().fold(0.0_f32, f32::max);
        println!(
            "{}: {} nodes, {} meshes, {} vertices, {} triangles, size {size:.2?} m",
            entry.path(),
            model.nodes.len(),
            lod.meshes.len(),
            lod.vertex_count(),
            lod.triangle_count(),
        );
        assert!(
            (MIN_LONGEST_SIDE..=MAX_LONGEST_SIDE).contains(&longest),
            "{}: longest side {longest} m is no plausible size",
            entry.path()
        );
        found += 1;
    }
    assert!(found > 0, "none of the known models is in the Archive");
    Ok(())
}
