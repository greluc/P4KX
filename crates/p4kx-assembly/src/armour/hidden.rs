// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Pieces that another worn piece hides: the items at the ports its `HiddenParts` names.

use super::Piece;
use super::jobs::piece_issue;
use super::piece::Source;
use crate::{Issue, IssueKind};

/// What decides whether a piece is hidden.
struct Facts<'p> {
    name: &'p str,
    port: Option<&'p str>,
    /// The name of the piece that holds the port this piece is mounted on.
    holder: Option<&'p str>,
    hides: &'p [String],
}

impl<'p> Facts<'p> {
    fn of(piece: &'p Piece) -> Self {
        let hides = match &piece.source {
            Source::Wearable(wearable) => wearable.hidden_ports(),
            _ => &[],
        };
        Self {
            name: piece.name(),
            port: piece.port(),
            holder: piece
                .mount
                .as_ref()
                .and_then(|mount| mount.holder.as_deref()),
            hides,
        }
    }

    fn hides_port(&self, port: &str) -> bool {
        self.hides
            .iter()
            .any(|hidden| hidden.eq_ignore_ascii_case(port))
    }
}

/// Returns the piece that hides the piece at `index` as an index into `facts`: of the other pieces
/// whose Record hides the port it sits at, the one with the smallest name.
fn direct(facts: &[Facts<'_>], index: usize) -> Option<usize> {
    let port = facts.get(index)?.port?;
    facts
        .iter()
        .enumerate()
        .filter(|(other, candidate)| *other != index && candidate.hides_port(port))
        .min_by_key(|(_, candidate)| candidate.name)
        .map(|(other, _)| other)
}

/// Returns, per piece, the piece that hides it.
///
/// Every piece hides by its Record, whether it is hidden itself or not: the census of LIVE shows
/// no port type that hides another in a cycle and no armour port that is ever hidden, and the
/// result does not depend on the order of the pieces, even for a cycle, which hides both pieces.
/// A piece mounted on a hidden holder is hidden by whatever hides the holder.
fn hidden_by(facts: &[Facts<'_>]) -> Vec<Option<usize>> {
    let mut by: Vec<Option<usize>> = (0..facts.len()).map(|at| direct(facts, at)).collect();
    // Each pass hides at least one more piece or stops, so at most `facts.len()` passes run.
    loop {
        let mut changed = false;
        for at in 0..facts.len() {
            if by.get(at).is_some_and(Option::is_some) {
                continue;
            }
            if let Some(hider) = through_holder(facts, &by, at)
                && let Some(slot) = by.get_mut(at)
            {
                *slot = Some(hider);
                changed = true;
            }
        }
        if !changed {
            return by;
        }
    }
}

/// Returns the piece that hides the holder of the piece at `at`, if it has a hidden holder.
fn through_holder(facts: &[Facts<'_>], by: &[Option<usize>], at: usize) -> Option<usize> {
    let name = facts.get(at)?.holder?;
    let holder = facts.iter().position(|candidate| candidate.name == name)?;
    *by.get(holder)?
}

/// Returns, per piece, whether it is worn, and lists each hidden piece in `issues`.
pub(super) fn worn(pieces: &[Piece], issues: &mut Vec<Issue>) -> Vec<bool> {
    let facts: Vec<Facts<'_>> = pieces.iter().map(Facts::of).collect();
    hidden_by(&facts)
        .into_iter()
        .zip(pieces)
        .map(|(hider, piece)| {
            let Some(hider) = hider.and_then(|at| pieces.get(at)) else {
                return true;
            };
            let kind = IssueKind::PartHidden {
                port: piece.port().unwrap_or_default().to_owned(),
                by: hider.name().to_owned(),
            };
            issues.push(piece_issue(piece, kind));
            false
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Facts, hidden_by};

    fn hides(ports: &[&str]) -> Vec<String> {
        ports.iter().map(|port| (*port).to_owned()).collect()
    }

    fn facts<'p>(
        name: &'p str,
        port: &'p str,
        holder: Option<&'p str>,
        hides: &'p [String],
    ) -> Facts<'p> {
        Facts {
            name,
            port: Some(port),
            holder,
            hides,
        }
    }

    #[test]
    fn a_chain_hides_every_link_whether_the_hider_is_hidden_or_not() {
        let (a, b) = (hides(&["b"]), hides(&["c"]));
        let none = hides(&[]);
        let pieces = [
            facts("a", "a", None, &a),
            facts("b", "b", None, &b),
            facts("c", "c", None, &none),
        ];
        assert_eq!(hidden_by(&pieces), [None, Some(0), Some(1)]);
    }

    #[test]
    fn a_cycle_hides_both_pieces_in_any_order() {
        let (a, b) = (hides(&["B"]), hides(&["a"]));
        let forward = [facts("a", "a", None, &a), facts("b", "b", None, &b)];
        assert_eq!(hidden_by(&forward), [Some(1), Some(0)]);
        let backward = [facts("b", "b", None, &b), facts("a", "a", None, &a)];
        assert_eq!(hidden_by(&backward), [Some(1), Some(0)]);
    }

    #[test]
    fn a_piece_does_not_hide_itself() {
        let own = hides(&["a"]);
        assert_eq!(hidden_by(&[facts("a", "a", None, &own)]), [None]);
    }

    #[test]
    fn the_smallest_name_among_the_hiders_is_reported() {
        let hat = hides(&["hair"]);
        let none = hides(&[]);
        let pieces = [
            facts("zeta", "z", None, &hat),
            facts("alpha", "x", None, &hat),
            facts("hair", "hair", None, &none),
        ];
        assert_eq!(hidden_by(&pieces)[2], Some(1));
    }

    #[test]
    fn a_piece_mounted_on_a_hidden_holder_is_hidden_by_its_hider() {
        let jacket = hides(&["shirt"]);
        let none = hides(&[]);
        let pieces = [
            facts("jacket", "jacket", None, &jacket),
            facts("shirt", "shirt", None, &none),
            facts("pouch", "pouch", Some("shirt"), &none),
            facts("strap", "strap", Some("pouch"), &none),
            facts("belt", "belt", Some("jacket"), &none),
        ];
        assert_eq!(hidden_by(&pieces), [None, Some(0), Some(0), Some(0), None]);
    }
}
