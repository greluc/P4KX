// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use p4kx_dataforge::StructValue;
use p4kx_model::Model;

use super::uncover::uncover;

use super::jobs::{Job, issue};
use super::{BodyVariant, PartOrigin};
use crate::read::{has_items, struct_items, text, unsigned, unsigned_list};
use crate::{Issue, IssueKind};

const CLOTHING: &str = "SCItemClothingParams";
const MAX_CHUNKS: usize = 256;

/// The layer the body itself is on; clothing is on the layers above.
const BODY_LAYER: u32 = 0;

/// One zone of the body an Entity covers when it is worn (`SCItemClothingMeshChunkParams`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneChunk {
    pub(crate) zone: String,
    pub(crate) layer: u32,
    pub(crate) visible_layers: Vec<u32>,
    pub(crate) conditional: bool,
}

impl ZoneChunk {
    /// Returns the name of the zone, e.g. `torso01_zone`.
    #[must_use]
    pub fn zone(&self) -> &str {
        &self.zone
    }

    /// Returns the layer the Entity covers the zone on: 0 is the body, 1 the clothing next to it.
    #[must_use]
    pub const fn layer(&self) -> u32 {
        self.layer
    }

    /// Returns the layers below this one that stay visible through it.
    #[must_use]
    pub fn visible_layers(&self) -> &[u32] {
        &self.visible_layers
    }

    /// Returns whether the Record makes the zone depend on conditions that are not evaluated:
    /// ports that must hold items, or tags the items must or must not carry. A condition whose
    /// entries name no port and no tag restricts nothing and does not count.
    #[must_use]
    pub const fn conditional(&self) -> bool {
        self.conditional
    }

    fn hides_body(&self) -> bool {
        self.layer > BODY_LAYER && !self.visible_layers.contains(&BODY_LAYER)
    }

    fn covers_body(&self) -> bool {
        self.hides_body() && !self.conditional
    }
}

/// The condition fields of `SCItemClothingPortVisibilityConditionParams` that restrict a chunk.
const RESTRICTIONS: [&str; 4] = ["Ports", "DisableTags", "SkipTags", "ActiveTags"];

/// Returns whether the `VisibilityConditions` of `chunk` restrict it. Only a condition whose
/// `PortVisibility` entries all name no port and no tag restricts nothing; one without entries is
/// not understood and counts as restricting.
fn restricted(chunk: &StructValue<'_>) -> bool {
    struct_items(chunk, "VisibilityConditions").any(|condition| {
        let mut entries = struct_items(condition, "PortVisibility").peekable();
        entries.peek().is_none()
            || entries.any(|entry| RESTRICTIONS.iter().any(|field| has_items(entry, field)))
    })
}

/// What the `SCItemClothingParams` of an Entity say: the body zones it covers and the ports whose
/// items it hides.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ZoneList {
    /// The zones, at most [`MAX_CHUNKS`] of them.
    pub(crate) chunks: Arc<[ZoneChunk]>,
    pub(crate) truncated: bool,
    /// The names of the ports, in lower case, whose items are not drawn when the Entity is worn.
    pub(crate) hidden_ports: Arc<[String]>,
}

/// Reads the zones and hidden ports of the `SCItemClothingParams` among `components`.
pub(crate) fn read_zones(components: &[&StructValue<'_>]) -> ZoneList {
    let clothing = || {
        components
            .iter()
            .filter(|component| component.type_name() == CLOTHING)
    };
    let mut chunks: Vec<ZoneChunk> = clothing()
        .flat_map(|component| struct_items(component, "Chunks"))
        .map(|chunk| ZoneChunk {
            zone: text(chunk, "MeshChunk").to_ascii_lowercase(),
            layer: unsigned(chunk, "Layer").unwrap_or(BODY_LAYER),
            visible_layers: unsigned_list(chunk, "VisibleLayers"),
            conditional: restricted(chunk),
        })
        .filter(|chunk| !chunk.zone.is_empty())
        .take(MAX_CHUNKS + 1)
        .collect();
    let truncated = chunks.len() > MAX_CHUNKS;
    chunks.truncate(MAX_CHUNKS);
    let hidden_ports = clothing()
        .flat_map(|component| struct_items(component, "HiddenParts"))
        .map(|part| text(part, "PortName").to_ascii_lowercase())
        .filter(|port| !port.is_empty())
        .collect();
    ZoneList {
        chunks: chunks.into(),
        truncated,
        hidden_ports,
    }
}

/// Returns the zones of the body that the `chunks` of any worn Entity cover: those a chunk above
/// the body names without letting the body show through it.
pub(crate) fn covered<'c>(chunks: impl IntoIterator<Item = &'c ZoneChunk>) -> BTreeSet<&'c str> {
    chunks
        .into_iter()
        .filter(|chunk| chunk.covers_body())
        .map(|chunk| chunk.zone.as_str())
        .collect()
}

