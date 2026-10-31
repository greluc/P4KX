// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Hair materials of pieces without a Character's material choices: they are drawn in the colour
//! of their own shader parameters, in the library and in the written GLB.

use std::error::Error;
use std::sync::LazyLock;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver, ModelFailure,
    ModelProvider, PartOrigin, Piece, ProvidedModel,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_model::{MaterialLibrary, MaterialStatus, Model};
use p4kx_testkit::{armour_entries, check_glb};

use crate::assembly_support::{TestResult, fixture};

const LIBRARY: &str = r#"<Material><SubMaterials>
    <Material Name="unused" Shader="Illum" Diffuse="0.5,0.5,0.5"/>
    <Material Name="lash" Shader="HairPBR" Diffuse="1,1,1">
      <PublicParams BaseMelanin="0" BaseMelaninRedness="0" BaseTintColor="5,-5,0.5"/>
    </Material>
    <Material Name="plain" Shader="Illum" Diffuse="0.5,0.5,0.5"/>
    </SubMaterials></Material>"#;

/// Reads from the Archive but answers every material library with [`LIBRARY`].
struct Lashes<'a> {
    models: ArchiveModels<'a>,
    library: MaterialStatus,
}

impl ModelProvider for Lashes<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        self.models.model(path, lod)
    }

    fn library(&self, _entry: &str, _name: &str) -> MaterialStatus {
        self.library.clone()
    }

    fn file(&self, path: &str, limit: u64) -> Result<Vec<u8>, ModelFailure> {
        self.models.file(path, limit)
    }

    fn deployed(&self, path: &str, model: &Model) -> Result<Option<Model>, ModelFailure> {
        self.models.deployed(path, model)
    }
}

fn diffuse_of(armour: &Armour, origin: PartOrigin, name: &str) -> Result<[f32; 3], Box<dyn Error>> {
    Ok(armour
        .parts()
        .iter()
        .find(|part| part.origin() == origin)
        .and_then(|part| part.materials().library())
        .and_then(|library| library.materials.iter().find(|m| m.name == name))
        .ok_or_else(|| format!("no material {name}"))?
        .diffuse)
}

fn near(found: &[f32], wanted: [f32; 3]) -> bool {
    found
        .iter()
        .zip(wanted)
        .all(|(found, wanted)| (found - wanted).abs() < 1e-3)
}

type Worn = (Armour, Vec<u8>);

static SUIT: LazyLock<Result<Worn, String>> =
    LazyLock::new(|| wear_suit().map_err(|error| error.to_string()));

fn worn_suit() -> Result<&'static Worn, Box<dyn Error>> {
    SUIT.as_ref().map_err(|error| error.clone().into())
}

fn written_factor(bytes: &[u8], name: &str) -> Result<[f32; 4], Box<dyn Error>> {
    let glb = check_glb(bytes)?;
    Ok(glb
        .document
        .materials()
        .find(|material| material.name() == Some(name))
        .ok_or_else(|| format!("the {name} material is written"))?
        .pbr_metallic_roughness()
        .base_color_factor())
}

fn wear_suit() -> Result<Worn, Box<dyn Error>> {
    let fixture = fixture(armour_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("TEST_Suit")?;
    let pieces = vec![Piece::wearable(resolver.wearable(&record)?)];
    let models = Lashes {
        models: ArchiveModels::new(&archive),
        library: MaterialStatus::Loaded(MaterialLibrary::parse("test", LIBRARY.as_bytes())?),
    };
    let armour = Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?;
    let mut bytes = Vec::new();
    armour.write_glb(&GlbOptions::default(), &mut bytes)?;
    Ok((armour, bytes))
}

#[test]
fn a_piece_without_a_look_is_written_with_the_colour_of_its_hair_parameters() -> TestResult {
    let (armour, bytes) = worn_suit()?;

    let lash = diffuse_of(armour, PartOrigin::Piece, "lash")?;
    assert!(
        near(&lash, [0.45, 0.0, 0.05]),
        "{lash:?}: blond, tinted by the clamped tint"
    );
    let written = written_factor(bytes, "lash")?;
    assert!(near(&written, [0.45, 0.0, 0.05]), "{written:?}");
    Ok(())
}

#[test]
fn a_piece_that_is_no_character_item_has_its_hair_material_coloured_too() -> TestResult {
    let (armour, bytes) = worn_suit()?;

    let body = diffuse_of(armour, PartOrigin::Body, "lash")?;
    let plain = diffuse_of(armour, PartOrigin::Body, "plain")?;

    assert!(near(&body, [0.45, 0.0, 0.05]), "{body:?}: the hair colour");
    assert_eq!(plain, [0.5; 3], "other shaders stay");
    let written = written_factor(bytes, "lash")?;
    assert!(near(&written, body), "{written:?} is the library colour");
    let written = written_factor(bytes, "plain")?;
    assert!(near(&written, [0.5; 3]), "{written:?}");
    Ok(())
}
