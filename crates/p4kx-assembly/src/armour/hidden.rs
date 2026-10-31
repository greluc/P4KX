// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Pieces that another worn piece hides: the items at the ports its `HiddenParts` names.

use super::Piece;
use super::jobs::piece_issue;
use crate::{Issue, IssueKind};

/// Returns the first other piece whose Record hides the port the piece at `index` sits at, if it
/// sits at one.
///
/// Every piece hides by its Record, whether it is hidden itself or not.
fn hider(pieces: &[Piece], index: usize) -> Option<&Piece> {
    let port = pieces.get(index)?.port()?;
    pieces
        .iter()
        .enumerate()
        .find(|(other, candidate)| *other != index && candidate.hides(port))
        .map(|(_, candidate)| candidate)
}

/// Returns, per piece, whether it is worn, and lists each hidden piece in `issues`.
pub(super) fn worn(pieces: &[Piece], issues: &mut Vec<Issue>) -> Vec<bool> {
    pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let (Some(port), Some(by)) = (piece.port(), hider(pieces, index)) else {
                return true;
            };
            let kind = IssueKind::PartHidden {
                port: port.to_owned(),
                by: by.name().to_owned(),
            };
            issues.push(piece_issue(piece, kind));
            false
        })
        .collect()
}
