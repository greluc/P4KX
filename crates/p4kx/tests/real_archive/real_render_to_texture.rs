// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The render-to-texture rule on the ship material files of a real Archive: decals the game fills
//! at runtime hide, screens and HUD planes that name the placeholder in another slot are drawn
//! (ADR-0232).

use std::error::Error;
use std::path::Path;

use p4kx_extraction::model::Material;
use p4kx_extraction::{Archive, ModelSource, PathIndex};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIP_FILES: &str = "data\\objects\\spaceships\\";
const PLACEHOLDER: &str = "$rendertotexture";
const MIN_DECALS: usize = 100;

fn names_placeholder(material: &Material, slot: &str) -> bool {
    material.textures.iter().any(|texture| {
        texture.map.eq_ignore_ascii_case(slot) && texture.file.eq_ignore_ascii_case(PLACEHOLDER)
    })
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn runtime_decals_are_hidden_and_screens_are_drawn() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let index = PathIndex::new(&archive);
    let (mut decals, mut screens) = (0_usize, 0_usize);
    for entry in archive.entries() {
        let lower = entry.path().to_ascii_lowercase();
        if !lower.starts_with(SHIP_FILES)
            || Path::new(&lower)
                .extension()
                .is_none_or(|extension| extension != "mtl")
        {
            continue;
        }
        let Some(library) = entry
            .path()
            .strip_prefix("Data\\")
            .and_then(|path| path.strip_suffix(".mtl"))
            .map(|path| path.replace('\\', "/"))
        else {
            continue;
        };
        let status = ModelSource::material_library(&index, entry.path(), &library);
        for material in status
            .library()
            .iter()
            .flat_map(|library| &library.materials)
        {
            if material.shader.eq_ignore_ascii_case("MeshDecal")
                && names_placeholder(material, "TexSlot1")
            {
                assert!(material.hidden(), "{} in {library}", material.name);
                decals += 1;
            } else if names_placeholder(material, "TexSlot9")
                && !material.flags.no_draw()
                && material.opacity > 0.0
            {
                assert!(!material.hidden(), "{} in {library}", material.name);
                screens += 1;
            }
        }
    }
    assert!(decals >= MIN_DECALS, "only {decals} runtime decals");
    assert!(screens > 0, "no screen found");
    Ok(())
}
