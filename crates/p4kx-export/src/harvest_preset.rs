// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForge, Guid, RecordTree, StructValue, TreeBuilder, Value};

use crate::contract_harvest::{
    AreaModifier, HarvestableArea, HarvestableElement, HarvestableGroup, HarvestableProviderPreset,
};
use crate::guid_text::CatalogGuid;
use crate::harvest_link::Links;
use crate::record_fields::{class_name, elements, number, reference, structs, structure, text};

const GROUPS_FIELD: &str = "harvestableGroups";
const HARVESTABLES_FIELD: &str = "harvestables";

/// Returns the provider preset the Record tree `tree` describes; what its harvestables spawn is
/// followed through `links`.
pub(crate) fn provider_preset<'f>(
    trees: &mut TreeBuilder<'f>,
    tree: &RecordTree<'f>,
    forge: &'f DataForge<'f>,
    links: &Links<'f>,
) -> HarvestableProviderPreset<'f> {
    let root = Some(tree.root());
    HarvestableProviderPreset {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        groups: structs(root, GROUPS_FIELD)
            .map(|group| group_of(trees, group, forge, links))
            .collect(),
        areas: structs(root, "areas")
            .map(|area| area_of(area, tree.guid()))
            .collect(),
    }
}

fn group_of<'f>(
    trees: &mut TreeBuilder<'f>,
    group: &StructValue<'f>,
    forge: &'f DataForge<'f>,
    links: &Links<'f>,
) -> HarvestableGroup<'f> {
    let group_value = Some(group);
    HarvestableGroup {
        group_name: text(group_value, "groupName"),
        group_probability: number(group_value, "groupProbability"),
        harvestables: structs(group_value, HARVESTABLES_FIELD)
            .map(|element| element_of(trees, element, forge, links))
            .collect(),
    }
}

fn element_of<'f>(
    trees: &mut TreeBuilder<'f>,
    element: &StructValue<'f>,
    forge: &'f DataForge<'f>,
    links: &Links<'f>,
) -> HarvestableElement<'f> {
    let value = Some(element);
    HarvestableElement {
        harvestable: links.of_element(trees, forge, element),
        setup_guid: reference(value, "harvestableSetup"),
        clustering_guid: reference(value, "clustering"),
        relative_probability: number(value, "relativeProbability"),
        geometry_tag_guids: geometry_tags(value),
    }
}

/// Returns the GUIDs of the `Tag` Records the `geometries` of `parent` link to.
pub(crate) fn geometry_tags(parent: Option<&StructValue<'_>>) -> Vec<CatalogGuid> {
    structs(parent, "geometries")
        .filter_map(|geometry| reference(Some(geometry), "tag"))
        .collect()
}

fn area_of<'f>(area: &StructValue<'f>, own: Guid) -> HarvestableArea<'f> {
    let value = Some(area);
    let area_type = structure(value, "areaType");
    HarvestableArea {
        name: text(value, "debugGroupName"),
        global_modifier: number(value, "globalModifier"),
        object_preset_paths: elements(area_type, "objectPresetPaths")
            .iter()
            .filter_map(|path| match path {
                Value::String(path) => Some(*path),
                _ => None,
            })
            .collect(),
        modifiers: structs(value, "modifiers")
            .map(|modifier| modifier_of(modifier, own))
            .collect(),
    }
}

fn modifier_of(modifier: &StructValue<'_>, own: Guid) -> AreaModifier {
    let value = Some(modifier);
    let place = match modifier.field("harvestableElement") {
        Some(Value::Reference(pointer)) if pointer.guid() == own => pointer
            .location()
            .and_then(|location| element_place(location.as_str())),
        _ => None,
    };
    AreaModifier {
        group_index: place.map(|(group, _)| group),
        harvestable_index: place.map(|(_, element)| element),
        modifier: number(value, "harvestableModifier"),
        geometry_tag_guids: geometry_tags(value),
    }
}

/// Returns the group and element index a weak pointer location such as
/// `harvestableGroups[1].harvestables[0]` names.
fn element_place(location: &str) -> Option<(u32, u32)> {
    let rest = location.strip_prefix(GROUPS_FIELD)?.strip_prefix('[')?;
    let (group, rest) = rest.split_once(']')?;
    let element = rest
        .strip_prefix('.')?
        .strip_prefix(HARVESTABLES_FIELD)?
        .strip_prefix('[')?
        .strip_suffix(']')?;
    Some((group.parse().ok()?, element.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::element_place;

    #[test]
    fn a_pointer_into_a_group_names_both_indices() {
        assert_eq!(
            element_place("harvestableGroups[1].harvestables[0]"),
            Some((1, 0))
        );
        assert_eq!(
            element_place("harvestableGroups[12].harvestables[34]"),
            Some((12, 34))
        );
    }

    #[test]
    fn any_other_pointer_names_no_place() {
        for location in [
            "",
            "areas[0]",
            "harvestableGroups[1]",
            "harvestableGroups[x].harvestables[0]",
            "harvestableGroups[1].harvestables[0].more",
            "harvestableGroups[-1].harvestables[0]",
        ] {
            assert_eq!(element_place(location), None, "{location}");
        }
    }
}
