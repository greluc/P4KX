// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_character::Dna;

use super::head::rig_part;
use super::piece::{Bespoke, Note, Source};
use super::wearable::Unfit;
use super::{BodyVariant, Expressions, Look, PartOrigin, Piece, ZoneChunk};
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
    pub(super) zones: Arc<[ZoneChunk]>,
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
            zones: Arc::default(),
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
    exists(&candidate, models).then_some(candidate)
}

/// Returns whether the Archive holds the file at `path`, a path below `Data`; a file that cannot
/// be read still exists.
fn exists(path: &str, models: &dyn ModelProvider) -> bool {
    !matches!(models.file(path, 0), Err(ModelFailure::Missing))
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
    let material = existing_library(piece, material, bespoke, models, issues);
    let zones = covered_zones(piece, issues);
    jobs.push(Job {
        zones,
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

/// Returns `material` for a piece of a story character's head, or, when the Archive lacks that
/// library, the head's own library for a part with a skin of its own, and none otherwise; the
/// missing library is reported to `issues`. Other pieces keep `material`.
fn existing_library(
    piece: &Piece,
    material: Option<String>,
    bespoke: bool,
    models: &dyn ModelProvider,
    issues: &mut Vec<Issue>,
) -> Option<String> {
    let (Some(head), Some(library)) = (&piece.bespoke, &material) else {
        return material;
    };
    if !library.contains(['/', '\\']) || exists(library, models) {
        return material;
    }
    let fallback = head.library.clone().filter(|_| bespoke);
    let reason = fallback.as_ref().map_or_else(
        || format!("the material library {library} does not exist"),
        |head_library| {
            format!(
                "the material library {library} does not exist; the library of the head, {head_library}, is used"
            )
        },
    );
    issues.push(piece_issue(piece, IssueKind::MaterialLibrary { reason }));
    fallback
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

/// Returns the skins to wear: the body and what each piece contributes, at most `max_parts` of
/// them; the jobs beyond are listed in `issues`.
pub(super) fn planned(
    variant: BodyVariant,
    pieces: &[Piece],
    models: &dyn ModelProvider,
    max_parts: usize,
    issues: &mut Vec<Issue>,
) -> Vec<Job> {
    let mut jobs = vec![Job::body(variant.skin())];
    for piece in pieces {
        gather(piece, variant, models, &mut jobs, issues);
    }
    if jobs.len() > max_parts {
        for job in jobs.split_off(max_parts) {
            let kind = IssueKind::TooManyParts { limit: max_parts };
            issues.push(issue(&job, kind));
        }
    }
    jobs
}

/// Returns the body zones the piece covers; a Record with more chunks than are read is reported.
fn covered_zones(piece: &Piece, issues: &mut Vec<Issue>) -> Arc<[ZoneChunk]> {
    let Source::Wearable(wearable) = &piece.source else {
        return Arc::default();
    };
    if wearable.zones.truncated {
        let reason = "the Record lists more body zones than are read; the rest are ignored";
        let kind = IssueKind::BodyZonesUnknown(reason.to_owned());
        issues.push(piece_issue(piece, kind));
    }
    Arc::clone(&wearable.zones.chunks)
}
