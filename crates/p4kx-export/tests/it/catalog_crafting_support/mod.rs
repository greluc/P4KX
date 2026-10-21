//! Building and reading the catalog of a synthetic Archive for the crafting catalog tests.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{BuiltCatalog, CatalogOptions, Filter, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry};
use serde_json::Value;

/// Returns the GUID `value` as canonical lower-case UUID text.
pub(crate) fn text(value: u128) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        value >> 96,
        (value >> 80) & 0xffff,
        (value >> 64) & 0xffff,
        (value >> 48) & 0xffff,
        value & 0xffff_ffff_ffff
    )
}

fn write_archive(dir: &Path, forge: &DataForgeBuilder) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()).encrypted())
        .entry(TestEntry::zstd(
            "Data\\Localization\\english\\global.ini",
            GlobalIniBuilder::new()
                .text("StatName_GPP_Armor", "Armour")
                .build(),
        ))
        .write_to(&path)?;
    Ok(path)
}

/// Builds the catalog of an Archive holding `forge`, and returns it with its JSON.
pub(crate) fn build_from(
    forge: &DataForgeBuilder,
    filter: Option<&Filter>,
) -> Result<(BuiltCatalog, Value), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = Archive::open_with_key(
        write_archive(dir.path(), forge)?,
        Some(p4kx_testkit::test_key()),
    )?;
    let options = CatalogOptions {
        filter,
        ..CatalogOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000))
    };
    let built = build_catalog(&archive, &options)?;
    let json = serde_json::from_slice(built.json())?;
    Ok((built, json))
}

/// Returns the blueprint with the GUID `guid`.
pub(crate) fn blueprint(catalog: &Value, guid: u128) -> Result<&Value, Box<dyn Error>> {
    let wanted = text(guid);
    catalog["blueprints"]
        .as_array()
        .and_then(|list| list.iter().find(|entry| entry["guid"] == wanted.as_str()))
        .ok_or_else(|| format!("blueprint {wanted}").into())
}
