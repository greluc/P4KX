//! Export service of P4K-Extractor: the Record export and the Basetool catalog (ADR-0002).
//!
//! [`write_record_export`] writes a selection of Records in a [`RecordForm`]: one Record XML or
//! Record JSON file per Record under its record path (ADR-0008), or one NDJSON file with one
//! Record per line in record-path order, and returns a [`RecordExportSummary`]. Record JSON is
//! defined in `docs/record-json.md` and `schemas/record-json.schema.json`.
//!
//! [`build_catalog`] reads the DataForge of an open Archive, selects the Records of every kind by
//! record-path prefix and type in parallel, leaves out development leftovers, resolves names and
//! descriptions in English and German from the Archive's `global.ini` files, takes the blueprints
//! with all tiers, the quality, pool, reward-source and dismantle Records from [`read_crafting`],
//! reads each item's
//! [`ItemClass`] from its English description, classifies every ship and item as a
//! [`Playability`] and writes the
//! catalog as deterministic UTF-8 JSON. A selected Record that cannot be read is left out and
//! reported as a [`CatalogFailure`] in the [`CatalogSummary`]. [`catalog_schema`] returns its JSON Schema, generated from
//! the same types (`docs/basetool-catalog.md`).
//!
//! [`build_ship_base`] reads the base data of every ship: hull mass and hit points from the
//! vehicle implementation, cargo capacity from the cargo grids of the default Loadout, size and
//! the Loadout itself, each value with its source file and field (`docs/ship-base-data.md`);
//! [`ShipReader`] reads one ship.
//!
//! [`TagTree::build`] reads the tag tree of a DataForge: tag GUID to tag path, parent and children.
//! The catalog kinds that match tags (paints, loadouts) use it, and `RecordExportOptions::tags`
//! adds the tag path to every tag Reference of an exported Record.
//!
//! [`read_crafting`] reads the complete crafting data of a DataForge as [`Crafting`]: every tier
//! of every blueprint with its selections ("choose n"), quantity multipliers and
//! quality modifiers, the quality distributions with their location overrides, the blueprint
//! pools with the contracts that reward them, the dismantle processes and the global
//! parameters. It is the library API the catalog's crafting fields are built on.
//!
//! [`write_localization_table`] writes every localisation key of a [`TextSet`] with its texts in
//! the chosen languages as JSON or CSV.

mod blueprint;
mod blueprint_category;
mod build;
mod catalog_crafting;
mod catalog_file;
mod catalog_output;
mod catalog_recipe;
mod collected;
mod commodity;
mod contract;
mod contract_crafting;
mod contract_entities;
mod contract_recipe;
mod contract_rewards;
mod crafting;
mod crafting_records;
mod crafting_sources;
mod crafting_tier;
mod crafting_types;
mod crafting_warning;
mod error;
mod failure;
mod form;
mod guid_text;
mod item;
mod item_class;
mod localization_table;
mod manufacturer;
mod ndjson;
mod options;
mod paint;
mod paint_port;
mod playability;
mod playability_rules;
mod record_export;
mod record_fields;
mod record_summary;
mod resource_group;
mod scan;
mod selection;
mod ship;
mod ship_base;
mod summary;
mod texts;

pub use build::{build_catalog, build_catalog_from, write_catalog, write_catalog_from};
pub use catalog_file::CatalogFile;
pub use catalog_output::{Compression, ZSTD_LEVEL, ZSTD_WINDOW_LOG};
pub use contract::{SCHEMA_VERSION, catalog_schema};
pub use crafting::{Crafting, read_crafting, read_crafting_limited};
pub use crafting_types::{
    BlueprintPool, BlueprintTiers, Cost, CostContext, DismantleProcess, DismantleReturn,
    GameplayProperty, GlobalParams, ItemCost, LocationOverride, ModifierKind, PoolEntry,
    PropertyModifier, QualityCurve, QualityDistribution, ResourceCost, RewardChance, RewardSource,
    Selection, Tier, ValueRange,
};
pub use crafting_warning::CraftingWarning;
pub use error::CatalogError;
pub use failure::CatalogFailure;
pub use form::{NDJSON_FILE, RecordForm};
pub use item_class::{ItemClass, ItemClassSummary};
pub use localization_table::{TableFormat, write_localization_table};
pub use options::{CatalogOptions, CatalogProgress};
pub use p4kx_extraction::{
    CancelToken, ConflictPolicy, ExtractionError, Filter, Guid, Language, LocalizationNote,
    PLACEHOLDER_KEYS, RecordExportError, RecordExportOptions, RecordFailure, TAG_PATH_SEPARATOR,
    Tag, TagResolver, TagTree, TagTreeIssues, TextSet,
};
pub use playability::{Playability, PlayabilityCounts, PlayabilitySummary};
pub use record_export::{RecordFilePlans, write_planned_record_export, write_record_export};
pub use record_summary::RecordExportSummary;
pub use selection::{EXCLUDED_TOKENS, Kind, KindRule};
pub use ship_base::{
    BuiltShipBase, Cargo, CargoGrid, Hull, HullThreshold, Implementation, LoadoutPort, Mass,
    MassBasis, SHIP_BASE_VERSION, ShipBase, ShipBaseError, ShipBaseOptions, ShipReader, Source,
    SourceKind, SourceShare, Sourced, UnreadablePartValue, build_ship_base, build_ship_base_from,
};
pub use summary::{
    BuiltCatalog, CatalogSummary, DECOMPRESSED_LIMIT, DECOMPRESSED_WARNING_SIZE, KindSummary,
    PaintSummary, SizeWarning, TextSummary, UPLOAD_LIMIT, UPLOAD_WARNING_SIZE,
};
