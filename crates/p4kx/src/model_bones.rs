// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::model::Model;

use crate::model_text::printable;

const NONE: &str = "-";

/// Writes the table of the bones of `model` after a blank line; nothing for a model without bones.
pub(crate) fn write_bones(out: &mut impl Write, model: &Model) -> io::Result<()> {
    if model.bones.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    writeln!(
        out,
        "{:>5}  {:<6}  {:<4}  {:>8}  {:>8}  {:>8}  Name",
        "Bone", "Parent", "Node", "X", "Y", "Z"
    )?;
    for (index, bone) in model.bones.iter().enumerate() {
        let parent = bone
            .parent
            .map_or_else(|| NONE.to_owned(), |parent| parent.to_string());
        let node = bone
            .node
            .map_or_else(|| NONE.to_owned(), |node| node.to_string());
        let [x, y, z] = bone.world.translation;
        let name = printable(if bone.name.is_empty() {
            NONE
        } else {
            &bone.name
        });
        writeln!(
            out,
            "{index:>5}  {parent:<6}  {node:<4}  {x:>8.3}  {y:>8.3}  {z:>8.3}  {name}"
        )?;
    }
    Ok(())
}
