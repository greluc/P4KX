// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use anyhow::Result;
use p4kx_assembly::{
    Assembly, EntityInterior, EntityNode, Interior, InteriorForge, InteriorOptions, InteriorWatch,
};
use p4kx_extraction::{Archive, DataForge, DataForgeEntry};

/// Adds the Interior of the Entity `tree` to `assembly`, with the class Records of the
/// containers' entities looked up in `forge`.
pub(crate) fn with_interior(
    assembly: Assembly,
    (archive, forge): (&Archive, &DataForge<'_>),
    tree: &EntityNode,
    lod: u32,
) -> Result<Assembly> {
    let options = InteriorOptions::default().with_lod(lod);
    let classes = InteriorForge::new(forge);
    let interior = EntityInterior::build_with(
        archive,
        &assembly,
        tree,
        &options,
        &InteriorWatch::none(),
        Some(&classes),
    )?;
    Ok(assembly.with_interior(interior))
}

/// Builds the Interior of the container chain `layers`, the Archive first, with the class Records
/// of the entities looked up in the Archive's DataForge; an Archive without one builds the
/// Interior without.
pub(crate) fn build(
    layers: &[Archive],
    entry: &str,
    options: &InteriorOptions,
) -> Result<Interior> {
    let refs: Vec<&Archive> = layers.iter().collect();
    let forge_entry = match layers.first() {
        Some(outer) if DataForgeEntry::locate(outer).is_some() => {
            Some(DataForgeEntry::read(outer)?)
        }
        _ => None,
    };
    let forge = forge_entry.as_ref().map(DataForgeEntry::open).transpose()?;
    let classes = forge.as_ref().map(InteriorForge::new);
    Ok(Interior::build_with(
        &refs,
        entry,
        options,
        &InteriorWatch::none(),
        classes.as_ref(),
    )?)
}
