use p4kx_extraction::Guid;

/// How the quality range of a gameplay-property modifier changes the property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModifierKind {
    /// A factor: `CraftingGameplayPropertyModifierValueRange_Linear`.
    Multiplier,
    /// A summand: `CraftingGameplayPropertyModifierValueRange_LinearIntegerAdditive`.
    Additive,
}

/// One linear range of a modifier: the value at the quality `start_quality` and at `end_quality`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueRange {
    /// Whether the values are factors or summands.
    pub kind: ModifierKind,
    /// The quality the range starts at, `None` when the Record holds none.
    pub start_quality: Option<i64>,
    /// The quality the range ends at.
    pub end_quality: Option<i64>,
    /// The modifier at the start quality.
    pub at_start: Option<f32>,
    /// The modifier at the end quality.
    pub at_end: Option<f32>,
}

/// A gameplay property of the crafted item with the ranges that scale it by quality.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyModifier {
    /// The GUID of the `CraftingGameplayPropertyDef` Record, `None` for a null Reference.
    pub property: Option<Guid>,
    /// The ranges in stored order.
    pub ranges: Vec<ValueRange>,
}

/// The context of a cost: what the game applies to it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CostContext<'f> {
    /// The `CraftingCostContext_QuantityMultiplier` factor, `None` without one.
    pub quantity_multiplier: Option<f32>,
    /// The option of `CraftingCostContext_ResultCompositionInclusion`, e.g. `Include`.
    pub composition_inclusion: Option<&'f str>,
    /// The modifiers of `CraftingCostContext_ResultGameplayPropertyModifiers`.
    pub modifiers: Vec<PropertyModifier>,
}

/// A resource line: a quantity of a `ResourceType`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceCost<'f> {
    /// The GUID of the `ResourceType` Record.
    pub resource: Option<Guid>,
    /// The quantity in standard cargo units (SCU), before the quantity multiplier.
    pub quantity_scu: Option<f32>,
    /// The minimum quality of the resource.
    pub min_quality: Option<i64>,
    /// What the game applies to the line.
    pub context: CostContext<'f>,
}

/// An item line: a number of entities.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemCost<'f> {
    /// The GUID of the entity.
    pub item: Option<Guid>,
    /// The number of units, before the quantity multiplier.
    pub quantity: Option<i64>,
    /// The minimum quality of the item.
    pub min_quality: Option<i64>,
    /// What the game applies to the line.
    pub context: CostContext<'f>,
}

/// A selection ("choose n"): the player picks `count` of its options.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection<'f> {
    /// How many options the player chooses, `None` when the Record holds no count.
    pub count: Option<i64>,
    /// The debug name of the slot, `None` without one.
    pub slot: Option<&'f str>,
    /// The localisation key of the slot name, `None` without one or for a placeholder.
    pub slot_key: Option<&'f str>,
    /// What the game applies to the selection.
    pub context: CostContext<'f>,
    /// The options in stored order.
    pub options: Vec<Cost<'f>>,
}

impl Selection<'_> {
    /// Returns whether the player may choose fewer options than the selection offers.
    #[must_use]
    pub fn chooses_fewer(&self) -> bool {
        self.count
            .and_then(|count| usize::try_from(count).ok())
            .is_some_and(|count| count < self.options.len())
    }
}

/// One cost of a recipe.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Cost<'f> {
    /// A selection of costs.
    Select(Selection<'f>),
    /// A resource line.
    Resource(ResourceCost<'f>),
    /// An item line.
    Item(ItemCost<'f>),
}

/// One tier of a blueprint: its craft time and its costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier<'f> {
    /// The craft time in whole seconds, `None` without one.
    pub craft_time_seconds: Option<i64>,
    /// The mandatory cost, `None` without one.
    pub mandatory: Option<Cost<'f>>,
    /// The optional costs in stored order.
    pub optional: Vec<Cost<'f>>,
}

/// A blueprint with all its tiers.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintTiers<'f> {
    /// The GUID of the `CraftingBlueprintRecord`.
    pub guid: Guid,
    /// The Record name without its type.
    pub class_name: &'f str,
    /// The record path, without `.xml`.
    pub record_path: &'f str,
    /// The GUID of the entity the blueprint produces, `None` without one.
    pub produced_item: Option<Guid>,
    /// The tiers in stored order, the first one being the tier the catalog's `ingredients`
    /// describe.
    pub tiers: Vec<Tier<'f>>,
}

/// A gameplay property definition: a `CraftingGameplayPropertyDef` Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayProperty<'f> {
    /// The GUID of the Record.
    pub guid: Guid,
    /// The Record name without its type.
    pub class_name: &'f str,
    /// The record path, without `.xml`.
    pub record_path: &'f str,
    /// The localisation key of the property name, `None` for none or a placeholder.
    pub name_key: Option<&'f str>,
}

