// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;
use std::sync::Arc;

use p4kx_dataforge::StructValue;
use p4kx_model::Model;

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

/// Reads the zones the `SCItemClothingParams` among `components` cover; at most
/// [`MAX_CHUNKS`].
pub(crate) fn read_zones(components: &[&StructValue<'_>]) -> Arc<[ZoneChunk]> {
    components
        .iter()
        .filter(|component| component.type_name() == CLOTHING)
        .flat_map(|component| struct_items(component, "Chunks"))
        .take(MAX_CHUNKS)
        .map(|chunk| ZoneChunk {
            zone: text(chunk, "MeshChunk").to_ascii_lowercase(),
            layer: unsigned(chunk, "Layer").unwrap_or(BODY_LAYER),
            visible_layers: unsigned_list(chunk, "VisibleLayers"),
            conditional: struct_items(chunk, "VisibilityConditions").next().is_some(),
        })
        .filter(|chunk| !chunk.zone.is_empty())
        .collect()
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

/// A body without the zones that worn clothing covers.
pub(crate) struct Uncovered {
    pub(crate) model: Model,
    pub(crate) hidden: Vec<&'static str>,
}

/// Returns `body` without the submeshes of the zones in `covered`, or the reason its submeshes
/// cannot be told apart by zone: every level of detail must number its submeshes with exactly
/// the node numbers of the variant's zones. `None` when no zone of the body is covered.
pub(crate) fn uncover(
    body: &Model,
    variant: BodyVariant,
    covered: &BTreeSet<&str>,
) -> Result<Option<Uncovered>, String> {
    let names = variant.zone_names();
    let hidden: Vec<&'static str> = names
        .iter()
        .copied()
        .filter(|zone| covered.contains(*zone))
        .collect();
    if hidden.is_empty() {
        return Ok(None);
    }
    let mut model = body.clone();
    for lod in &mut model.lods {
        for mesh in &mut lod.meshes {
            let present: BTreeSet<u16> = mesh.submeshes.iter().map(|part| part.node_id).collect();
            let fits = present.len() == names.len()
                && present
                    .iter()
                    .enumerate()
                    .all(|(at, node)| usize::from(*node) == at);
            if !fits {
                return Err(format!(
                    "level {} of the body has {} zones numbered {:?}..., not the {} the zone table lists",
                    lod.level,
                    present.len(),
                    present.iter().take(3).collect::<Vec<_>>(),
                    names.len()
                ));
            }
            mesh.submeshes.retain(|part| {
                names
                    .get(usize::from(part.node_id))
                    .is_none_or(|zone| !covered.contains(*zone))
            });
        }
    }
    Ok(Some(Uncovered { model, hidden }))
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
    match uncover(&model, variant, covered) {
        Ok(None) => model,
        Ok(Some(done)) => {
            let zones = done.hidden.iter().map(|zone| (*zone).to_owned()).collect();
            issues.push(issue(job, IssueKind::BodyZonesHidden { zones }));
            Arc::new(done.model)
        }
        Err(reason) => {
            issues.push(issue(job, IssueKind::BodyZonesUnknown { reason }));
            model
        }
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
