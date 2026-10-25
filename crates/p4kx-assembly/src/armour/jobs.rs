// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::Dna;

use super::head::rig_part;
use super::piece::{Bespoke, Note, Source};
use super::wearable::Unfit;
use super::{BodyVariant, Expressions, Look, PartOrigin, Piece};
use crate::{Issue, IssueKind, MaterialVariant, ModelFailure, ModelProvider};

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
    pub(super) bespoke: bool,
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
            bespoke: false,
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

/// Returns the skin of the story character's own head that stands for the generic skin at
/// `path`, if the Archive has one: the file `<prefix>_<part>.skin` in the folder of `bespoke`.
fn bespoke_path(path: &str, bespoke: &Bespoke, models: &dyn ModelProvider) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next()?;
    let (stem, extension) = name.rsplit_once('.')?;
    if !extension.eq_ignore_ascii_case("skin") {
        return None;
    }
    let part = rig_part(stem)?;
    let candidate = format!("{}/{}_{part}.skin", bespoke.folder, bespoke.prefix);
    match models.file(&candidate, 0) {
        Err(ModelFailure::Missing) => None,
        _ => Some(candidate),
    }
}

/// Adds the skin `piece` contributes for `variant` to `jobs`, or the reason it contributes none
/// to `issues`; the notes of the piece go to `issues` in either case.
pub(super) fn gather(
    piece: &Piece,
    variant: BodyVariant,
    models: &dyn ModelProvider,
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
        Source::File { path, material } => (path.clone(), material.clone()),
        Source::Wearable(wearable) => match wearable.geometry_selected(variant, &piece.selecting) {
            Ok(geometry) => (
                geometry.path().to_owned(),
                chosen_material(piece, wearable, variant, geometry.material()),
            ),
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
    let own = piece
        .bespoke
        .as_ref()
        .and_then(|bespoke| bespoke_path(&path, bespoke, models));
    let bespoke = own.is_some();
    let material = material.filter(|library| {
        piece.bespoke.is_none() || !library.contains(['/', '\\']) || {
            !matches!(models.file(library, 0), Err(ModelFailure::Missing))
        }
    });
    jobs.push(Job {
        origin: PartOrigin::Piece,
        part: piece.name.clone(),
        owner: String::new(),
        label: piece.name.clone(),
        path: own.unwrap_or(path),
        material: piece.material.clone().or(material),
        shape: piece.shape.clone(),
        expressions: piece.expressions.clone(),
        look: piece.look.clone(),
        follows_head: piece.follows_head,
        bespoke,
    });
}

/// Returns the material library of a worn Entity: the one its material tree has for the tags of
/// the piece and the body, else the one of its geometry.
fn chosen_material(
    piece: &Piece,
    wearable: &super::Wearable,
    variant: BodyVariant,
    geometry: Option<&str>,
) -> Option<String> {
    if piece.variant_tags.is_empty() {
        return geometry.map(str::to_owned);
    }
    let mut tags = piece.variant_tags.clone();
    tags.push(variant.tag().to_owned());
    MaterialVariant::resolve(wearable.material_variants(), &tags)
        .or(geometry)
        .map(str::to_owned)
}
