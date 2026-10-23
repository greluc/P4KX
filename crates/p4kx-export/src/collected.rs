use p4kx_extraction::RecordTree;

use crate::blueprint_category::blueprint_category;
use crate::catalog_crafting::CraftingKinds;
use crate::commodity::commodity;
use crate::contract::{
    Blueprint, BlueprintCategory, BlueprintPool, Commodity, ContractEntry, CraftingGlobal,
    DismantleProcess, GameplayProperty, Item, Loadout, Manufacturer, MineableElement, Paint,
    QualityDistribution, RefiningProcess, ResourceGroup, Ship, StarmapObject,
};
use crate::crafting_warning::CraftingWarning;
use crate::guid_text::CatalogGuid;
use crate::item::item;
use crate::item_class::ClassLine;
use crate::loadout::carries_ports;
use crate::manufacturer::manufacturer;
use crate::mining::{mineable_element, refining_process};
use crate::paint_port::VehicleFacts;
use crate::playability::PlayabilitySummary;
use crate::playability_rules::Evidence;
use crate::resource_group::{Groups, resource_group};
use crate::ship::ship;
use crate::ship_base::ShipRead;
use crate::texts::Texts;
use crate::{CatalogFailure, ItemClassSummary, Kind, KindSummary};

/// A Record a kind selected: its entity, or a development leftover of the kind.
///
/// The kinds that [`read_crafting`](crate::read_crafting) fills are not picked.
pub(crate) enum Picked<'f> {
    /// A manufacturer.
    Manufacturer(Manufacturer<'f>),
    /// An item, the `Class:` line of its English description and whether it may have ports.
    Item(Item<'f>, ClassLine, bool),
    /// A ship and the vehicle facts its paints are derived from.
    Ship(Ship<'f>, VehicleFacts<'f>),
    /// A commodity.
    Commodity(Commodity<'f>),
    /// A resource group.
    ResourceGroup(ResourceGroup<'f>),
    /// A blueprint category.
    BlueprintCategory(BlueprintCategory<'f>),
    /// A starmap object.
    Starmap(StarmapObject<'f>),
    /// A mineable element.
    Mineable(MineableElement<'f>),
    /// A refining process.
    Refining(RefiningProcess<'f>),
    /// A development leftover of the kind.
    Skipped(Kind),
    /// A Record that could not be read.
    Failed(CatalogFailure),
}

impl<'f> Picked<'f> {
    /// Returns the entity of `kind` the Record tree `tree` describes, its texts resolved by
    /// `texts`; a commodity may take texts from its `ResourceType` Record `resource_type` and its
    /// type from `groups`, a ship its playability from `evidence` and its base data from
    /// `ship_base`. `None` for a kind that [`read_crafting`](crate::read_crafting) fills, and for
    /// a ship without `ship_base`.
    pub(crate) fn entity(
        kind: Kind,
        tree: &RecordTree<'f>,
        texts: &Texts<'f>,
        resource_type: Option<&RecordTree<'f>>,
        groups: &Groups,
        evidence: &Evidence,
        ship_base: Option<&ShipRead>,
    ) -> Option<Self> {
        Some(match kind {
            Kind::Manufacturers => Self::Manufacturer(manufacturer(tree, texts)),
            Kind::Items => {
                let (item, line) = item(tree, texts);
                Self::Item(item, line, carries_ports(tree))
            }
            Kind::Ships => {
                let (ship, vehicle) = ship(tree, texts, evidence, ship_base?);
                Self::Ship(ship, vehicle)
            }
            Kind::Commodities => Self::Commodity(commodity(tree, texts, resource_type, groups)),
            Kind::ResourceGroups => Self::ResourceGroup(resource_group(tree, texts, groups)),
            Kind::BlueprintCategories => Self::BlueprintCategory(blueprint_category(tree, texts)),
            Kind::MineableElements => Self::Mineable(mineable_element(tree)),
            Kind::RefiningProcesses => Self::Refining(refining_process(tree, texts)),
            Kind::Blueprints
            | Kind::GameplayProperties
            | Kind::QualityDistributions
            | Kind::BlueprintPools
            | Kind::DismantleProcesses
            | Kind::Paints
            | Kind::Loadouts
            | Kind::StarmapObjects
            | Kind::Contracts => return None,
        })
    }
}

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
    pub(crate) vehicles: Vec<VehicleFacts<'f>>,
    pub(crate) item_classes: ItemClassSummary,
    pub(crate) playability: PlayabilitySummary,
    pub(crate) failures: Vec<CatalogFailure>,
    pub(crate) warnings: Vec<CraftingWarning>,
    skipped: [usize; Kind::ALL.len()],
}

impl<'f> Collected<'f> {
    /// Sorts `picked` into the kinds and orders every kind by the text of the GUID; `crafting`
    /// fills the kinds [`read_crafting`](crate::read_crafting) reads.
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
            vehicles: Vec::new(),
            item_classes: ItemClassSummary::default(),
            playability: PlayabilitySummary::default(),
            failures: crafting.failures,
            warnings: crafting.warnings,
            skipped,
        };
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
                Picked::Skipped(kind) => {
                    if let Some(skipped) = collected.skipped.get_mut(kind.index()) {
                        *skipped = skipped.saturating_add(1);
                    }
                }
                Picked::Failed(failure) => collected.failures.push(failure),
            }
        }
        collected.order();
        collected
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
        self.manufacturers
            .sort_by_cached_key(|entity| entity.guid.text());
        self.items.sort_by_cached_key(|entity| entity.guid.text());
        self.ships.sort_by_cached_key(|entity| entity.guid.text());
        self.commodities
            .sort_by_cached_key(|entity| entity.guid.text());
        self.resource_groups
            .sort_by_cached_key(|entity| entity.guid.text());
        self.blueprint_categories
            .sort_by_cached_key(|entity| entity.guid.text());
        self.starmap_objects
            .sort_by_cached_key(|entity| entity.guid.text());
        self.mineable_elements
            .sort_by_cached_key(|entity| entity.guid.text());
        self.refining_processes
            .sort_by_cached_key(|entity| entity.guid.text());
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
                    Kind::BlueprintPools => self.blueprint_pools.len(),
                    Kind::DismantleProcesses => self.dismantle_processes.len(),
                    Kind::Paints => self.paints.len(),
                    Kind::Loadouts => self.loadouts.len(),
                    Kind::StarmapObjects => self.starmap_objects.len(),
                    Kind::Contracts => self.contracts.len(),
                    Kind::MineableElements => self.mineable_elements.len(),
                    Kind::RefiningProcesses => self.refining_processes.len(),
                },
                skipped: self.skipped.get(kind.index()).copied().unwrap_or_default(),
                complete,
            })
            .collect()
    }
}
