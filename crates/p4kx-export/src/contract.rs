use std::borrow::Cow;

use schemars::{JsonSchema, Schema};
use serde::Serialize;

use crate::guid_text::CatalogGuid;

pub(crate) use crate::contract_crafting::{Blueprint, Commodity, Ingredient};
pub(crate) use crate::contract_entities::{Item, Ship};

/// The version of the catalog shape.
///
/// Version 1 stays open until the Basetool import reads it: kinds and fields are added to it
/// until M3 is complete; from then on every change of the shape raises it.
pub const SCHEMA_VERSION: u32 = 1;

/// The Basetool catalog: the game data the basetool's P4K import reads.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
#[schemars(title = "P4K-Extractor Basetool catalog")]
pub(crate) struct Catalog<'a> {
    /// The version of the catalog shape; an import rejects a version it does not know.
    #[schemars(extend("const" = 1))]
    pub(crate) schema_version: u32,
    /// The Archive and Build the catalog was made from.
    pub(crate) source: Source<'a>,
    /// Whether each kind is a complete snapshot of the Build.
    pub(crate) completeness: Completeness<'a>,
    /// The manufacturers, ordered by GUID.
    pub(crate) manufacturers: Vec<Manufacturer<'a>>,
    /// The items, ordered by GUID.
    pub(crate) items: Vec<Item<'a>>,
    /// The ships, ordered by GUID.
    pub(crate) ships: Vec<Ship<'a>>,
    /// The commodities, ordered by GUID.
    pub(crate) commodities: Vec<Commodity<'a>>,
    /// The blueprints, ordered by GUID.
    pub(crate) blueprints: Vec<Blueprint<'a>>,
}

/// Where a catalog came from.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct Source<'a> {
    /// The Channel: the name of the folder holding the Archive, `null` when there is none.
    pub(crate) channel: Option<String>,
    /// The Build from the Channel's build manifest, `null` when it is unknown.
    pub(crate) build: Option<BuildSource<'a>>,
    /// The size of the Archive file in bytes.
    pub(crate) archive_size: u64,
    /// The modification time of the Archive file, UTC in RFC 3339 form, `null` when unknown.
    pub(crate) archive_modified: Option<String>,
    /// The version of P4K-Extractor that wrote the catalog.
    pub(crate) tool_version: &'static str,
    /// When the catalog was written, UTC in RFC 3339 form.
    pub(crate) exported_at: String,
}

/// The Build of the Archive.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct BuildSource<'a> {
    /// The branch, e.g. `sc-alpha-4.4.0`.
    pub(crate) branch: &'a str,
    /// The version, e.g. `4.4.162.12345`.
    pub(crate) version: &'a str,
    /// The Perforce change number the Build was made from.
    pub(crate) change_number: &'a str,
    /// The `BuildId` of the build manifest.
    pub(crate) build_id: &'a str,
}

/// The completeness of every kind.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct Completeness<'a> {
    /// The completeness of `manufacturers`.
    pub(crate) manufacturers: KindCompleteness<'a>,
    /// The completeness of `items`.
    pub(crate) items: KindCompleteness<'a>,
    /// The completeness of `ships`.
    pub(crate) ships: KindCompleteness<'a>,
    /// The completeness of `commodities`.
    pub(crate) commodities: KindCompleteness<'a>,
    /// The completeness of `blueprints`.
    pub(crate) blueprints: KindCompleteness<'a>,
}

/// Whether a kind holds every Record of the Build or only those a Filter selected.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum KindCompleteness<'a> {
    /// Every Record of the kind is listed; one missing from the catalog was removed from the game.
    Complete,
    /// Only the Records the Filter selected are listed.
    Partial {
        /// The Filter text.
        filter: &'a str,
    },
}

/// A manufacturer: an `SCItemManufacturer` Record.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct Manufacturer<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `ACAS` for `SCItemManufacturer.ACAS`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The manufacturer code, e.g. `ACAS`, `null` when it cannot be read.
    pub(crate) code: Option<&'a str>,
    /// The name.
    pub(crate) name: LocalisedText<'a>,
    /// The description.
    pub(crate) description: LocalisedText<'a>,
}

/// A text the game localises: its localisation key and the resolved English and German texts,
/// as the language's `global.ini` stores them with the escape `\n` decoded to a line feed.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(transform = every_field_required)]
pub(crate) struct LocalisedText<'a> {
    /// The localisation key, e.g. `@manufacturer_NameACAS`, `null` when it is unknown.
    pub(crate) key: Option<&'a str>,
    /// The English text, `null` for no key, a placeholder key, or a key the English `global.ini`
    /// lacks or holds empty.
    pub(crate) en: Option<Cow<'a, str>>,
    /// The German text, `null` for no key, a placeholder key, or a key the German `global.ini`
    /// lacks or holds empty.
    pub(crate) de: Option<Cow<'a, str>>,
}

/// Makes every property of an object schema required: the catalog writes `null`, never omits.
pub(crate) fn every_field_required(schema: &mut Schema) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let names: Vec<serde_json::Value> = object
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|properties| {
            properties
                .keys()
                .cloned()
                .map(serde_json::Value::String)
                .collect()
        })
        .unwrap_or_default();
    object.insert("required".to_owned(), serde_json::Value::Array(names));
}

/// Returns the JSON Schema (draft 2020-12) of the Basetool catalog.
#[must_use]
pub fn catalog_schema() -> serde_json::Value {
    schemars::schema_for!(Catalog<'static>).to_value()
}
