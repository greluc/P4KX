// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Armour drawn with the tint palette of a worn piece: the one its geometry names, else the
//! default of its material library.

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver,
    PaletteFailure, PaletteProvider, PartOrigin, Piece,
};
use p4kx_dataforge::Guid;
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_model::{Material, PaletteSlot, TintEntry, TintPalette};
use p4kx_testkit::{PAINTED_LIBRARY_PALETTE, PAINTED_SUIT, painted_entries};

use crate::assembly_support::{TestResult, fixture};

const LIBRARY_GUID: [u8; 16] = [7; 16];
const FROM_GEOMETRY: [f32; 3] = [0.03, 0.03, 0.03];
const FROM_LIBRARY: [f32; 3] = [0.8, 0.7, 0.1];

struct Stub {
    geometry_exists: bool,
}

fn palette(name: &str, tint: [f32; 3]) -> TintPalette {
    let entry = TintEntry {
        tint,
        specular: [0.05; 3],
        glossiness: 200.0,
    };
    TintPalette::new(name, [entry; 3], [1.0; 3])
}

impl PaletteProvider for Stub {
    fn by_guid(&self, guid: Guid) -> Result<TintPalette, PaletteFailure> {
        if guid == Guid::from_stored(LIBRARY_GUID) {
            Ok(palette("from_library", FROM_LIBRARY))
        } else if self.geometry_exists {
            Ok(palette("from_geometry", FROM_GEOMETRY))
        } else {
            Err(PaletteFailure::Missing)
        }
    }

    fn guid_at(&self, path: &str) -> Result<Guid, PaletteFailure> {
        if path.eq_ignore_ascii_case(PAINTED_LIBRARY_PALETTE) {
            Ok(Guid::from_stored(LIBRARY_GUID))
        } else {
            Err(PaletteFailure::Missing)
        }
    }
}

fn wear(name: &str, palettes: Option<&Stub>) -> Result<Armour, Box<dyn std::error::Error>> {
    let fixture = fixture(painted_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find(name)?;
    let pieces = vec![Piece::wearable(resolver.wearable(&record)?)];
    let models = ArchiveModels::new(&archive);
    let options = AssemblyOptions::default();
    Ok(match palettes {
        Some(stub) => {
            Armour::build_with_palettes(BodyVariant::Male, &pieces, &models, stub, &options)?
        }
        None => Armour::build(BodyVariant::Male, &pieces, &models, &options)?,
    })
}

fn caps(armour: &Armour) -> Result<Material, &'static str> {
    armour
        .parts()
        .iter()
        .find(|part| part.origin() == PartOrigin::Piece)
        .and_then(|part| part.materials().library())
        .and_then(|library| library.materials.iter().find(|m| m.name == "caps"))
        .cloned()
        .ok_or("the caps material of the piece")
}

#[test]
fn a_piece_takes_the_palette_its_geometry_names() -> TestResult {
    let armour = wear(
        PAINTED_SUIT,
        Some(&Stub {
            geometry_exists: true,
        }),
    )?;

    let caps = caps(&armour)?;

    assert_eq!(caps.palette, Some(PaletteSlot::Primary));
    assert!(caps.tinted, "drawn with the palette");
    assert_eq!(caps.diffuse, FROM_GEOMETRY);
    assert_eq!(
        caps.layers
            .first()
            .and_then(|layer| layer.palette_entry)
            .map(|entry| entry.tint),
        Some(FROM_GEOMETRY)
    );
    assert!(armour.issues().is_empty(), "{:?}", armour.issues());
    Ok(())
}

#[test]
fn a_piece_without_a_palette_of_its_own_takes_the_default_of_its_library() -> TestResult {
    let armour = wear(
        "TEST_Suit",
        Some(&Stub {
            geometry_exists: true,
        }),
    )?;

    assert_eq!(caps(&armour)?.diffuse, FROM_LIBRARY);
    Ok(())
}

#[test]
fn a_palette_that_cannot_be_read_is_listed_and_the_library_default_is_used() -> TestResult {
    let armour = wear(
        PAINTED_SUIT,
        Some(&Stub {
            geometry_exists: false,
        }),
    )?;

    assert_eq!(caps(&armour)?.diffuse, FROM_LIBRARY);
    let listed = armour
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::Palette { .. }))
        .count();
    assert_eq!(listed, 1);
    Ok(())
}

#[test]
fn without_palettes_nothing_is_painted() -> TestResult {
    let armour = wear(PAINTED_SUIT, None)?;

    let caps = caps(&armour)?;

    assert!(!caps.tinted);
    assert_ne!(caps.diffuse, FROM_GEOMETRY);
    Ok(())
}