/// A normal distribution of quality.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityCurve<'f> {
    /// The struct of the distribution, e.g. `CraftingQualityDistributionNormal`.
    pub kind: &'f str,
    /// The lowest quality.
    pub min: Option<f32>,
    /// The highest quality.
    pub max: Option<f32>,
    /// The mean.
    pub mean: Option<f32>,
    /// The standard deviation.
    pub stddev: Option<f32>,
}

/// A distribution that replaces the default one at a location.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationOverride<'f> {
    /// The GUID of the location Record, `None` for a null Reference.
    pub location: Option<Guid>,
    /// The distribution at the location, `None` when the Record holds none.
    pub distribution: Option<QualityCurve<'f>>,
}

/// A quality distribution Record: the default distribution of a source or its location
/// overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityDistribution<'f> {
    /// The GUID of the Record.
    pub guid: Guid,
    /// The Record name without its type.
    pub class_name: &'f str,
    /// The record path, without `.xml`.
    pub record_path: &'f str,
    /// The distribution of a `CraftingQualityDistributionRecord`, `None` for an override Record.
    pub distribution: Option<QualityCurve<'f>>,
    /// The overrides of a `CraftingQualityLocationOverrideRecord`, empty for a default Record.
    pub overrides: Vec<LocationOverride<'f>>,
}

/// One entry of a blueprint pool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolEntry {
    /// The GUID of the blueprint, `None` for a null Reference.
    pub blueprint: Option<Guid>,
    /// The weight of the entry among the pool's.
    pub weight: Option<f32>,
}

/// A blueprint pool: a `BlueprintPoolRecord` a reward draws a blueprint from.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintPool<'f> {
    /// The GUID of the Record.
    pub guid: Guid,
    /// The Record name without its type.
    pub class_name: &'f str,
    /// The record path, without `.xml`.
    pub record_path: &'f str,
    /// The entries in stored order.
    pub entries: Vec<PoolEntry>,
}

impl BlueprintPool<'_> {
    /// Returns the sum of the positive weights of the entries.
    #[must_use]
    pub fn total_weight(&self) -> f32 {
        self.entries
            .iter()
            .filter_map(|entry| entry.weight)
            .filter(|weight| *weight > 0.0)
            .sum()
    }
}

/// A place that rewards a blueprint of a pool with a chance: a `BlueprintRewards` result of a
/// contract.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardSource<'f> {
    /// The GUID of the Record that holds the reward.
    pub record: Guid,
    /// The record path of that Record, without `.xml`.
    pub record_path: &'f str,
    /// The `id` of the nearest enclosing struct that has one, e.g. the contract.
    pub contract_id: Option<Guid>,
    /// The `debugName` of the nearest enclosing struct that has one.
    pub contract_name: Option<&'f str>,
    /// The GUID of the `BlueprintPoolRecord`, `None` for a null Reference.
    pub pool: Option<Guid>,
    /// The chance that the reward is given, 0 to 1, `None` when the Record holds none.
    pub chance: Option<f32>,
}

/// The chance that one reward source gives a blueprint: the chance of the source times the
/// weight of the blueprint over the sum of the weights of its pool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RewardChance<'f> {
    /// The GUID of the Record that holds the reward.
    pub record: Guid,
    /// The `id` of the contract, if any.
    pub contract_id: Option<Guid>,
    /// The `debugName` of the contract, if any.
    pub contract_name: Option<&'f str>,
    /// The GUID of the pool.
    pub pool: Guid,
    /// The chance that the source gives the blueprint, 0 to 1.
    pub chance: f64,
}

/// A generic dismantle process: a `CraftingBlueprintRecord` under `crafting/blueprints/dismantle`
/// whose process is a `GenericCraftingProcess_Dismantle`.
#[derive(Debug, Clone, PartialEq)]
pub struct DismantleProcess<'f> {
    /// The GUID of the Record.
    pub guid: Guid,
    /// The Record name without its type.
    pub class_name: &'f str,
    /// The record path, without `.xml`.
    pub record_path: &'f str,
    /// The share of the crafting ingredients a dismantled item returns, 0 to 1.
    pub efficiency: Option<f32>,
    /// The dismantle time in whole seconds.
    pub time_seconds: Option<i64>,
}

/// The `CraftingGlobalParams` Record: the default blueprints and the dismantle exclusions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalParams {
    /// The blueprints a player has by default (`defaultBlueprintSelection`).
    pub default_blueprints: Vec<Guid>,
    /// The `ResourceType` Records dismantling never returns.
    pub dismantle_blacklist_resources: Vec<Guid>,
    /// The entities that cannot be dismantled.
    pub dismantle_blacklist_items: Vec<Guid>,
}

/// The dismantle return of a blueprint's item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DismantleReturn<'c, 'f> {
    /// The process that dismantles the item.
    pub process: &'c DismantleProcess<'f>,
    /// The GUID of the blueprint whose ingredients the item returns by `process.efficiency`.
    pub blueprint: Guid,
    /// The GUID of the item.
    pub item: Guid,
}
