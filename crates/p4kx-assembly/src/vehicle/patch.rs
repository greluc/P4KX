use std::collections::HashMap;

use crate::markup::Element;

/// One `Elem` of a Modification: the attribute `name` of every element with the `id` `id_ref`
/// is set to `value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Elem {
    id_ref: String,
    name: String,
    value: String,
}

/// A Modification of the base implementation: the patch file it names and its `Elem` overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Modification {
    pub(super) patch_file: Option<String>,
    pub(super) elems: Vec<Elem>,
}

/// Finds the Modification `name` below the `Modifications` element of `vehicle`, preferring an
/// exact name over one that differs in ASCII case.
pub(super) fn find_modification(vehicle: &Element, name: &str) -> Option<Modification> {
    let all: Vec<&Element> = vehicle
        .children_named("Modifications")
        .flat_map(|modifications| modifications.children_named("Modification"))
        .collect();
    let element = all
        .iter()
        .find(|element| element.attribute("name") == Some(name))
        .or_else(|| {
            all.iter().find(|element| {
                element
                    .attribute("name")
                    .is_some_and(|found| found.eq_ignore_ascii_case(name))
            })
        })?;
    Some(Modification {
        patch_file: element
            .attribute("patchFile")
            .filter(|path| !path.trim().is_empty())
            .map(str::to_owned),
        elems: element
            .children_named("Elems")
            .flat_map(|elems| elems.children_named("Elem"))
            .filter_map(|elem| {
                Some(Elem {
                    id_ref: elem.attribute("idRef")?.to_owned(),
                    name: elem.attribute("name")?.to_owned(),
                    value: elem.attribute("value").unwrap_or_default().to_owned(),
                })
            })
            .collect(),
    })
}

/// Applies a patch file to the base implementation `base`.
///
/// The patch root is `Modifications` (or `Vehicle`). A `Vehicle` child, and any child without an
/// `id`, such as `Parts`, replaces the child of the same name of the base wholesale. Any other
/// child with an `id` is merged into every element of the base with that `id`: its attributes
/// are set and its children replace the element's.
pub(super) fn apply_patch(base: &mut Element, patch: &Element) {
    if patch.name.eq_ignore_ascii_case("Vehicle") {
        replace_children(base, patch);
        return;
    }
    for child in &patch.children {
        if child.name.eq_ignore_ascii_case("Vehicle") {
            replace_children(base, child);
        } else if let Some(id) = child.attribute("id") {
            merge_by_id(base, id, child);
        } else {
            replace_child(base, child);
        }
    }
}

fn replace_children(base: &mut Element, source: &Element) {
    for child in &source.children {
        replace_child(base, child);
    }
}

fn replace_child(base: &mut Element, child: &Element) {
    if child.name.eq_ignore_ascii_case("Modifications") {
        return;
    }
    if let Some(slot) = base
        .children
        .iter_mut()
        .find(|existing| existing.name.eq_ignore_ascii_case(&child.name))
    {
        slot.clone_from(child);
    } else {
        base.children.push(child.clone());
    }
}

fn merge_by_id(element: &mut Element, id: &str, patch: &Element) {
    for child in &mut element.children {
        if child.attribute("id") == Some(id) {
            for (name, value) in &patch.attributes {
                child.set_attribute(name, value);
            }
            child.children.clone_from(&patch.children);
        } else if !child.name.eq_ignore_ascii_case("Modifications") {
            merge_by_id(child, id, patch);
        }
    }
}

/// Applies the `Elem` overrides to every element of `base` whose `id` they name.
pub(super) fn apply_elems(base: &mut Element, elems: &[Elem]) {
    if elems.is_empty() {
        return;
    }
    let mut by_id: HashMap<&str, Vec<&Elem>> = HashMap::new();
    for elem in elems {
        by_id.entry(elem.id_ref.as_str()).or_default().push(elem);
    }
    set_elems(base, &by_id);
}

fn set_elems(element: &mut Element, by_id: &HashMap<&str, Vec<&Elem>>) {
    if let Some(id) = element.attribute("id")
        && let Some(elems) = by_id.get(id)
    {
        for elem in elems {
            element.set_attribute(&elem.name, &elem.value);
        }
    }
    for child in &mut element.children {
        if !child.name.eq_ignore_ascii_case("Modifications") {
            set_elems(child, by_id);
        }
    }
}
