// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Text XML nested deeper than the shared cap is refused by every reader of this crate, before
//! the parser recurses that deep.

use p4kx_cryxml::MAX_TEXT_DEPTH;
use p4kx_model::{MaterialError, MaterialLibrary, ObjectContainer, ObjectContainerError};
use p4kx_model::{SocEntities, SocEntitiesError};

fn nested(root: &str, levels: usize) -> Vec<u8> {
    format!(
        "<{root}>{}{}</{root}>",
        "<a>".repeat(levels),
        "</a>".repeat(levels)
    )
    .into_bytes()
}

#[test]
fn a_material_nested_too_deep_is_a_typed_error() {
    let result = MaterialLibrary::parse("x", &nested("Material", MAX_TEXT_DEPTH));
    assert!(
        matches!(result, Err(MaterialError::TooDeep(_))),
        "{result:?}"
    );
}

#[test]
fn soc_entities_nested_too_deep_are_a_typed_error() {
    let result = SocEntities::parse(&nested("Entities", MAX_TEXT_DEPTH));
    assert!(
        matches!(result, Err(SocEntitiesError::TooDeep(_))),
        "{result:?}"
    );
}

#[test]
fn an_object_container_nested_too_deep_is_a_typed_error() {
    let result = ObjectContainer::parse(&nested("ObjectContainer", MAX_TEXT_DEPTH));
    assert!(
        matches!(result, Err(ObjectContainerError::TooDeep(_))),
        "{result:?}"
    );
}

#[test]
fn a_document_at_the_cap_passes_the_depth_check() {
    let result = MaterialLibrary::parse("x", &nested("Material", MAX_TEXT_DEPTH - 1));
    assert!(
        !matches!(result, Err(MaterialError::TooDeep(_))),
        "{result:?}"
    );
}
