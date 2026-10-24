// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::Dna;

use super::piece::{Note, Source};
use super::wearable::Unfit;
use super::{BodyVariant, Expressions, Look, PartOrigin, Piece};
use crate::{Issue, IssueKind};

/// One skin to read and wear.
pub(super) struct Job {
    pub(super) origin: PartOrigin,
    pub(super) part: String,
    pub(super) owner: String,
    pub(super) label: String,
    pub(super) path: String,
    pub(super) material: Option<String>,
    pub(super) shape: Option<Dna>,
    pub(super) expressions: Expressions,
    pub(super) look: Option<Look>,
    pub(super) follows_head: bool,
}

impl Job {
    /// Returns the job of the default body skin at `path`.
    pub(super) fn body(path: &str) -> Self {
        Self {
            origin: PartOrigin::Body,
            part: "body".to_owned(),
            owner: String::new(),
            label: "body".to_owned(),
            path: path.to_owned(),
            material: None,
            shape: None,
            expressions: Expressions::None,
            look: None,
            follows_head: false,
        }
    }
}

pub(super) fn issue(job: &Job, kind: IssueKind) -> Issue {
    Issue {
        path: job.owner.clone(),
        name: job.label.clone(),
        kind,
    }
}

pub(super) fn piece_issue(piece: &Piece, kind: IssueKind) -> Issue {
    Issue {
        path: String::new(),
        name: piece.name.clone(),
        kind,
    }
}

fn note_kind(note: &Note) -> IssueKind {
    match note {
        Note::UnknownMaterial(material) => IssueKind::MaterialChoiceUnknown {
            material: material.clone(),
        },
        Note::LookIgnored(what) => IssueKind::LookIgnored { what: what.clone() },
    }
}

/// Adds the skin `piece` contributes for `variant` to `jobs`, or the reason it contributes none
/// to `issues`; the notes of the piece go to `issues` in either case.
pub(super) fn gather(
    piece: &Piece,
    variant: BodyVariant,
    jobs: &mut Vec<Job>,
    issues: &mut Vec<Issue>,
) {
    issues.extend(
        piece
            .notes
            .iter()
            .map(|note| piece_issue(piece, note_kind(note))),
    );
    let (path, material) = match &piece.source {
        Source::Failed(reason) => {
            let kind = IssueKind::PieceNotFound {
                reason: reason.clone(),
            };
            issues.push(piece_issue(piece, kind));
            return;
        }
        Source::File { path, material } => (path.as_str(), material.as_deref()),
        Source::Wearable(wearable) => match wearable.geometry_for(variant) {
            Ok(geometry) => (geometry.path(), geometry.material()),
            Err(Unfit::NoGeometry) => {
                issues.push(piece_issue(piece, IssueKind::NoWornGeometry));
                return;
            }
            Err(Unfit::NoVariant) => {
                let body = variant.code().to_owned();
                issues.push(piece_issue(piece, IssueKind::NoVariantGeometry { body }));
                return;
            }
        },
    };
    jobs.push(Job {
        origin: PartOrigin::Piece,
        part: piece.name.clone(),
        owner: String::new(),
        label: piece.name.clone(),
        path: path.to_owned(),
        material: piece.material.as_deref().or(material).map(str::to_owned),
        shape: piece.shape.clone(),
        expressions: piece.expressions.clone(),
        look: piece.look.clone(),
        follows_head: piece.follows_head,
    });
}
