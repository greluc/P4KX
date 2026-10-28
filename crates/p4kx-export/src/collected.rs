// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use crate::blueprint_category::unlink_categories;
use crate::catalog_crafting::CraftingKinds;
use crate::contract::{
    Blueprint, BlueprintCategory, BlueprintPool, Commodity, ContractEntry, CraftingGlobal,
    DismantleProcess, GameplayProperty, HarvestableLocation, HarvestableProviderPreset, Item,
    Loadout, LootContainer, Manufacturer, MineableComposition, MineableElement, MineableEntity,
    MiningGlobalParams, Paint, QualityDistribution, QualityQuantization, RefiningProcess,
    ResourceGroup, Ship, Shop, StarmapObject, SubHarvestableMultiConfig,
};
use crate::crafting_warning::CraftingWarning;
use crate::guid_text::{CatalogGuid, sort_by_guid};
use crate::loot_kinds::LootKinds;
use crate::mission_kinds::MissionKinds;
use crate::paint_port::VehicleFacts;
use crate::picked::Picked;
use crate::playability::PlayabilitySummary;
use crate::{CatalogFailure, ItemClassSummary, Kind, KindSummary, QuantizationSummary};

/// The entities of every kind, ordered by GUID, with the counts of the summary.
pub(crate) struct Collected<'f> {
    pub(crate) manufacturers: Vec<Manufacturer<'f>>,
    pub(crate) items: Vec<Item<'f>>,
    pub(crate) ships: Vec<Ship<'f>>,
    pub(crate) commodities: Vec<Commodity<'f>>,
    pub(crate) blueprints: Vec<Blueprint<'f>>,
    pub(crate) resource_groups: Vec<ResourceGroup<'f>>,
    pub(crate) blueprint_categories: Vec<BlueprintCategory<'f>>,
    pub(crate) gameplay_properties: Vec<GameplayProperty<'f>>,
    pub(crate) quality_distributions: Vec<QualityDistribution<'f>>,
    pub(crate) quality_quantizations: Vec<QualityQuantization<'f>>,
    pub(crate) quantization_summary: QuantizationSummary,
    pub(crate) blueprint_pools: Vec<BlueprintPool<'f>>,
    pub(crate) dismantle_processes: Vec<DismantleProcess<'f>>,
    pub(crate) crafting_global: CraftingGlobal,
    pub(crate) paints: Vec<Paint>,
    pub(crate) loadouts: Vec<Loadout>,
    pub(crate) contracts: Vec<ContractEntry<'f>>,
    pub(crate) port_items: Vec<CatalogGuid>,
    pub(crate) starmap_objects: Vec<StarmapObject<'f>>,
    pub(crate) mineable_elements: Vec<MineableElement<'f>>,
    pub(crate) refining_processes: Vec<RefiningProcess<'f>>,
    pub(crate) mineable_compositions: Vec<MineableComposition<'f>>,
    pub(crate) mining_global_params: Vec<MiningGlobalParams<'f>>,
    pub(crate) mineable_entities: Vec<MineableEntity<'f>>,
    pub(crate) harvestable_provider_presets: Vec<HarvestableProviderPreset<'f>>,
    pub(crate) sub_harvestable_multi_configs: Vec<SubHarvestableMultiConfig<'f>>,
    pub(crate) harvestable_locations: Vec<HarvestableLocation>,
    pub(crate) loot_containers: Vec<LootContainer>,
    pub(crate) shops: Vec<Shop>,
    pub(crate) mission_kinds: MissionKinds<'f>,
    pub(crate) loot_kinds: LootKinds<'f>,
    pub(crate) vehicles: Vec<VehicleFacts>,
    pub(crate) item_classes: ItemClassSummary,
    pub(crate) playability: PlayabilitySummary,
    pub(crate) failures: Vec<CatalogFailure>,
    pub(crate) warnings: Vec<CraftingWarning>,
    skipped: [usize; Kind::ALL.len()],
}