/// Returns the zones of `variant`'s body that a conditional chunk of `chunks` would cover if its
/// conditions held and that no other chunk covers: the zones whose cut depends on conditions that
/// are not evaluated.
pub(crate) fn undecided<'c>(
    chunks: impl IntoIterator<Item = &'c ZoneChunk>,
    variant: BodyVariant,
    covered: &BTreeSet<&str>,
) -> BTreeSet<&'c str> {
    chunks
        .into_iter()
        .filter(|chunk| chunk.hides_body() && chunk.conditional)
        .map(|chunk| chunk.zone.as_str())
        .filter(|zone| {
            variant.zone_names().iter().any(|name| name == zone) && !covered.contains(zone)
        })
        .collect()
}

/// The zones of the body that worn clothing covers, and those it would cover if conditions that
/// are not evaluated held.
pub(super) struct Coverage<'c> {
    pub(super) covered: BTreeSet<&'c str>,
    pub(super) undecided: BTreeSet<&'c str>,
}

impl<'c> Coverage<'c> {
    /// Returns the coverage of the `chunks` of the worn Entities on the body of `variant`.
    pub(super) fn of(
        chunks: impl IntoIterator<Item = &'c ZoneChunk> + Clone,
        variant: BodyVariant,
    ) -> Self {
        let covered = covered(chunks.clone());
        let undecided = undecided(chunks, variant, &covered);
        Self { covered, undecided }
    }
}

/// Returns the model of `job` as it is worn: the body without the zones that clothing covers,
/// reported in `issues` with the zones whose cover is undecided; any other model as it is.
pub(super) fn without_covered(
    job: &Job,
    model: Arc<Model>,
    variant: BodyVariant,
    coverage: &Coverage<'_>,
    issues: &mut Vec<Issue>,
) -> Arc<Model> {
    if job.origin != PartOrigin::Body {
        return model;
    }
    let (model, kind) = uncovered(model, variant, &coverage.covered);
    issues.extend(kind.map(|kind| issue(job, kind)));
    if !coverage.undecided.is_empty() {
        let zones = coverage.undecided.iter().map(|zone| (*zone).to_owned());
        let kind = IssueKind::ClothingConditionsUndecided(zones.collect());
        issues.push(issue(job, kind));
    }
    model
}

/// Returns the body `model` without the zones in `covered`, with the issue that says which were
/// hidden or why none could be; the model as it is when no zone is covered.
pub(crate) fn uncovered(
    model: Arc<Model>,
    variant: BodyVariant,
    covered: &BTreeSet<&str>,
) -> (Arc<Model>, Option<IssueKind>) {
    match uncover(&model, variant, covered) {
        Ok(None) => (model, None),
        Ok(Some(done)) => {
            let zones = done.hidden.iter().map(|zone| (*zone).to_owned()).collect();
            (
                Arc::new(done.model),
                Some(IssueKind::BodyZonesHidden(zones)),
            )
        }
        Err(misfit) => (model, Some(IssueKind::BodyZonesUnknown(misfit.to_string()))),
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyVariant, ZoneChunk, covered, undecided};

    fn chunk(zone: &str, layer: u32, visible: &[u32], conditional: bool) -> ZoneChunk {
        ZoneChunk {
            zone: zone.to_owned(),
            layer,
            visible_layers: visible.to_vec(),
            conditional,
        }
    }

    #[test]
    fn a_chunk_above_the_body_covers_its_zone() {
        let chunks = [chunk("torso01_zone", 1, &[], false)];
        assert_eq!(
            covered(&chunks).into_iter().collect::<Vec<_>>(),
            ["torso01_zone"]
        );
    }

    #[test]
    fn a_chunk_that_shows_the_body_through_covers_nothing() {
        let chunks = [chunk("l_arm04_zone", 1, &[0], false)];
        assert!(covered(&chunks).is_empty());
    }

    #[test]
    fn the_body_is_covered_when_any_chunk_above_it_hides_it() {
        let chunks = [
            chunk("torso01_zone", 1, &[], false),
            chunk("torso01_zone", 2, &[0, 1], false),
        ];
        assert_eq!(covered(&chunks).len(), 1);
    }

    #[test]
    fn chunks_on_the_body_layer_and_conditional_ones_cover_nothing() {
        let chunks = [
            chunk("torso01_zone", 0, &[], false),
            chunk("torso02_zone", 2, &[], true),
        ];
        assert!(covered(&chunks).is_empty());
    }

    #[test]
    fn a_conditional_chunk_on_a_body_zone_is_undecided_unless_another_covers_it() {
        let chunks = [
            chunk("hips_zone", 2, &[], true),
            chunk("torso02_zone", 2, &[], true),
            chunk("torso01_zone", 2, &[], true),
            chunk("torso01_zone", 1, &[], false),
            chunk("theta_core_zone", 2, &[], true),
            chunk("l_arm01_zone", 2, &[0], true),
        ];
        let covered = covered(&chunks);
        let open = undecided(&chunks, BodyVariant::Male, &covered);
        assert_eq!(open.into_iter().collect::<Vec<_>>(), ["torso02_zone"]);
    }
}
