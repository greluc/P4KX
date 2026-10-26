// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{ArchiveFiles, Entities};
use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, Filter, Guid, Record, RecordTree, TreeBuilder,
};
use rayon::prelude::*;

use crate::collected::Picked;
use crate::commodity::{resource_commodity, resource_type_guid};
use crate::contract::Blueprint;
use crate::contract_recipe::CostLine;
use crate::entity_selection::{Selected, select};
use crate::playability_rules::Evidence;
use crate::resource_group::{Groups, Listing, listing};
use crate::ship_pool::ShipPool;
use crate::starmap::starmap_object;
use crate::starmap_containers::Placements;
use crate::texts::Texts;
use crate::{CatalogError, CatalogFailure, CatalogOptions, CatalogPhase, CatalogProgress, Kind};
use crate::{item_stats, item_variant};

const PROGRESS_STEP: usize = 1024;
const RESOURCE_TYPE: &str = "ResourceType";

/// Returns whether the `resourceGroups` kind selects `record` under `filter`; a Record whose path
/// or type cannot be read is not selected here, the check of the Record reports it.
fn selects_group(record: &Record<'_>, filter: Option<&Filter>) -> bool {
    matches!(
        select(record, filter),
        Ok(Some(Selected::Entity(Kind::ResourceGroups)))
    )
}

/// Appends to `out` the `ResourceType` GUID of every resource line of `line`, selections opened.
fn resources_of(line: &CostLine<'_>, out: &mut Vec<Guid>) {
    out.extend(line.resource_guid.map(|guid| guid.0));
    for option in &line.options {
        resources_of(option, out);
    }
}

/// The parallel check of every Record, with its progress.
pub(crate) struct Scan<'o, 'a, 'f> {
    forge: &'f DataForge<'f>,
    ships: ShipPool<'f>,
    options: &'o CatalogOptions<'a>,
    texts: &'o Texts<'f>,
    placements: &'o Placements,
    groups: Groups,
    evidence: Evidence,
    done: AtomicUsize,
    total: usize,
}

impl<'o, 'a, 'f> Scan<'o, 'a, 'f> {
    /// Checks every Record of `forge` in parallel and returns those a kind selected; a ship's base
    /// data is read here, once, by the worker that checks it.
    pub(crate) fn run(
        forge: &'f DataForge<'f>,
        files: &'f ArchiveFiles<'f>,
        entities: Arc<Entities<'f>>,
        options: &'o CatalogOptions<'a>,
        texts: &'o Texts<'f>,
        blueprints: &[Blueprint<'f>],
        placements: &'o Placements,
    ) -> Result<Vec<Picked<'f>>, CatalogError> {
        let records: Vec<Record<'f>> = forge.records().collect();
        let (groups, group_failures) = Self::groups(&records, options.filter);
        let (evidence, evidence_failures) = Evidence::read(&records);
        let scan = Self {
            forge,
            ships: ShipPool::new(entities, files),
            options,
            texts,
            placements,
            groups,
            evidence,
            done: AtomicUsize::new(0),
            total: records.len(),
        };
        let mut picked: Vec<Picked<'f>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| scan.check(trees, record))
            .filter_map(Result::transpose)
            .collect::<Result<_, _>>()?;
        picked.extend(
            group_failures
                .into_iter()
                .chain(evidence_failures)
                .map(Picked::Failed),
        );
        let resources = scan.resource_commodities(&picked, blueprints);
        picked.extend(resources);
        Ok(picked)
    }

    /// Returns what the `ResourceTypeGroup` Records of the `resourceGroups` kind list, the first
    /// group in Record order winning a `ResourceType` or group several list, and the groups that
    /// cannot be read and that the kind, under the Filter, does not report itself.
    fn groups(records: &[Record<'f>], filter: Option<&Filter>) -> (Groups, Vec<CatalogFailure>) {
        let listed: Vec<Result<Option<Listing>, CatalogFailure>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                if !selects_group(record, None) {
                    return Ok(None);
                }
                match trees.build(record) {
                    Ok(tree) => Ok(Some(listing(&tree))),
                    Err(_) if selects_group(record, filter) => Ok(None),
                    Err(source) => Err(CatalogFailure::of(record, &source)),
                }
            })
            .collect();
        let mut groups = Groups::default();
        let mut failures = Vec::new();
        for result in listed {
            match result {
                Ok(Some(listing)) => groups.add(listing),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }
        (groups, failures)
    }

