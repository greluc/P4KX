// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForge, Guid, StructValue, TreeBuilder};

use crate::contract_harvest::HarvestableLink;
use crate::guid_text::CatalogGuid;
use crate::memo::Memo;
use crate::record_fields::{class_name, component, reference};

const MINEABLE_PARAMS: &str = "MineableParams";

/// What a `HarvestablePreset` Record says: its name and the entity it spawns.
#[derive(Clone, Copy)]
struct PresetFacts<'f> {
    class_name: Option<&'f str>,
    entity: Option<CatalogGuid>,
}

/// What the `MineableParams` of an entity link to.
#[derive(Clone, Copy)]
struct MiningFacts {
    composition: Option<CatalogGuid>,
    global_params: Option<CatalogGuid>,
}

/// Follows a harvestable from its `HarvestablePreset` Record to the entity and the mining
/// parameters of the entity, each Record read once.
#[derive(Default)]
pub(crate) struct Links<'f> {
    presets: Memo<PresetFacts<'f>>,
    minings: Memo<MiningFacts>,
}

impl<'f> Links<'f> {
    /// Returns what the element `element` spawns, from its References `harvestable` and
    /// `harvestableEntityClass`; `None` when it names neither.
    pub(crate) fn of_element(
        &self,
        trees: &mut TreeBuilder<'f>,
        forge: &'f DataForge<'f>,
        element: &StructValue<'f>,
    ) -> Option<HarvestableLink<'f>> {
        let preset_guid = reference(Some(element), "harvestable");
        let element_entity = reference(Some(element), "harvestableEntityClass");
        if preset_guid.is_none() && element_entity.is_none() {
            return None;
        }
        let preset = preset_guid.map(|guid| self.preset(trees, forge, guid));
        let entity_guid = element_entity.or_else(|| preset.and_then(|facts| facts.entity));
        let mining = entity_guid.map(|guid| self.mining(trees, forge, guid));
        Some(HarvestableLink {
            preset_guid,
            preset_class_name: preset.and_then(|facts| facts.class_name),
            entity_guid,
            mineable_composition_guid: mining.and_then(|facts| facts.composition),
            mining_global_params_guid: mining.and_then(|facts| facts.global_params),
        })
    }

    fn preset(
        &self,
        trees: &mut TreeBuilder<'f>,
        forge: &'f DataForge<'f>,
        guid: CatalogGuid,
    ) -> PresetFacts<'f> {
        self.presets.get_or_make(guid.0, || {
            let tree = record_tree(trees, forge, guid.0);
            PresetFacts {
                class_name: tree.as_ref().map(class_name),
                entity: tree
                    .as_ref()
                    .and_then(|tree| reference(Some(tree.root()), "entityClass")),
            }
        })
    }

    fn mining(
        &self,
        trees: &mut TreeBuilder<'f>,
        forge: &'f DataForge<'f>,
        guid: CatalogGuid,
    ) -> MiningFacts {
        self.minings.get_or_make(guid.0, || {
            let tree = record_tree(trees, forge, guid.0);
            let params = tree
                .as_ref()
                .and_then(|tree| component(tree, MINEABLE_PARAMS));
            MiningFacts {
                composition: reference(params, "composition"),
                global_params: reference(params, "globalParams"),
            }
        })
    }
}

fn record_tree<'f>(
    trees: &mut TreeBuilder<'f>,
    forge: &'f DataForge<'f>,
    guid: Guid,
) -> Option<p4kx_extraction::RecordTree<'f>> {
    let record = forge.record_by_guid(guid)?;
    trees.build(&record).ok()
}
