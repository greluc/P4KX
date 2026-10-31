// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Census of the `Requirement` of the loadout assortments and their groups, and of the ones the
//! Assembly cannot decide.

#![expect(clippy::print_stdout, reason = "the census is the output")]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use p4kx_assembly::{ENTITY_TYPE, LoadoutError, LoadoutResolver};
use p4kx_dataforge::{StructValue, Value};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const ASSORTMENT: &str = "SLoadoutAssortment";
const TAGS: &str = "SLoadoutRequirementTags";

/// Returns the Requirement a field holds, or `None` for a null pointer or no field.
fn requirement<'a, 'f>(structure: &'a StructValue<'f>) -> Option<&'a StructValue<'f>> {
    match structure.field("Requirement") {
        Some(Value::Struct(requirement)) => Some(requirement),
        _ => None,
    }
}

/// Returns whether the Requirement is a tag Requirement that names no tag.
fn names_no_tag(requirement: &StructValue<'_>) -> bool {
    requirement.type_name() == TAGS
        && ["RequiredTags", "BannedTags"].iter().all(
            |name| !matches!(requirement.field(name), Some(Value::Array(tags)) if !tags.is_empty()),
        )
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_requirements_of_the_assortments_are_counted_and_the_undecided_ones_listed() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;

    let mut assortments = 0_usize;
    let mut assortments_required = 0_usize;
    let mut groups = 0_usize;
    let mut unrestricted = 0_usize;
    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    for record in forge
        .records()
        .filter(|record| record.type_name().is_ok_and(|name| name == ASSORTMENT))
    {
        let Ok(tree) = record.tree() else { continue };
        assortments += 1;
        if let Some(Value::Struct(requirement)) = tree.field("Requirement") {
            assortments_required += 1;
            *by_type
                .entry(format!("assortment {}", requirement.type_name()))
                .or_default() += 1;
        }
        if let Some(Value::Array(items)) = tree.field("GroupedLoadouts") {
            for group in items.iter().filter_map(|item| match item {
                Value::Struct(group) => Some(group),
                _ => None,
            }) {
                groups += 1;
                if let Some(requirement) = requirement(group) {
                    let kind = requirement.type_name();
                    *by_type.entry(format!("group {kind}")).or_default() += 1;
                    unrestricted += usize::from(names_no_tag(requirement));
                }
            }
        }
    }
    let groups_required: usize = by_type
        .iter()
        .filter(|(key, _)| key.starts_with("group "))
        .map(|(_, count)| count)
        .sum();
    println!(
        "{assortments} assortments, {assortments_required} with a Requirement; {groups} groups, \
         {groups_required} with a Requirement"
    );
    for (kind, count) in &by_type {
        println!("  {kind}: {count}");
    }
    println!("{unrestricted} group Requirements name no tag and restrict nothing");

    let archive_files = p4kx_assembly::ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&archive_files)
        .stopping_at_docked_vehicles();
    let mut undecided: BTreeSet<(String, String)> = BTreeSet::new();
    let mut entities = 0_usize;
    for record in forge
        .records()
        .filter(|record| record.type_name().is_ok_and(|name| name == ENTITY_TYPE))
    {
        let Ok(node) = resolver.resolve(&record) else {
            continue;
        };
        let mut listed = false;
        for issue in node.issues() {
            if let LoadoutError::AssortmentRequirement { guid, scope } = issue {
                undecided.insert((guid.to_string(), scope.clone()));
                listed = true;
            }
        }
        entities += usize::from(listed);
    }
    println!(
        "{} assortments or groups reached by an Entity depend on spawn tags; {entities} Entities \
         list one",
        undecided.len()
    );

    assert!(assortments > 0, "the Archive holds loadout assortments");
    assert!(
        undecided.len() <= assortments_required + groups_required,
        "only a Requirement that exists can be undecided"
    );
    Ok(())
}
