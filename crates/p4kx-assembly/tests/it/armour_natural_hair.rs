// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Hair materials of pieces without a Character's material choices: they are drawn in the colour
//! of their own shader parameters, in the library and in the written GLB.

use std::error::Error;

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
    <Material Name="body_hair" Shader="HairPBR" Diffuse="1,1,1">
      <PublicParams BaseMelanin="1" BaseMelaninRedness="0" BaseTintColor="1,1,1"/>
    </Material>
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

fn wear_suit() -> Result<(Armour, Vec<u8>), Box<dyn Error>> {
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
    let (armour, bytes) = wear_suit()?;

    let lash = diffuse_of(&armour, PartOrigin::Piece, "lash")?;
    assert!(
        near(&lash, [0.45, 0.0, 0.05]),
        "{lash:?}: blond, tinted by the clamped tint"
    );
    let glb = check_glb(&bytes)?;
    let written = glb
        .document
        .materials()
        .find(|material| material.name() == Some("lash"))
        .ok_or("the lash material is written")?
        .pbr_metallic_roughness()
        .base_color_factor();
    assert!(near(&written, [0.45, 0.0, 0.05]), "{written:?}");
    Ok(())
}

#[test]
fn a_piece_that_is_no_character_item_has_its_hair_material_coloured_too() -> TestResult {
    let (armour, _) = wear_suit()?;

    let body = diffuse_of(&armour, PartOrigin::Body, "body_hair")?;
    let plain = diffuse_of(&armour, PartOrigin::Body, "plain")?;

    assert!(
        body.iter().all(|channel| *channel < 0.05),
        "{body:?}: black"
    );
    assert_eq!(plain, [0.5; 3], "other shaders stay");
    Ok(())
}
