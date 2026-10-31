// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layer files shared by the loads of several libraries, their byte budget, and the plain
//! read that leaves them unread.

use std::cell::Cell;
use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    CompanionError, CompanionPath, LayerCache, LodSelection, MAX_LAYER_BYTES, MaterialLibrary,
    MaterialStatus, Model,
};
use p4kx_testkit::{TestModel, TestMtl, TestMtlMaterial};

type TestResult = Result<(), Box<dyn Error>>;

const FIRST: &str = "objects/test/first";
const SECOND: &str = "objects/test/second";
const SET: &str = "materials/layers/set";

fn hull(submaterial: &str) -> Vec<u8> {
    let material = TestMtlMaterial::new("hull").layer_with(&[
        ("Name", "L"),
        ("Path", "materials/layers/set.mtl"),
        ("Submtl", submaterial),
    ]);
    TestMtl::single(&material).cryxml()
}

fn set() -> Vec<u8> {
    TestMtl::multi(&[
        TestMtlMaterial::new("first").diffuse([0.1; 3]),
        TestMtlMaterial::new("second").diffuse([0.9; 3]),
    ])
    .cryxml()
}

fn diffuse_of(status: &MaterialStatus) -> Result<Option<[f32; 3]>, Box<dyn Error>> {
    let library = status.library().ok_or("loaded")?;
    let layer = library
        .materials
        .first()
        .and_then(|material| material.layers.first())
        .ok_or("layer")?;
    Ok(layer.surface.as_ref().map(|surface| surface.diffuse))
}

#[test]
fn a_layer_file_shared_by_two_libraries_is_fetched_once() -> TestResult {
    let fetched = Cell::new(0);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(FIRST) => Some(hull("first")),
            CompanionPath::Material(SECOND) => Some(hull("second")),
            CompanionPath::Material(SET) => {
                fetched.set(fetched.get() + 1);
                Some(set())
            }
            _ => None,
        })
    };
    let mut cache = LayerCache::new();

    let first = MaterialStatus::load_cached(Some(FIRST), &mut companions, &mut cache);
    let second = MaterialStatus::load_cached(Some(SECOND), &mut companions, &mut cache);

    assert_eq!(diffuse_of(&first)?, Some([0.1; 3]), "first");
    assert_eq!(diffuse_of(&second)?, Some([0.9; 3]), "another sub-material");
    assert_eq!(fetched.get(), 1, "one fetch and one parse for both");
    Ok(())
}

#[test]
fn without_a_shared_cache_each_load_fetches_the_file() {
    let fetched = Cell::new(0);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(FIRST) => Some(hull("first")),
            CompanionPath::Material(SET) => {
                fetched.set(fetched.get() + 1);
                Some(set())
            }
            _ => None,
        })
    };

    MaterialStatus::load(Some(FIRST), &mut companions);
    MaterialStatus::load(Some(FIRST), &mut companions);

    assert_eq!(fetched.get(), 2);
}

#[test]
fn the_bytes_one_load_makes_the_reader_fetch_are_limited() -> TestResult {
    let big = vec![0_u8; MAX_LAYER_BYTES / 2 + 1];
    let layers = ["a", "b", "c"].map(|name| format!("materials/layers/{name}.mtl"));
    let material = layers
        .iter()
        .fold(TestMtlMaterial::new("hull"), |material, path| {
            material.layer_with(&[("Name", "L"), ("Path", path)])
        });
    let library = TestMtl::single(&material).cryxml();
    let fetched = Cell::new(0);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(FIRST) => Some(library.clone()),
            CompanionPath::Material(_) => {
                fetched.set(fetched.get() + 1);
                Some(big.clone())
            }
            _ => None,
        })
    };

    let status = MaterialStatus::load(Some(FIRST), &mut companions);

    let library = status.library().ok_or("loaded")?;
    assert_eq!(fetched.get(), 2, "the third file is over the budget");
    assert_eq!(
        library.unresolved_layers.len(),
        3,
        "none parses, all listed"
    );
    Ok(())
}

#[test]
fn a_plain_load_reads_no_layer_file() -> TestResult {
    let fetched = Cell::new(0);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(FIRST) => Some(hull("first")),
            CompanionPath::Material(_) => {
                fetched.set(fetched.get() + 1);
                Some(set())
            }
            _ => None,
        })
    };

    let status = MaterialStatus::load_plain(Some(FIRST), &mut companions);

    let library = status.library().ok_or("loaded")?;
    assert_eq!(fetched.get(), 0, "nothing is asked for");
    assert!(library.unresolved_layers.is_empty(), "nothing is reported");
    assert_eq!(diffuse_of(&status)?, None, "no surface");
    Ok(())
}

#[test]
fn a_model_read_with_plain_materials_reads_no_layer_file_and_a_drawn_read_does() -> TestResult {
    let cube = TestModel::cube().with_material_library(FIRST);
    let (cgfm, library) = (cube.cgfm(), hull("first"));
    let file = ChunkFile::parse(cube.cgf())?;
    let fetched = Cell::new(0);
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("cube.cgfm") => Some(cgfm.clone()),
            CompanionPath::Material(FIRST) => Some(library.clone()),
            CompanionPath::Material(_) => {
                fetched.set(fetched.get() + 1);
                Some(set())
            }
            _ => None,
        })
    };

    let plain = Model::read_plain_materials("cube.cgf", &file, &mut companions, LodSelection::All)?;
    assert_eq!(fetched.get(), 0, "no layer file for a listing");
    assert_eq!(diffuse_of(&plain.materials)?, None, "no surface");

    let drawn = Model::read("cube.cgf", &file, &mut companions)?;
    assert_eq!(fetched.get(), 1, "one layer file for drawing");
    assert_eq!(diffuse_of(&drawn.materials)?, Some([0.1; 3]), "surface");
    Ok(())
}

#[test]
fn the_sub_material_of_a_layer_is_picked_by_one_rule() -> TestResult {
    let library = MaterialLibrary::parse(SET, &set())?;
    assert_eq!(
        library.layer_position(""),
        Some(0),
        "the first without a name"
    );
    assert_eq!(
        library.layer_position(" Second "),
        Some(1),
        "by name, trimmed"
    );
    assert_eq!(library.layer_position("absent"), None, "no such one");
    let single = MaterialLibrary::parse(
        SET,
        &TestMtl::single(&TestMtlMaterial::new("only")).cryxml(),
    )?;
    assert_eq!(single.layer_position("anything"), Some(0), "a single file");
    Ok(())
}
