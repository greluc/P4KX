use schemars::JsonSchema;
use serde::Serialize;

use crate::ItemClass;
use crate::contract::{LocalisedText, every_field_required};
use crate::guid_text::CatalogGuid;

/// An item: an `EntityClassDefinition` Record under `entities/scitem` or `entities/legacyitems`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct Item<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `POWR_ACAS_S01_Lumen`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The name, from `AttachDef.Localization.Name`.
    pub(crate) name: LocalisedText<'a>,
    /// The description, from `AttachDef.Localization.Description`.
    pub(crate) description: LocalisedText<'a>,
    /// The item type of `AttachDef.Type`, e.g. `PowerPlant`, `null` without one.
    pub(crate) r#type: Option<&'a str>,
    /// The item subtype of `AttachDef.SubType`, e.g. `Power`, `null` without one.
    pub(crate) sub_type: Option<&'a str>,
    /// The size of `AttachDef.Size`, `null` without one.
    pub(crate) size: Option<i64>,
    /// The grade of `AttachDef.Grade`, `null` without one.
    pub(crate) grade: Option<i64>,
    /// The mass in kilograms of the physics controller, `null` without one.
    pub(crate) mass: Option<f32>,
    /// The GUID of the manufacturer the Reference `AttachDef.Manufacturer` links to, `null` for
    /// a null Reference.
    pub(crate) manufacturer_guid: Option<CatalogGuid>,
    /// The words of `AttachDef.Tags`, in stored order.
    pub(crate) tags: Vec<&'a str>,
    /// The display type of the purchasable parameters, e.g. `@item_DisplayType_PowerPlant`.
    pub(crate) display_type: LocalisedText<'a>,
    /// The item class from the `Class:` line of the English description, `null` for another
    /// value or without that line.
    #[serde(rename = "itemClass")]
    pub(crate) class: Option<ItemClass>,
}

/// A ship or ground vehicle: an `EntityClassDefinition` Record under `entities/spaceships` or
/// `entities/groundvehicles`.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct Ship<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `AEGS_Avenger_Titan`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The name, from `vehicleName` of the vehicle parameters.
    pub(crate) name: LocalisedText<'a>,
    /// The description, from `vehicleDescription` of the vehicle parameters.
    pub(crate) description: LocalisedText<'a>,
    /// The role, from `vehicleRole`, e.g. `@vehicle_class_lightfreight`.
    pub(crate) role: LocalisedText<'a>,
    /// The career, from `vehicleCareer`, e.g. `@vehicle_focus_transporter`.
    pub(crate) career: LocalisedText<'a>,
    /// The crew size, `null` without one.
    pub(crate) crew_size: Option<i64>,
    /// The GUID of the manufacturer the Reference `manufacturer` links to, `null` for a null
    /// Reference.
    pub(crate) manufacturer_guid: Option<CatalogGuid>,
    /// The path of the vehicle definition XML, `null` without one.
    pub(crate) vehicle_definition: Option<&'a str>,
}
