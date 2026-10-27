// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The relief-decal rule on the ship material files of a real Archive: decals with a grey mask hide,
//! decals with a coloured diffuse map (buttons, stitching, damage) are drawn (ADR-0225).

#![expect(clippy::print_stdout, reason = "the judged materials are the output")]

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use p4kx_extraction::model::Material;
use p4kx_extraction::{Archive, ModelSource, PathIndex};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIP_FILES: &str = "data\\objects\\spaceships\\";
const GREY_MASK: &str = "rsi_pom_decals_diff";
const COLOURED_MAPS: [&str; 4] = [
    "anvl_switches_buttons_diff",
    "aegs_switches_buttons_diff",
    "gen_pom_metal_dmg_atlas_01_diff",
    "behr_trim_1_a_diff",
];
const MIN_JUDGED: usize = 20;

fn ship_libraries(archive: &Archive) -> Vec<(String, String)> {
    archive
        .entries()
        .iter()
        .filter(|entry| {
            let lower = entry.path().to_ascii_lowercase();
            lower.starts_with(SHIP_FILES)
                && Path::new(&lower)
                    .extension()
                    .is_some_and(|extension| extension == "mtl")
        })
        .filter_map(|entry| {
            let library = entry
                .path()
                .strip_prefix("Data\\")?
                .strip_suffix(".mtl")?
                .replace('\\', "/");
            Some((entry.path().to_owned(), library))
        })
        .collect()
}

fn map_stem(material: &Material) -> String {
    material
        .textures
        .iter()
        .find(|texture| texture.map.eq_ignore_ascii_case("TexSlot1"))
        .and_then(|texture| texture.file.rsplit(['/', '\\']).next())
        .and_then(|name| name.split('.').next())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn coloured_relief_candidates_are_drawn_and_grey_ones_are_hidden() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let mut judged: BTreeMap<(String, String, bool), usize> = BTreeMap::new();
    for (entry_path, library) in ship_libraries(&archive) {
        let status = ModelSource::material_library(&index, &entry_path, &library);
        for material in status
            .library()
            .iter()
            .flat_map(|library| &library.materials)
        {
            if material.diffuse_map_is_grey.is_some() {
                let key = (
                    material.name.clone(),
                    map_stem(material),
                    material.is_relief_decal(),
                );
                *judged.entry(key).or_default() += 1;
            }
        }
    }
    for ((name, map, hidden), files) in &judged {
        println!("{files:4} x {name} {map} hidden {hidden}");
    }
    assert!(judged.len() >= MIN_JUDGED, "only {} judged", judged.len());
    assert!(
        judged
            .iter()
            .any(|((_, map, hidden), _)| map == GREY_MASK && *hidden),
        "{GREY_MASK} is hidden"
    );
    for coloured in COLOURED_MAPS {
        let hidden = judged
            .iter()
            .any(|((_, map, hidden), _)| map == coloured && *hidden);
        assert!(!hidden, "{coloured} is drawn");
    }
    Ok(())
}