    /// Returns a commodity for every `ResourceType` the ingredients of the picked blueprints use
    /// that no picked commodity carries, those the Filter matches; one that cannot be read
    /// becomes a failure.
    fn resource_commodities(
        &self,
        picked: &[Picked<'f>],
        blueprints: &[Blueprint<'f>],
    ) -> Vec<Picked<'f>> {
        let mut carried = HashSet::new();
        for pick in picked {
            if let Picked::Commodity(commodity) = pick {
                carried.extend(commodity.subtype_guid.map(|guid| guid.0));
            }
        }
        let mut used = Vec::new();
        for tier in blueprints.iter().flat_map(|blueprint| &blueprint.tiers) {
            for line in tier.mandatory.iter().chain(&tier.optional) {
                resources_of(line, &mut used);
            }
        }
        used.sort_unstable();
        used.dedup();
        let mut trees = TreeBuilder::new();
        let mut found = Vec::new();
        for guid in used.into_iter().filter(|guid| !carried.contains(guid)) {
            let Some(record) = self.forge.record_by_guid(guid) else {
                continue;
            };
            match self.resource(&mut trees, &record) {
                Ok(Some(pick)) => found.push(pick),
                Ok(None) => {}
                Err(source) => found.push(Picked::Failed(CatalogFailure::of(&record, &source))),
            }
        }
        found
    }

    fn resource(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, DataForgeError> {
        if record.type_name()? != RESOURCE_TYPE {
            return Ok(None);
        }
        let path = record.record_path()?;
        if self
            .options
            .filter
            .is_some_and(|filter| !filter.matches(path))
        {
            return Ok(None);
        }
        let tree = trees.build(record)?;
        Ok(Some(Picked::Commodity(resource_commodity(
            &tree,
            self.texts,
            &self.groups,
        ))))
    }

    fn check(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, CatalogError> {
        if self.options.cancel.is_some_and(CancelToken::is_cancelled) {
            return Err(CatalogError::Cancelled);
        }
        self.tick();
        Ok(self
            .select(trees, record)
            .unwrap_or_else(|source| Some(Picked::Failed(CatalogFailure::of(record, &source)))))
    }

    fn select(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, DataForgeError> {
        let kind = match select(record, self.options.filter)? {
            None => return Ok(None),
            Some(Selected::Leftover(kind)) => {
                return Ok(Some(Picked::Skipped(kind, record.guid())));
            }
            Some(Selected::Entity(kind)) => kind,
        };
        let tree = trees.build(record)?;
        if kind == Kind::StarmapObjects {
            return Ok(Some(Picked::Starmap(starmap_object(
                &tree,
                self.texts,
                self.forge,
                self.placements,
            ))));
        }
        let resource_type = match kind {
            Kind::Commodities => self.resource_type(trees, &tree)?,
            _ => None,
        };
        let mut picked = Picked::entity(
            kind,
            &tree,
            self.texts,
            resource_type.as_ref(),
            &self.groups,
            &self.evidence,
            &|| self.ships.read(record, &tree),
        );
        if let Some(Picked::Item(item, _, _)) = &mut picked {
            item.stats = item_stats::read(&tree, self.forge, trees).map(Box::new);
            item.variant_key = item_variant::key(&tree, item.stats.as_deref());
        }
        Ok(picked)
    }

    fn resource_type(
        &self,
        trees: &mut TreeBuilder<'f>,
        commodity: &RecordTree<'f>,
    ) -> Result<Option<RecordTree<'f>>, DataForgeError> {
        let Some(guid) = resource_type_guid(commodity) else {
            return Ok(None);
        };
        self.forge
            .record_by_guid(guid.0)
            .map(|record| trees.build(&record))
            .transpose()
    }

    fn tick(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        if let Some(progress) = self.options.progress
            && (done.is_multiple_of(PROGRESS_STEP) || done == self.total)
        {
            progress(CatalogProgress {
                phase: CatalogPhase::Records,
                records_done: done,
                records_total: self.total,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guid_text::CatalogGuid;

    fn line(resource: Option<&str>, options: Vec<CostLine<'static>>) -> CostLine<'static> {
        CostLine {
            kind: "select",
            count: None,
            slot: None,
            slot_key: None,
            options,
            resource_guid: resource.and_then(|text| text.parse().ok()).map(CatalogGuid),
            item_guid: None,
            quantity_scu: None,
            quantity_units: None,
            min_quality: None,
            quantity_multiplier: None,
            composition_inclusion: None,
            modifiers: Vec::new(),
        }
    }

    #[test]
    fn the_resources_of_a_line_include_those_inside_selections() {
        let first = "a1000000-0000-4000-8000-000000000001";
        let second = "b2000000-0000-4000-8000-000000000002";
        let nested = line(
            None,
            vec![
                line(Some(first), Vec::new()),
                line(None, vec![line(Some(second), Vec::new())]),
            ],
        );
        let mut found = Vec::new();
        resources_of(&nested, &mut found);
        let expected: Vec<Guid> = [first, second]
            .iter()
            .filter_map(|text| text.parse().ok())
            .collect();
        assert_eq!(found, expected);
    }
}
