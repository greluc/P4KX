// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_assembly::EntityNode;
use serde::Serialize;

use crate::query_table::one_line;

#[derive(Serialize)]
pub(crate) struct BodyJson<'a> {
    variant: &'static str,
    head: &'a str,
}

pub(crate) fn body_json(tree: &EntityNode) -> Option<BodyJson<'_>> {
    tree.character_body().map(|body| BodyJson {
        variant: body.variant().code(),
        head: body.head().class_name(),
    })
}

/// Prints the body the clothing of a character was chosen for and the head that names it; nothing
/// for a tree without a head.
pub(crate) fn print_body(out: &mut impl Write, tree: &EntityNode) -> io::Result<()> {
    body_json(tree).map_or(Ok(()), |body| {
        writeln!(
            out,
            "body {}, named by the head {}",
            body.variant,
            one_line(body.head)
        )
    })
}
