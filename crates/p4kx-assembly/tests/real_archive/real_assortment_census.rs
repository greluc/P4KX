// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Census of the Entities whose default Loadout is generated from loadout assortments: which
//! kinds of Entity they are, and how many of them get ports filled by it.

#![expect(clippy::print_stdout, reason = "the census is the output")]

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{ArchiveFiles, Attachment, LoadoutResolver};
use p4kx_dataforge::{StructValue, Value};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const GENERATED: &str = "SItemPortLoadoutRandomGeneratedParams";
const DEFAULT_LOADOUT: &str = "SEntityComponentDefaultLoadoutParams";
const ENTITY_FOLDER_SEGMENTS: usize = 2;
const MAX_EXAMPLES: usize = 3;

/// Returns whether the struct or anything nested in it is a generated Loadout.
fn holds_generated(structure: &StructValue<'_>) -> bool {
    structure.type_name() == GENERATED
        || structure
            .fields()
            .iter()
            .any(|field| value_holds_generated(field.value()))
}

fn value_holds_generated(value: &Value<'_>) -> bool {
    match value {
        Value::Struct(structure) => holds_generated(structure),
        Value::Array(items) => items.iter().any(value_holds_generated),
        _ => false,
    }
}

/// Returns the first folders of a record path below `libs/foundry/records/entities`.
fn kind_of(record_path: &str) -> String {
    let below = record_path
        .strip_prefix("libs/foundry/records/")
        .unwrap_or(record_path);
    below
        .split('/')
        .take(ENTITY_FOLDER_SEGMENTS)
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Default)]
struct Row {
    entities: usize,
    with_attachments: usize,
    rigged: usize,
    examples: Vec<String>,
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_census_lists_the_entities_whose_default_loadout_is_generated() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for record in forge.records().filter(|record| {
        record
            .type_name()
            .is_ok_and(|name| name == p4kx_assembly::ENTITY_TYPE)
    }) {
        let Ok(tree) = record.tree() else { continue };
        let generated = match tree.field("Components") {
            Some(Value::Array(items)) => items.iter().any(|item| match item {
                Value::Struct(component) if component.type_name() == DEFAULT_LOADOUT => {
                    holds_generated(component)
                }
                _ => false,
            }),
            _ => false,
        };
        if !generated {
            continue;
        }
        let Ok(node) = resolver.resolve(&record) else {
            continue;
        };
        let attached = node
            .walk()
            .iter()
            .any(|(_, port)| matches!(port.attachment(), Attachment::Entity(_)));
        let row = rows.entry(kind_of(tree.record_path())).or_default();
        row.entities += 1;
        row.with_attachments += usize::from(attached);
        row.rigged += usize::from(
            node.geometry()
                .is_some_and(p4kx_assembly::Geometry::is_rigged_file_outside_human_rigs),
        );
        if row.examples.len() < MAX_EXAMPLES {
            row.examples.push(node.class_name().to_owned());
        }
    }
    println!(
        "record folder: entities with a generated default Loadout, with an Entity attached, \
         with a rigged root outside the human rigs"
    );
    for (kind, row) in &rows {
        println!(
            "{kind}: {}, {}, {} (e.g. {})",
            row.entities,
            row.with_attachments,
            row.rigged,
            row.examples.join(", ")
        );
    }
    assert!(!rows.is_empty(), "some Entity has a generated Loadout");
    Ok(())
}
