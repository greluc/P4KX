//! Levels of detail and sub-object hierarchies of the models of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one (and `P4KX_DATA_P4K_B` for a second).

use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use p4kx_chunks::{ChunkFile, ivo};
use p4kx_extraction::model::{Lod, LodSelection, Model, ModelError, Transform};
use p4kx_extraction::{Archive, ConversionError, Entry, GlbOptions, ModelSource, PathIndex};
use p4kx_testkit::{check_glb, real_archive_b_path, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const LOD_SAMPLE: usize = 120;
const HIERARCHY_SAMPLE: usize = 600;
const MAX_CHUNK_FILE: u64 = 1 << 28;
const MIN_DECREASING_SHARE: f64 = 0.9;
const MIN_FITTING_SHARE: f64 = 0.5;
const BOX_TOLERANCE: f32 = 0.02;

fn archives() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = vec![real_archive_path()?];
    paths.extend(real_archive_b_path());
    Ok(paths)
}

fn sample<'a>(archive: &'a Archive, suffix: &str, count: usize) -> Vec<&'a Entry> {
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(suffix))
        .collect();
    let stride = (candidates.len() / count).max(1);
    candidates.into_iter().step_by(stride).take(count).collect()
}

#[derive(Default)]
struct LodTally {
    models: u64,
    levels: u64,
    pairs: u64,
    decreasing: u64,
    failures: Vec<String>,
}

impl LodTally {
    fn merge(mut self, other: Self) -> Self {
        self.models += other.models;
        self.levels += other.levels;
        self.pairs += other.pairs;
        self.decreasing += other.decreasing;
        self.failures.extend(other.failures);
        self
    }

    fn fail(mut self, path: &str, what: impl std::fmt::Display) -> Self {
        self.failures.push(format!("{path}: {what}"));
        self
    }
}

fn export_level(
    index: &PathIndex<'_>,
    base: &str,
    lod: &Lod,
    out: &std::path::Path,
) -> Result<(), String> {
    let source = ModelSource::from_index_with(index, base, LodSelection::Only(lod.level))
        .map_err(|error| format!("{error:?}"))?;
    let only = source.model().lods.first().map(Lod::triangle_count);
    if source.model().lods.len() != 1 || only != Some(lod.triangle_count()) {
        return Err("read alone, it differs".to_owned());
    }
    source
        .write_glb(&GlbOptions::default().with_lod(lod.level), out)
        .map_err(|error| format!("{error:?}"))?;
    let bytes = fs::read(out).map_err(|error| error.to_string())?;
    check_glb(&bytes).map(|_| ())
}