impl<'f> Collected<'f> {
    /// Sorts `picked` into the kinds and orders every kind by the text of the GUID; `crafting`
    /// fills the kinds [`read_crafting`](crate::read_crafting) reads.
    #[expect(
        clippy::too_many_lines,
        reason = "one initialiser per kind of the catalog"
    )]
    pub(crate) fn new(picked: Vec<Picked<'f>>, crafting: CraftingKinds<'f>) -> Self {
        let skipped = Self::crafting_skipped(&crafting);
        let mut collected = Self {
            manufacturers: Vec::new(),
            items: Vec::new(),
            ships: Vec::new(),
            commodities: Vec::new(),
            blueprints: crafting.blueprints,
            resource_groups: Vec::new(),
            blueprint_categories: Vec::new(),
            gameplay_properties: crafting.properties,
            quality_distributions: crafting.distributions,
            quality_quantizations: crafting.quantizations,
            quantization_summary: crafting.quantization_summary,
            blueprint_pools: crafting.pools,
            dismantle_processes: crafting.dismantle,
            crafting_global: crafting.global,
            paints: Vec::new(),
            loadouts: Vec::new(),
            contracts: Vec::new(),
            port_items: Vec::new(),
            starmap_objects: Vec::new(),
            mineable_elements: Vec::new(),
            refining_processes: Vec::new(),
            mineable_compositions: Vec::new(),
            mining_global_params: Vec::new(),
            mineable_entities: Vec::new(),
            harvestable_provider_presets: Vec::new(),
            sub_harvestable_multi_configs: Vec::new(),
            harvestable_locations: Vec::new(),
            loot_containers: Vec::new(),
            shops: Vec::new(),
            mission_kinds: MissionKinds::default(),
            loot_kinds: LootKinds::default(),
            vehicles: Vec::new(),
            item_classes: ItemClassSummary::default(),
            playability: PlayabilitySummary::default(),
            failures: crafting.failures,
            warnings: crafting.warnings,
            skipped,
        };
        let mut missing_categories = HashSet::new();
        for pick in picked {
            match pick {
                Picked::Manufacturer(entity) => collected.manufacturers.push(entity),
                Picked::Item(entity, line, ports) => {
                    collected.item_classes.count(line);
                    if ports {
                        collected.port_items.push(entity.guid);
                    }
                    collected.playability.items.count(entity.playability);
                    collected.items.push(entity);
                }
                Picked::Ship(entity, vehicle) => {
                    collected.playability.ships.count(entity.playability);
                    collected.ships.push(entity);
                    collected.vehicles.push(vehicle);
                }
                Picked::Commodity(entity) => collected.commodities.push(entity),
                Picked::ResourceGroup(entity) => collected.resource_groups.push(entity),
                Picked::BlueprintCategory(entity) => collected.blueprint_categories.push(entity),
                Picked::Starmap(entity) => collected.starmap_objects.push(entity),
                Picked::Mineable(entity) => collected.mineable_elements.push(entity),
                Picked::Refining(entity) => collected.refining_processes.push(entity),
                Picked::Composition(entity) => collected.mineable_compositions.push(entity),
                Picked::MiningParams(entity) => collected.mining_global_params.push(entity),
                Picked::MineableEntity(entity) => collected.mineable_entities.push(entity),
                Picked::Provider(entity) => collected.harvestable_provider_presets.push(entity),
                Picked::MultiConfig(entity) => collected.sub_harvestable_multi_configs.push(entity),
                Picked::Mission(entity) => collected.mission_kinds.push(entity),
                Picked::Loot(entity) => collected.loot_kinds.push(entity),
                Picked::Skipped(kind, guid) => {
                    if kind == Kind::BlueprintCategories {
                        missing_categories.insert(guid);
                    }
                    collected.skip(kind);
                }
                Picked::SkippedPaint => {
                    collected.skip(Kind::Items);
                    collected.skip(Kind::Paints);
                }
                Picked::Failed(failure) => collected.failures.push(failure),
                Picked::Warned(warning) => collected.warnings.push(warning),
            }
        }
        missing_categories.extend(collected.failures.iter().map(|failure| failure.guid));
        unlink_categories(&mut collected.blueprints, &missing_categories);
        collected.order();
        collected
    }

    fn skip(&mut self, kind: Kind) {
        if let Some(skipped) = self.skipped.get_mut(kind.index()) {
            *skipped = skipped.saturating_add(1);
        }
    }

    fn crafting_skipped(crafting: &CraftingKinds<'f>) -> [usize; Kind::ALL.len()] {
        let mut skipped = [0; Kind::ALL.len()];
        for (kind, count) in &crafting.skipped {
            if let Some(slot) = skipped.get_mut(kind.index()) {
                *slot = *count;
            }
        }
        skipped
    }

    /// Stores the contracts, counts the entries left out as `skipped` and adds the failures.
    pub(crate) fn add_contracts(
        &mut self,
        contracts: Vec<ContractEntry<'f>>,
        skipped: usize,
        failures: Vec<CatalogFailure>,
    ) {
        self.contracts = contracts;
        if let Some(slot) = self.skipped.get_mut(Kind::Contracts.index()) {
            *slot = skipped;
        }
        self.add_failures(failures);
    }

    /// Adds `failures` and orders all of them by the text of the GUID, one per Record.
    pub(crate) fn add_failures(&mut self, failures: Vec<CatalogFailure>) {
        self.failures.extend(failures);
        self.order_failures();
    }

    fn order_failures(&mut self) {
        self.failures
            .sort_by_cached_key(|failure| CatalogGuid(failure.guid).text());
        self.failures
            .dedup_by(|later, earlier| later.guid == earlier.guid);
    }

    /// Orders the failures and the picked kinds by the text of the GUID.
    fn order(&mut self) {
        self.order_failures();
        sort_by_guid(&mut self.manufacturers, |entity| entity.guid);
        sort_by_guid(&mut self.items, |entity| entity.guid);
        sort_by_guid(&mut self.ships, |entity| entity.guid);
        sort_by_guid(&mut self.commodities, |entity| entity.guid);
        sort_by_guid(&mut self.resource_groups, |entity| entity.guid);
        sort_by_guid(&mut self.blueprint_categories, |entity| entity.guid);
        sort_by_guid(&mut self.starmap_objects, |entity| entity.guid);
        sort_by_guid(&mut self.mineable_elements, |entity| entity.guid);
        sort_by_guid(&mut self.refining_processes, |entity| entity.guid);
        sort_by_guid(&mut self.mineable_compositions, |entity| entity.guid);
        sort_by_guid(&mut self.mining_global_params, |entity| entity.guid);
        sort_by_guid(&mut self.mineable_entities, |entity| entity.guid);
        sort_by_guid(&mut self.harvestable_provider_presets, |entity| entity.guid);
        sort_by_guid(&mut self.sub_harvestable_multi_configs, |entity| {
            entity.guid
        });
        self.mission_kinds.order();
        self.loot_kinds.order();
    }

    /// Returns the counts of every kind, in the order of [`Kind::ALL`].
    pub(crate) fn summary(&self, complete: bool) -> Vec<KindSummary> {
        Kind::ALL
            .into_iter()
            .map(|kind| KindSummary {
                kind,
                count: match kind {
                    Kind::Manufacturers => self.manufacturers.len(),
                    Kind::Items => self.items.len(),
                    Kind::Ships => self.ships.len(),
                    Kind::Commodities => self.commodities.len(),
                    Kind::Blueprints => self.blueprints.len(),
                    Kind::ResourceGroups => self.resource_groups.len(),
                    Kind::BlueprintCategories => self.blueprint_categories.len(),
                    Kind::GameplayProperties => self.gameplay_properties.len(),
                    Kind::QualityDistributions => self.quality_distributions.len(),
                    Kind::QualityQuantizations => self.quality_quantizations.len(),
                    Kind::BlueprintPools => self.blueprint_pools.len(),
                    Kind::DismantleProcesses => self.dismantle_processes.len(),
                    Kind::Paints => self.paints.len(),
                    Kind::Loadouts => self.loadouts.len(),
                    Kind::StarmapObjects => self.starmap_objects.len(),
                    Kind::Contracts => self.contracts.len(),
                    Kind::MineableElements => self.mineable_elements.len(),
                    Kind::RefiningProcesses => self.refining_processes.len(),
                    Kind::MineableCompositions => self.mineable_compositions.len(),
                    Kind::MiningGlobalParams => self.mining_global_params.len(),
                    Kind::MineableEntities => self.mineable_entities.len(),
                    Kind::HarvestableProviderPresets => self.harvestable_provider_presets.len(),
                    Kind::SubHarvestableMultiConfigs => self.sub_harvestable_multi_configs.len(),
                    Kind::HarvestableLocations => self.harvestable_locations.len(),
                    Kind::LootContainers => self.loot_containers.len(),
                    Kind::Shops => self.shops.len(),
                    Kind::FactionReputations
                    | Kind::ReputationScopes
                    | Kind::ReputationStandings
                    | Kind::ReputationContexts
                    | Kind::MissionTypes
                    | Kind::MissionLocalities
                    | Kind::ContractScenarios => self.mission_kinds.count(kind),
                    Kind::LootTables
                    | Kind::LootArchetypes
                    | Kind::LootPoolFilters
                    | Kind::LegacyLootTables
                    | Kind::LegacyLootArchetypes => self.loot_kinds.count(kind),
                },
                skipped: self.skipped.get(kind.index()).copied().unwrap_or_default(),
                complete,
            })
            .collect()
    }
}
