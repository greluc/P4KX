// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Property tests: every Character and every container round-trips byte-exactly, and corrupt
//! input never panics.

use p4kx_character::{
    Character, Chf, ColorParam, Dna, DnaBlend, FloatParam, Guid, ItemPort, Material, NameTable,
    SubMaterial, Texture, Versions,
};
use p4kx_testkit::TestChf;
use proptest::collection::vec;
use proptest::prelude::*;

fn guid() -> impl Strategy<Value = Guid> {
    any::<[u8; 16]>().prop_map(Guid::from_stored)
}

fn dna() -> impl Strategy<Value = Dna> {
    (0..=14_usize, 0..=5_usize).prop_flat_map(|(parts, per_part)| {
        let blend =
            (any::<u16>(), any::<u16>()).prop_map(|(value, head_id)| DnaBlend { value, head_id });
        (
            any::<u32>(),
            any::<u32>(),
            any::<u16>(),
            any::<u16>(),
            vec(vec(blend, per_part), parts),
        )
            .prop_map(
                move |(gender_hash, variant_hash, header_unknown, max_head_id, face_parts)| Dna {
                    gender_hash,
                    variant_hash,
                    blends_per_part: u16::try_from(per_part).unwrap_or_default(),
                    header_unknown,
                    max_head_id,
                    face_parts,
                },
            )
    })
}

fn item_ports() -> impl Strategy<Value = ItemPort> {
    let leaf = (any::<u32>(), guid(), any::<u32>()).prop_map(|(port, item, tail_count)| ItemPort {
        port,
        item,
        tail_count,
        children: Vec::new(),
    });
    leaf.prop_recursive(3, 12, 3, |inner| {
        (any::<u32>(), guid(), any::<u32>(), vec(inner, 0..=3)).prop_map(
            |(port, item, tail_count, children)| ItemPort {
                port,
                item,
                tail_count,
                children,
            },
        )
    })
}

fn sub_material() -> impl Strategy<Value = SubMaterial> {
    (
        any::<u32>(),
        vec(
            (any::<u8>(), guid()).prop_map(|(slot, texture)| Texture { slot, texture }),
            0..=2,
        ),
        vec(
            (any::<u32>(), any::<u32>()).prop_map(|(name, bits)| FloatParam {
                name,
                value: f32::from_bits(bits),
            }),
            0..=3,
        ),
        vec(
            (any::<u32>(), any::<[u8; 4]>()).prop_map(|(name, rgba)| ColorParam { name, rgba }),
            0..=2,
        ),
    )
        .prop_map(|(name, textures, floats, colors)| SubMaterial {
            name,
            textures,
            floats,
            colors,
        })
}

fn material() -> impl Strategy<Value = Material> {
    (
        any::<u32>(),
        guid(),
        any::<u32>(),
        vec(sub_material(), 0..=3),
    )
        .prop_map(
            |(attachment, base_material, flags, sub_materials)| Material {
                attachment,
                base_material,
                flags,
                sub_materials,
            },
        )
}

fn character() -> impl Strategy<Value = Character> {
    (
        prop_oneof![Just(7_u32), Just(8_u32)],
        guid(),
        guid(),
        proptest::option::of(dna()),
        proptest::option::of(item_ports()),
        any::<u32>(),
        vec(material(), 0..=3),
    )
        .prop_map(
            |(male, body_type, voice, dna, item_ports, item_port_tail, materials)| Character {
                versions: Versions { female: 2, male },
                body_type,
                voice,
                dna,
                item_ports,
                item_port_tail,
                materials,
            },
        )
}

proptest! {
    #[test]
    fn every_character_is_written_and_read_back_unchanged(character in character()) {
        let bytes = Chf::new(character.clone()).write().map_err(|error| TestCaseError::fail(error.to_string()))?;

        let chf = Chf::read(&bytes).map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert_eq!(&chf.character, &character);
        prop_assert_eq!(chf.write().ok(), Some(bytes));
    }

    #[test]
    fn from_json_of_to_json_gives_identical_bytes(
        character in character(),
        flags in any::<u16>(),
        padding in vec(any::<u8>(), 0..=600),
    ) {
        let body = character.write().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let bytes = TestChf { flags, padding, ..TestChf::v7() }.with_body(&body);
        let chf = Chf::read(&bytes).map_err(|error| TestCaseError::fail(error.to_string()))?;

        let json = chf.to_json().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let written = Chf::from_json(json.as_bytes()).and_then(|chf| chf.write());

        prop_assert_eq!(written.ok(), Some(bytes));
    }

    #[test]
    fn from_json_of_the_named_json_gives_identical_bytes(
        character in character(),
    ) {
        let body = character.write().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let bytes = TestChf::v7().with_body(&body);
        let chf = Chf::read(&bytes).map_err(|error| TestCaseError::fail(error.to_string()))?;

        let (json, _) = chf.to_json_named(&NameTable::builtin()).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let written = Chf::from_json(json.as_bytes()).and_then(|chf| chf.write());

        prop_assert_eq!(written.ok(), Some(bytes));
    }

    #[test]
    fn a_corrupt_body_never_panics(
        character in character(),
        edits in vec((any::<prop::sample::Index>(), any::<u8>()), 1..=8),
        cut in any::<prop::sample::Index>(),
    ) {
        let mut body = character.write().map_err(|error| TestCaseError::fail(error.to_string()))?;
        for (at, value) in edits {
            let index = at.index(body.len());
            if let Some(byte) = body.get_mut(index) {
                *byte = value;
            }
        }
        body.truncate(cut.index(body.len() + 1));

        let read = Chf::read(&TestChf::v7().with_body(&body));

        if let Ok(chf) = read {
            prop_assert_eq!(chf.character.write().ok(), Some(body));
        }
    }

    #[test]
    fn corrupt_files_and_json_never_panic(bytes in vec(any::<u8>(), 0..=4200), text in ".{0,200}") {
        let _file = Chf::read(&bytes);
        let _json = Chf::from_json(text.as_bytes());
    }
}