fn check_levels(index: &PathIndex<'_>, base: &str, dir: &std::path::Path) -> LodTally {
    let mut tally = LodTally::default();
    let model = match ModelSource::from_index(index, base) {
        Ok(source) => source,
        Err(error) => return tally.fail(base, format!("{error:?}")),
    };
    let model = model.model();
    tally.models = 1;
    let levels: Vec<u32> = model.lods.iter().map(|lod| lod.level).collect();
    if levels.first() != Some(&0)
        || !levels
            .windows(2)
            .all(|pair| matches!(pair, [first, second] if first < second))
    {
        return tally.fail(base, format!("levels {levels:?}"));
    }
    if !levels.contains(&1) {
        return tally.fail(base, format!("an _lod1 file but levels {levels:?}"));
    }
    tally.levels = levels.len() as u64;
    for lod in &model.lods {
        if lod.meshes.iter().any(|mesh| mesh.node >= model.nodes.len()) {
            return tally.fail(base, format!("LOD{} mesh on a missing node", lod.level));
        }
    }
    for pair in model.lods.windows(2) {
        if let [finer, coarser] = pair {
            tally.pairs += 1;
            tally.decreasing += u64::from(coarser.triangle_count() <= finer.triangle_count());
        }
    }
    for lod in &model.lods {
        let out = dir.join(format!(
            "{}.glb",
            rayon::current_thread_index().unwrap_or(0)
        ));
        let exported = export_level(index, base, lod, &out);
        let _ignored = fs::remove_file(&out);
        if let Err(error) = exported {
            return tally.fail(base, format!("LOD{}: {error}", lod.level));
        }
    }
    match ModelSource::from_index_with(index, base, LodSelection::Only(99)) {
        Err(ConversionError::Model {
            source: ModelError::NoSuchLod { lod: 99, .. },
            ..
        }) => {}
        other => return tally.fail(base, format!("LOD99: {other:?}")),
    }
    tally
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn models_with_levels_of_detail_read_and_export_every_level() -> TestResult {
    let out = tempfile::tempdir()?;
    for path in archives()? {
        let archive = Archive::open(&path)?;
        let index = PathIndex::new(&archive);
        let mut bases: Vec<String> = Vec::new();
        for (suffix, base_suffix) in [
            ("_lod1.cgf", ".cgf"),
            ("_lod1.cga", ".cga"),
            ("_lod1.skin", ".skin"),
        ] {
            for entry in sample(&archive, suffix, LOD_SAMPLE) {
                let stem = entry.path().get(..entry.path().len() - suffix.len());
                if let Some(stem) = stem {
                    let base = format!("{stem}{base_suffix}");
                    if index.find(&base).is_some() {
                        bases.push(base);
                    }
                }
            }
        }
        let tally = bases
            .par_iter()
            .map(|base| check_levels(&index, base, out.path()))
            .reduce(LodTally::default, LodTally::merge);

        assert!(
            tally.models > 100,
            "{}: {} models",
            path.display(),
            tally.models
        );
        assert!(
            tally.failures.is_empty(),
            "{}: {:#?}",
            path.display(),
            tally.failures
        );
        #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
        let share = tally.decreasing as f64 / tally.pairs.max(1) as f64;
        assert!(
            share >= MIN_DECREASING_SHARE,
            "{}: only {share:.3} of {} level pairs get no more triangles ({} models, {} levels)",
            path.display(),
            tally.pairs,
            tally.models,
            tally.levels
        );
    }
    Ok(())
}

fn apply(transform: &Transform, [x, y, z]: [f32; 3]) -> [f32; 3] {
    transform
        .rows
        .map(|[a, b, c, d]| a.mul_add(x, b.mul_add(y, c.mul_add(z, d))))
}

fn place(model: &Model, node: usize, point: [f32; 3], whole_chain: bool) -> [f32; 3] {
    let mut point = point;
    let mut at = Some(node);
    for _ in 0..model.nodes.len() {
        let Some(current) = at.and_then(|at| model.nodes.get(at)) else {
            break;
        };
        point = apply(&current.transform, point);
        at = current.parent.filter(|_| whole_chain);
    }
    point
}

type Bounds = ([f32; 3], [f32; 3]);

fn bounds_of(model: &Model, place: impl Fn(usize, [f32; 3]) -> [f32; 3]) -> Option<Bounds> {
    let lod = model.lods.first()?;
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for mesh in &lod.meshes {
        for &point in &mesh.positions {
            let at = place(mesh.node, point);
            for ((low, high), value) in min.iter_mut().zip(&mut max).zip(at) {
                *low = low.min(value);
                *high = high.max(value);
            }
        }
    }
    (min[0] <= max[0]).then_some((min, max))
}

fn fits((found_min, found_max): Bounds, (wanted_min, wanted_max): Bounds) -> bool {
    found_min
        .iter()
        .zip(found_max)
        .zip(wanted_min.iter().zip(wanted_max))
        .all(|((&low, high), (&wanted_low, wanted_high))| {
            let tolerance = (wanted_high - wanted_low).abs().max(1.0) * BOX_TOLERANCE;
            (low - wanted_low).abs() < tolerance && (high - wanted_high).abs() < tolerance
        })
}

fn mesh_info_box(archive: &Archive, entry: &Entry) -> Option<Bounds> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .ok()?
        .take(MAX_CHUNK_FILE)
        .read_to_end(&mut bytes)
        .ok()?;
    let file = ChunkFile::parse(bytes).ok()?;
    let (index, _) = file.chunks_of(ivo::MESH_INFO).next()?;
    let data = file.chunk_data(index)?;
    let word = |at: usize| {
        data.get(at * 4..at * 4 + 4)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map_or(0.0, f32::from_le_bytes)
    };
    Some(([word(6), word(7), word(8)], [word(9), word(10), word(11)]))
}

#[derive(Default, Clone, Copy)]
struct HierarchyTally {
    models: u64,
    chain: u64,
    own: u64,
    none: u64,
}

impl HierarchyTally {
    const fn merge(mut self, other: Self) -> Self {
        self.models += other.models;
        self.chain += other.chain;
        self.own += other.own;
        self.none += other.none;
        self
    }
}

fn hierarchy_fit(index: &PathIndex<'_>, archive: &Archive, entry: &Entry) -> HierarchyTally {
    let mut tally = HierarchyTally::default();
    let Ok(source) = ModelSource::from_index(index, entry.path()) else {
        return tally;
    };
    let model = source.model();
    let (Some(info), true) = (mesh_info_box(archive, entry), model.nodes.len() > 1) else {
        return tally;
    };
    let fit = |whole_chain: Option<bool>| {
        bounds_of(model, |node, point| {
            whole_chain.map_or(point, |chain| place(model, node, point, chain))
        })
        .is_some_and(|found| fits(found, info))
    };
    tally.models = 1;
    tally.chain = u64::from(fit(Some(true)));
    tally.own = u64::from(fit(Some(false)));
    tally.none = u64::from(fit(None));
    tally
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_node_chain_places_the_sub_objects_where_the_mesh_info_box_has_them() -> TestResult {
    for path in archives()? {
        let archive = Archive::open(&path)?;
        let index = PathIndex::new(&archive);
        let mut entries = sample(&archive, ".cga", HIERARCHY_SAMPLE);
        entries.extend(sample(&archive, ".cgf", HIERARCHY_SAMPLE));
        let tally = entries
            .par_iter()
            .map(|entry| hierarchy_fit(&index, &archive, entry))
            .reduce(HierarchyTally::default, HierarchyTally::merge);

        assert!(
            tally.models > 50,
            "{}: {} models",
            path.display(),
            tally.models
        );
        #[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
        let share = tally.chain as f64 / tally.models as f64;
        assert!(
            share >= MIN_FITTING_SHARE && tally.chain > tally.own && tally.chain > tally.none,
            "{}: of {} models with several nodes the whole chain fits {} boxes, the node alone {}, \
             no transform {}",
            path.display(),
            tally.models,
            tally.chain,
            tally.own,
            tally.none
        );
    }
    Ok(())
}
