// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Weak pointers into class arrays and owned instances of the real DataForge.

use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_extraction::{
    Archive, DataForge, DataForgeEntry, JsonLayout, Record, RecordJson, RecordTree, RecordXml,
    Reference, TreeBuilder, Value, ValueError,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use super::{TestResult, report};

const CONVERSATION: &str = "libs/foundry/records/conversation/arenacommander/crusader/charlie/conversation.charlie_job_offer";
const CONVERSATION_PREFIX: &str = "libs/foundry/records/conversation/";
const CONVERSATION_TYPE: &str = "Conversation";
const START_NODE: &str = "ConversationNode_Start";
const ELIMINATE_ALL: &str =
    "libs/foundry/records/missiondata/declarations/moduledeclarations/eliminateall";

/// The number of weak pointers of the whole DataForge that end as `OutsideRecord`; LIVE 4.10 has
/// none, so any owned instance the builder fails to track shows up as a failure.
const MAX_OUTSIDE_RECORD: usize = 0;

fn open() -> Result<DataForgeEntry, Box<dyn std::error::Error>> {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    Ok(DataForgeEntry::read(&archive)?)
}

fn field<'t, 'f>(
    fields: &'t [p4kx_extraction::Field<'f>],
    name: &str,
) -> Result<&'t Value<'f>, String> {
    fields
        .iter()
        .find(|field| field.name() == name)
        .map(p4kx_extraction::Field::value)
        .ok_or_else(|| format!("no field {name}"))
}

fn element<'t, 'f>(value: &'t Value<'f>, index: usize) -> Result<&'t Value<'f>, String> {
    match value {
        Value::Array(items) => items
            .get(index)
            .ok_or_else(|| format!("no element {index}")),
        other => Err(format!("no array: {other:?}")),
    }
}

fn fields_of<'t, 'f>(value: &'t Value<'f>) -> Result<&'t [p4kx_extraction::Field<'f>], String> {
    match value {
        Value::Struct(structure) => Ok(structure.fields()),
        other => Err(format!("no struct: {other:?}")),
    }
}

fn weak_link<'t, 'f>(value: &'t Value<'f>) -> Result<&'t Reference<'f>, String> {
    match value {
        Value::Reference(link) => Ok(link),
        other => Err(format!("no Reference: {other:?}")),
    }
}

fn expect_link(tree: &RecordTree<'_>, link: &Reference<'_>, at: &str, type_name: &str) {
    assert_eq!(link.guid(), tree.guid(), "the Record itself");
    assert_eq!(
        link.record_path(),
        tree.record_path(),
        "the same record path"
    );
    assert_eq!(
        link.location().map(ToString::to_string).as_deref(),
        Some(at),
        "the location of the target"
    );
    assert_eq!(link.type_name(), type_name, "the type of the target");
}

fn tree_of<'f>(
    forge: &'f DataForge<'f>,
    path: &str,
) -> Result<RecordTree<'f>, Box<dyn std::error::Error>> {
    let record = forge
        .records_at_path(path)
        .next()
        .ok_or_else(|| format!("the Record {path} no longer exists in this Build"))?;
    Ok(record.tree()?)
}

fn start_next<'t, 'f>(tree: &'t RecordTree<'f>) -> Result<&'t Value<'f>, String> {
    let start = element(field(tree.fields(), "fragments")?, 0)?;
    field(fields_of(start)?, "next")
}

