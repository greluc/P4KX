// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use p4kx_dataforge::StructValue;
use p4kx_model::Model;

use super::uncover::uncover;

use super::jobs::{Job, issue};
use super::{BodyVariant, PartOrigin};
use crate::read::{struct_items, text, unsigned, unsigned_list};
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

    /// Returns whether the Record makes the zone depend on conditions (the ports that hold
    /// items), which are not evaluated.
    #[must_use]
    pub const fn conditional(&self) -> bool {
        self.conditional
    }

    fn covers_body(&self) -> bool {
        self.layer > BODY_LAYER && !self.conditional && !self.visible_layers.contains(&BODY_LAYER)
    }
}

/// The zones an Entity covers, at most [`MAX_CHUNKS`] of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ZoneList {
    pub(crate) chunks: Arc<[ZoneChunk]>,
    pub(crate) truncated: bool,
}

/// Reads the zones the `SCItemClothingParams` among `components` cover.
pub(crate) fn read_zones(components: &[&StructValue<'_>]) -> ZoneList {
    let mut chunks: Vec<ZoneChunk> = components
        .iter()
        .filter(|component| component.type_name() == CLOTHING)
        .flat_map(|component| struct_items(component, "Chunks"))
        .map(|chunk| ZoneChunk {
            zone: text(chunk, "MeshChunk").to_ascii_lowercase(),
            layer: unsigned(chunk, "Layer").unwrap_or(BODY_LAYER),
            visible_layers: unsigned_list(chunk, "VisibleLayers"),
            conditional: struct_items(chunk, "VisibilityConditions").next().is_some(),
        })
        .filter(|chunk| !chunk.zone.is_empty())
        .take(MAX_CHUNKS + 1)
        .collect();
    let truncated = chunks.len() > MAX_CHUNKS;
    chunks.truncate(MAX_CHUNKS);
    ZoneList {
        chunks: chunks.into(),
        truncated,
    }
}

/// Returns the zones of the body that some zone of `chunks` covers: those a chunk above the body
/// names without letting the body show through it.
pub(crate) fn covered<'c>(chunks: impl IntoIterator<Item = &'c ZoneChunk>) -> BTreeSet<&'c str> {
    chunks
        .into_iter()
        .filter(|chunk| chunk.covers_body())
        .map(|chunk| chunk.zone.as_str())
        .collect()
}

/// Returns the model of `job` as it is worn: the body without the zones that clothing covers,
/// reported in `issues`; any other model as it is.
pub(super) fn without_covered(
    job: &Job,
    model: Arc<Model>,
    variant: BodyVariant,
    covered: &BTreeSet<&str>,
    issues: &mut Vec<Issue>,
) -> Arc<Model> {
    if job.origin != PartOrigin::Body {
        return model;
    }
    let (model, kind) = uncovered(model, variant, covered);
    issues.extend(kind.map(|kind| issue(job, kind)));
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
    use super::{ZoneChunk, covered};

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
}
