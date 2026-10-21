use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::{LocalisedText, every_field_required};
use crate::guid_text::CatalogGuid;

/// A commodity: an `EntityClassDefinition` Record under `entities/commodities` or a
/// `ResourceType` Record a blueprint ingredient uses that no commodity Record carries.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Commodity<'a> {
    /// The GUID of the Record; for a `ResourceType` entry the GUID of the `ResourceType`.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `Gold`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The name, from `SCItemPurchasableParams.displayName`.
    pub(crate) name: LocalisedText<'a>,
    /// The description, from the entity's display parameters or, without one, from the
    /// `ResourceType` Record of the commodity.
    pub(crate) description: LocalisedText<'a>,
    /// The GUID of the `ResourceTypeGroup` Record that directly lists the `ResourceType`, `null`
    /// without a `ResourceType` or a group.
    pub(crate) type_guid: Option<CatalogGuid>,
    /// The GUID of the `ResourceType` Record the commodity's default composition holds first,
    /// `null` without one. The `resourceGuid` of an ingredient equals the
    /// `subtypeGuid` of exactly one commodity.
    pub(crate) subtype_guid: Option<CatalogGuid>,
    /// The display type of the purchasable parameters, e.g. `@items_commodities_type_metal`;
    /// without a key for a `ResourceType` entry.
    pub(crate) display_type: LocalisedText<'a>,
}

/// A crafting blueprint: a `CraftingBlueprintRecord` Record under `crafting/blueprints/crafting`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Blueprint<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `BP_CRAFT_behr_lmg_ballistic_01_mag`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The blueprint key, the Record name.
    pub(crate) key: &'a str,
    /// The name, from `CraftingBlueprint.blueprintName`.
    pub(crate) name: LocalisedText<'a>,
    /// The GUID of the category Record, `null` without one.
    pub(crate) category_guid: Option<CatalogGuid>,
    /// The GUID of the entity the blueprint produces, `null` without one or for a Reference to
    /// a Record the DataForge lacks.
    pub(crate) produced_item_guid: Option<CatalogGuid>,
    /// The craft time of the first tier in seconds, `null` without one.
    pub(crate) craft_time_seconds: Option<i64>,
    /// The ingredients of the first tier in stored order.
    pub(crate) ingredients: Vec<Ingredient<'a>>,
}

/// One ingredient line of a blueprint: a resource or an item with its quantity.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Ingredient<'a> {
    /// The GUID of the `ResourceType` Record of a resource line, `null` for an item line.
    pub(crate) resource_guid: Option<CatalogGuid>,
    /// The GUID of the entity of an item line, `null` for a resource line.
    pub(crate) item_guid: Option<CatalogGuid>,
    /// The quantity of a resource line in standard cargo units (SCU), `null` for an item line.
    pub(crate) quantity_scu: Option<f32>,
    /// The quantity of an item line in units, `null` for a resource line.
    pub(crate) quantity_units: Option<i64>,
    /// The minimum quality the ingredient must have, `null` without one.
    pub(crate) min_quality: Option<i64>,
    /// The debug name of the slot the ingredient fills, `null` without one.
    pub(crate) slot: Option<&'a str>,
    /// The localisation key of the slot name, `null` without one or for a placeholder.
    pub(crate) slot_key: Option<&'a str>,
}
