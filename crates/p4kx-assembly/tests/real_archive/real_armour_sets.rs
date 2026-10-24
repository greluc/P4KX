// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The armour sets of a real Archive on both body variants: no piece is the loot box of its
//! Entity's main geometry, and a sample of the NPC Loadouts exports as valid GLB.

use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, LoadoutResolver,
    ModelProvider, PartOrigin, Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_model::{Lod, Mesh};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const PIECES: [&str; 2] = [
    "rsi_odyssey_undersuit_01_01_01",
    "rsi_odyssey_undersuit_helmet_01_01_01",
];

const LOADOUT_FOLDER: &str = "scripts/loadouts/character/human/armor/";

const SURVEY_LIMIT: usize = 40;

const LOD: u32 = 2;

const BOX_CORNERS: usize = 8;

const BOX_INDICES: usize = 36;

fn corners(mesh: &Mesh) -> usize {
    mesh.positions
        .iter()
        .map(|position| position.map(f32::to_bits))
        .collect::<HashSet<_>>()
        .len()
}

fn is_box(mesh: &Mesh) -> bool {
    corners(mesh) == BOX_CORNERS && mesh.indices.len() == BOX_INDICES
}

fn meshes(part: &ArmourPart) -> Vec<&Mesh> {
    part.model()
        .lods
        .iter()
        .flat_map(|lod| &lod.meshes)
        .collect()
}

fn boxes(armour: &Armour) -> Vec<String> {
    armour
        .parts()
        .iter()
        .filter(|part| {
            let meshes = meshes(part);
            meshes.is_empty() || meshes.iter().all(|mesh| is_box(mesh))
        })
        .map(|part| format!("{} ({})", part.name(), part.file()))
        .collect()
}

fn placeholders(armour: &Armour) -> usize {
    armour
        .parts()
        .iter()
        .filter(|part| {
            let meshes = meshes(part);
            !meshes.is_empty() && meshes.iter().all(|mesh| mesh.indices.len() <= 3)
        })
        .count()
}

fn rigid(armour: &Armour) -> usize {
    armour
        .parts()
        .iter()
        .filter(|part| !meshes(part).iter().any(|mesh| mesh.influences.is_some()))
        .count()
}

fn same_file(left: &str, right: &str) -> bool {
    left.replace('\\', "/")
        .eq_ignore_ascii_case(&right.replace('\\', "/"))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn a_piece_is_worn_with_the_skin_of_its_body_and_never_with_the_loot_box() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    for name in PIECES {
        let record = resolver.entities().find(name)?;
        let wearable = resolver.wearable(&record)?;
        let main = wearable
            .geometry()
            .map(|geometry| geometry.path().to_owned());
        for variant in BodyVariant::ALL {
            let skin = wearable
                .variants()
                .iter()
                .find(|found| found.has_tag(variant.tag()))
                .map(|found| found.geometry().path().to_owned())
                .ok_or_else(|| format!("{name}: no sub-geometry for {variant:?}"))?;
            let armour = Armour::build(
                variant,
                &[Piece::wearable(wearable.clone())],
                &models,
                &AssemblyOptions::default().with_lod(LOD),
            )?;
            let part = armour
                .parts()
                .iter()
                .find(|part| part.origin() == PartOrigin::Piece)
                .ok_or_else(|| format!("{name} {variant:?}: the piece is not worn"))?;
            assert!(same_file(part.file(), &skin), "{name}: {}", part.file());
            if let Some(main) = &main {
                assert!(
                    !same_file(part.file(), main),
                    "{name} {variant:?}: worn with the main geometry {main}"
                );
            }
            assert_eq!(boxes(&armour), Vec::<String>::new(), "{name} {variant:?}");

            let worn: usize = part.model().lods.iter().map(Lod::vertex_count).sum();
            let crate_vertices = main
                .as_ref()
                .and_then(|main| models.model(main, LOD).ok())
                .map(|provided| {
                    provided
                        .model
                        .lods
                        .iter()
                        .map(Lod::vertex_count)
                        .sum::<usize>()
                });
            println!(
                "{name} {variant:?}: worn {} with {worn} vertices; main geometry {main:?} with \
                 {crate_vertices:?}",
                part.file()
            );
        }
    }
    Ok(())
}