fn is_start_with_link(record: &Record<'_>) -> bool {
    let Ok(tree) = record.tree() else {
        return false;
    };
    start_next(&tree)
        .is_ok_and(|next| matches!(next, Value::Reference(link) if link.location().is_some()))
        && tree
            .field("fragments")
            .and_then(|fragments| element(fragments, 0).ok())
            .is_some_and(|start| matches!(start, Value::Struct(s) if s.type_name() == START_NODE))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_start_node_of_a_conversation_links_to_its_first_dialogue_node() -> TestResult {
    let entry = open()?;
    let forge = entry.open()?;
    let path = if forge.records_at_path(CONVERSATION).next().is_some() {
        CONVERSATION.to_owned()
    } else {
        let other = forge
            .records()
            .filter(|record| record.type_name().is_ok_and(|t| t == CONVERSATION_TYPE))
            .filter(|record| {
                record
                    .record_path()
                    .is_ok_and(|path| path.starts_with(CONVERSATION_PREFIX))
            })
            .find(is_start_with_link)
            .ok_or("no Conversation Record with a start node that links")?;
        let path = other.record_path()?.to_owned();
        report(&format!(
            "{CONVERSATION} no longer exists in this Build; checking {path} instead"
        ));
        path
    };
    let tree = tree_of(&forge, &path)?;

    let link = weak_link(start_next(&tree)?)?;
    let at = link.location().ok_or("no location")?.to_string();
    expect_link(&tree, link, &at, "ConversationNode_Dialogue");
    if path == CONVERSATION {
        assert_eq!(at, "fragments[1]", "the first dialogue node");
    }
    let (name, index) = at
        .strip_suffix(']')
        .and_then(|at| at.split_once('['))
        .ok_or("a class array location")?;
    let target = element(field(tree.fields(), name)?, index.parse()?)?;
    assert!(
        matches!(target, Value::Struct(s) if s.type_name() == link.type_name()),
        "the location names a node of that type: {target:?}"
    );

    check_xml(&tree, link, &at)?;
    check_json(&tree, link, &at)
}

fn check_xml(tree: &RecordTree<'_>, link: &Reference<'_>, at: &str) -> TestResult {
    let mut xml = Vec::new();
    RecordXml::new().write(tree, &mut xml)?;
    let xml = String::from_utf8(xml)?;
    let document = roxmltree::Document::parse(&xml)?;
    let reference = document
        .descendants()
        .find(|node| node.has_tag_name("reference") && node.attribute("at") == Some(at))
        .ok_or("no p4kx:reference with that location in the Record XML")?;
    assert_eq!(
        reference.attribute("guid"),
        Some(tree.guid().to_string().as_str()),
        "XML guid"
    );
    assert_eq!(
        reference.attribute("path"),
        Some(tree.record_path()),
        "XML path"
    );
    assert_eq!(
        reference.attribute("type"),
        Some(link.type_name()),
        "XML type"
    );
    let holder = reference
        .parent()
        .and_then(|next| next.parent())
        .ok_or("a holder")?;
    assert_eq!(
        holder.tag_name().name(),
        START_NODE,
        "held by the start node"
    );
    Ok(())
}

fn check_json(tree: &RecordTree<'_>, link: &Reference<'_>, at: &str) -> TestResult {
    let mut json = Vec::new();
    RecordJson::new(JsonLayout::Pretty).write(tree, &mut json)?;
    let json: serde_json::Value = serde_json::from_slice(&json)?;
    let next = json
        .pointer("/fragments/0/next")
        .ok_or("no link at fragments[0].next in the Record JSON")?;
    let text = |key: &str| next.get(key).and_then(serde_json::Value::as_str);
    assert_eq!(
        text("guid"),
        Some(tree.guid().to_string().as_str()),
        "JSON guid"
    );
    assert_eq!(text("path"), Some(tree.record_path()), "JSON path");
    assert_eq!(text("type"), Some(link.type_name()), "JSON type");
    assert_eq!(text("at"), Some(at), "JSON location");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_variable_of_eliminateall_links_to_an_entry_under_location_entities() -> TestResult {
    let entry = open()?;
    let forge = entry.open()?;
    let tree = tree_of(&forge, ELIMINATE_ALL)?;

    let variable = element(field(tree.fields(), "declaredVariables")?, 0)?;
    let entities = field(fields_of(variable)?, "locationEntities")?;
    let link = weak_link(element(entities, 0)?)?;

    expect_link(
        &tree,
        link,
        "locationEntities[0]",
        "ModuleLocationEntities_Static",
    );
    let target = element(field(tree.fields(), "locationEntities")?, 0)?;
    assert!(
        matches!(target, Value::Struct(s) if s.type_name() == "ModuleLocationEntities_Static"),
        "the location names that entry: {target:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn no_weak_pointer_of_the_dataforge_ends_outside_its_record() -> TestResult {
    let entry = open()?;
    let forge = entry.open()?;
    let linked = AtomicUsize::new(0);
    let outside = AtomicUsize::new(0);
    let examples: std::sync::Mutex<Vec<String>> = std::sync::Mutex::default();

    forge
        .records()
        .par_bridge()
        .for_each_init(TreeBuilder::new, |builder, record| {
            let Ok(tree) = builder.build(&record) else {
                return;
            };
            for (location, error) in tree.located_errors() {
                if let ValueError::OutsideRecord { .. } = error {
                    outside.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut found) = examples.lock()
                        && found.len() < 10
                    {
                        found.push(format!("{}: {location}", tree.record_path()));
                    }
                }
            }
            linked.fetch_add(count_inside_links(tree.root().fields()), Ordering::Relaxed);
        });

    let outside = outside.into_inner();
    let linked = linked.into_inner();
    report(&format!(
        "weak pointers into their own Record: {linked}; ending as OutsideRecord: {outside}; e.g. {:?}",
        examples.into_inner()?
    ));
    assert!(
        linked > 0,
        "the DataForge has weak pointers into their own Record"
    );
    assert_eq!(
        outside, MAX_OUTSIDE_RECORD,
        "weak pointers end as OutsideRecord"
    );
    Ok(())
}

fn count_inside_links(fields: &[p4kx_extraction::Field<'_>]) -> usize {
    fields.iter().map(|field| count_value(field.value())).sum()
}

fn count_value(value: &Value<'_>) -> usize {
    match value {
        Value::Reference(link) => usize::from(link.location().is_some()),
        Value::Struct(structure) => count_inside_links(structure.fields()),
        Value::Array(items) => items.iter().map(count_value).sum(),
        _ => 0,
    }
}
