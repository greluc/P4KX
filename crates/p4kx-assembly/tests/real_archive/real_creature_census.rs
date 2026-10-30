// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Census of the Entities whose geometry is a character file outside the human rigs: how many
//! have a readable model, a missing skeleton or a missing material library, and whether every
//! one of them exports to a valid GLB.

#![expect(clippy::print_stdout, reason = "the census is the output")]

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_model::MaterialStatus;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const TEXTURE_MIP: u32 = 3;
const MAX_EXAMPLES: usize = 2;

/// What the Entities of one geometry folder and file kind came to.
#[derive(Default)]
struct Row {
    entities: usize,
    model_readable: usize,
    draws: usize,
    geometry_missing: usize,
    geometry_unreadable: usize,
    skeleton_missing: usize,
    skeleton_unreadable: usize,
    material_missing: usize,
    examples: Vec<String>,
}

impl Row {
    fn count(&mut self, class: &str, assembly: &Assembly) {
        self.entities += 1;
        self.model_readable += usize::from(assembly.modelled_parts() > 0);
        self.draws += usize::from(assembly.parts().iter().any(|part| {
            part.model()
                .is_some_and(|model| model.lods.iter().any(|lod| lod.vertex_count() > 0))
        }));
        self.material_missing += usize::from(
            assembly
                .parts()
                .iter()
                .any(|part| matches!(part.materials(), MaterialStatus::Missing { .. })),
        );
        let has = |code: &str| {
            assembly
                .issues()
                .iter()
                .any(|issue| issue.kind().code() == code)
        };
        self.geometry_missing += usize::from(has("geometry_missing"));
        self.geometry_unreadable += usize::from(has("geometry_unreadable"));
        self.skeleton_missing += usize::from(has("skeleton_missing"));
        self.skeleton_unreadable += usize::from(has("skeleton_unreadable"));
        let listed = assembly.issues().iter().filter(|issue| {
            ["geometry", "skeleton"]
                .iter()
                .any(|prefix| issue.kind().code().starts_with(prefix))
        });
        for issue in listed.take(MAX_EXAMPLES.saturating_sub(self.examples.len())) {
            self.examples.push(format!("{class}: {}", issue.kind()));
        }
    }

    fn print(&self, group: &str) {
        println!(
            "{group}: {}, {}, {}, {} / {}, {} / {}, {}",
            self.entities,
            self.model_readable,
            self.draws,
            self.geometry_missing,
            self.geometry_unreadable,
            self.skeleton_missing,
            self.skeleton_unreadable,
            self.material_missing
        );
        for example in &self.examples {
            println!("    {example}");
        }
    }
}

/// Returns the file kind and the folder of the geometry file, lower case, as the census groups.
fn group_of(path: &str) -> String {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    let (folder, file) = path.rsplit_once('/').unwrap_or(("", &path));
    let kind = file.rsplit_once('.').map_or("", |(_, kind)| kind);
    format!("{kind} {folder}")
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_nonhuman_character_entity_exports_to_a_valid_glb_and_the_census_lists_what_is_missing()
-> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let glb_options = GlbOptions::default()
        .with_materials(MaterialMode::Textures)
        .with_texture_mip(TEXTURE_MIP);
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for record in forge.records().filter(|record| {
        record
            .type_name()
            .is_ok_and(|name| name == p4kx_assembly::ENTITY_TYPE)
    }) {
        let Ok(tree) = resolver.resolve(&record) else {
            continue;
        };
        let Some(geometry) = tree.geometry() else {
            continue;
        };
        if !geometry.is_rigged_file_outside_human_rigs() {
            continue;
        }
        let assembly =
            Assembly::build_with_palettes(&tree, &models, &palettes, &AssemblyOptions::default())?;
        let mut bytes = Vec::new();
        assembly.write_glb_with(&glb_options, &textures, &mut bytes)?;
        check_glb(&bytes).map_err(|error| format!("{}: {error}", tree.class_name()))?;
        rows.entry(group_of(geometry.path()))
            .or_default()
            .count(tree.class_name(), &assembly);
    }

    println!(
        "kind folder: entities, model readable, draws vertices, geometry missing / unreadable, \
         skeleton missing / unreadable, material missing"
    );
    for (group, row) in &rows {
        row.print(group);
    }
    assert!(
        rows.iter()
            .any(|(group, row)| group.starts_with("chr ") && row.draws > 0),
        "a skeleton-rooted character that draws"
    );
    Ok(())
}