fn loadout_paths(archive: &Archive) -> Vec<String> {
    let mut all: Vec<String> = archive
        .entries()
        .iter()
        .map(|entry| entry.path().replace('\\', "/"))
        .filter(|path| {
            let lowered = path.to_ascii_lowercase();
            lowered.contains(LOADOUT_FOLDER)
                && Path::new(&lowered)
                    .extension()
                    .is_some_and(|extension| extension == "xml")
        })
        .collect();
    all.sort();
    all
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the sets, pieces, sizes and times are the measurement"
)]
fn a_sample_of_the_npc_loadouts_exports_on_both_bodies_without_a_box() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);

    let all = loadout_paths(&archive);
    let stride = (all.len() / SURVEY_LIMIT).max(1);
    let sample: Vec<&String> = all.iter().step_by(stride).collect();
    assert!(!sample.is_empty(), "the Archive holds armour Loadouts");

    let mut tally = Tally::default();
    let started = Instant::now();
    for path in sample {
        let Ok(tree) = resolver.resolve_file(path) else {
            tally.failures.push(format!("{path}: not resolved"));
            continue;
        };
        let pieces = resolver.worn(&tree);
        if pieces.is_empty() {
            tally.empty += 1;
            continue;
        }
        for variant in BodyVariant::ALL {
            if let Err(failure) = tally.export(path, variant, &pieces, &models) {
                tally
                    .failures
                    .push(format!("{path} {variant:?}: {failure}"));
            }
        }
    }
    println!(
        "{} Loadouts in the Archive, {} sampled ({} without worn pieces), {} exports, {} pieces \
         worn ({} parts without skin weights, {} one-triangle placeholders), pieces skipped {:?}, {} GLB bytes, slowest {:?}, all in {:?}",
        all.len(),
        all.len().div_ceil(stride),
        tally.empty,
        tally.sets,
        tally.worn,
        tally.rigid,
        tally.placeholders,
        tally.skipped,
        tally.bytes,
        tally.slowest,
        started.elapsed()
    );
    assert!(tally.sets >= SURVEY_LIMIT, "{} exports", tally.sets);
    assert_eq!(tally.failures, Vec::<String>::new());
    assert_eq!(tally.boxes, Vec::<String>::new());
    Ok(())
}

#[derive(Default)]
struct Tally {
    sets: usize,
    empty: usize,
    worn: usize,
    rigid: usize,
    placeholders: usize,
    skipped: BTreeMap<String, usize>,
    bytes: usize,
    slowest: Duration,
    failures: Vec<String>,
    boxes: Vec<String>,
}

impl Tally {
    fn export(
        &mut self,
        path: &str,
        variant: BodyVariant,
        pieces: &[Piece],
        models: &ArchiveModels<'_>,
    ) -> Result<(), Box<dyn Error>> {
        let started = Instant::now();
        let armour = Armour::build(
            variant,
            pieces,
            models,
            &AssemblyOptions::default().with_lod(LOD),
        )?;
        let mut bytes = Vec::new();
        armour.write_glb(&GlbOptions::default().with_lod(LOD), &mut bytes)?;
        check_glb(&bytes)?;
        self.slowest = self.slowest.max(started.elapsed());
        self.bytes += bytes.len();
        self.sets += 1;
        self.worn += armour.count(PartOrigin::Piece);
        self.rigid += rigid(&armour);
        self.placeholders += placeholders(&armour);
        for issue in armour.issues().iter().filter(|issue| issue.kind().skips()) {
            *self
                .skipped
                .entry(format!("{} ({})", issue.kind().code(), variant.code()))
                .or_default() += 1;
        }
        self.boxes.extend(
            boxes(&armour)
                .into_iter()
                .map(|part| format!("{path} {variant:?}: {part}")),
        );
        Ok(())
    }
}
