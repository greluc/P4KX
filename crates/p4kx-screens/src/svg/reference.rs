// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::Affine;
use roxmltree::Node;

use super::number::{self, number_list};
use super::style::Style;
use super::{SvgError, Walk, transform, visit};

pub(super) fn reference(
    walk: &mut Walk<'_, '_>,
    node: Node<'_, '_>,
    style: &Style,
    total: &Affine,
    depth: usize,
) -> Result<(), SvgError> {
    let target = node
        .attribute(("http://www.w3.org/1999/xlink", "href"))
        .or_else(|| node.attribute("href"))
        .and_then(|link| link.strip_prefix('#'))
        .and_then(|id| walk.ids.get(id).copied());
    let Some(target) = target else {
        walk.note("a use element of another file or of nothing is left out");
        return Ok(());
    };
    if target.tag_name().name() == "symbol" {
        return symbol(walk, node, target, style, total, depth);
    }
    let shift = |name: &str| {
        node.attribute(name)
            .and_then(number::leading_number)
            .unwrap_or(0.0)
    };
    let moved = total.then(&transform::matrix(
        1.0,
        0.0,
        0.0,
        1.0,
        shift("x"),
        shift("y"),
    ));
    visit(walk, target, style, &moved, depth + 1)
}

/// Draws what a `use` element shows of a `symbol`: its children, with the `viewBox` of the symbol
/// fitted to the size of the `use`.
fn symbol(
    walk: &mut Walk<'_, '_>,
    user: Node<'_, '_>,
    target: Node<'_, '_>,
    style: &Style,
    total: &Affine,
    depth: usize,
) -> Result<(), SvgError> {
    let length = |name: &str| user.attribute(name).and_then(number::leading_number);
    let moved = total.then(&transform::matrix(
        1.0,
        0.0,
        0.0,
        1.0,
        length("x").unwrap_or(0.0),
        length("y").unwrap_or(0.0),
    ));
    let fitted = match target
        .attribute("viewBox")
        .and_then(|text| number_list(text, 4))
        .as_deref()
    {
        Some(&[x, y, w, h]) if w > 0.0 && h > 0.0 => {
            let sx = length("width").unwrap_or(w) / w;
            let sy = length("height").unwrap_or(h) / h;
            moved.then(&transform::matrix(sx, 0.0, 0.0, sy, -x * sx, -y * sy))
        }
        _ => moved,
    };
    for child in target.children().filter(Node::is_element) {
        visit(walk, child, style, &fitted, depth + 1)?;
    }
    Ok(())
}
