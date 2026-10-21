use std::collections::{HashMap, HashSet};

use crate::markup::{Budget, Element};

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

/// What applying a Modification did, to judge the patch rule of ADR-0050 on real data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModificationReport {
    pub(crate) vehicle_children: usize,
    pub(crate) wholesale_children: usize,
    pub(crate) id_children: usize,
    pub(crate) unmatched_children: Vec<String>,
    pub(crate) unmatched_ids: Vec<String>,
    pub(crate) replaced_ids: Vec<String>,
    pub(crate) unmatched_elems: Vec<String>,
}

impl ModificationReport {
    /// Returns how many children of the patch came from a `Vehicle` element and replaced the
    /// base's child of the same name.
    #[must_use]
    pub const fn vehicle_children(&self) -> usize {
        self.vehicle_children
    }

    /// Returns how many children of the patch root had no `id` and replaced the base's child of
    /// the same name, e.g. `Parts`.
    #[must_use]
    pub const fn wholesale_children(&self) -> usize {
        self.wholesale_children
    }

    /// Returns how many children of the patch root had an `id` to merge by.
    #[must_use]
    pub const fn id_children(&self) -> usize {
        self.id_children
    }

    /// Returns the names of the patch's replacing children the base has no child of.
    #[must_use]
    pub fn unmatched_children(&self) -> &[String] {
        &self.unmatched_children
    }

    /// Returns the `id`s of the patch that no base element has.
    #[must_use]
    pub fn unmatched_ids(&self) -> &[String] {
        &self.unmatched_ids
    }

    /// Returns the `id`s of the patch's wrappers that no element has, and none of whose children
    /// has an `id` the base's child of that name has either, so they replaced that child wholesale.
    #[must_use]
    pub fn replaced_ids(&self) -> &[String] {
        &self.replaced_ids
    }

    /// Returns the `idRef`s of the Modification's `Elem`s that no element has.
    #[must_use]
    pub fn unmatched_elems(&self) -> &[String] {
        &self.unmatched_elems
    }
}

/// Returns the `id` of `element`, none when it has none or an empty one.
fn id_of(element: &Element) -> Option<&str> {
    element.attribute("id").filter(|id| !id.is_empty())
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
                    id_ref: elem
                        .attribute("idRef")
                        .filter(|id| !id.is_empty())?
                        .to_owned(),
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
/// are set and its children replace the element's. A child with an `id` that no base element has
/// is merged into the base's child of its name by the `id`s of its descendants, or replaces it
/// wholesale when none matches, as a wrapper such as `<Parts id="modParts">` does.
///
/// # Errors
///
/// Returns why the budget is spent; `base` is then partly patched and must be discarded.
pub(super) fn apply_patch(
    base: &mut Element,
    patch: &Element,
    budget: &mut Budget,
) -> Result<ModificationReport, String> {
    let mut report = ModificationReport::default();
    if patch.name.eq_ignore_ascii_case("Vehicle") {
        replace_children(base, patch, &mut report, budget)?;
        return Ok(report);
    }
    for child in &patch.children {
        if child.name.eq_ignore_ascii_case("Vehicle") {
            replace_children(base, child, &mut report, budget)?;
        } else {
            apply_child(base, child, &mut report, budget)?;
        }
    }
    Ok(report)
}

/// Applies one child of the patch root that is not a `Vehicle`: a child with an `id` some base
/// element has is merged by `id`; a wrapper whose `id` no element has but whose name a base child
/// has is merged into that child by the `id`s of its descendants, or replaces it wholesale when
/// none of them matches; anything else replaces the base's child of its name wholesale.
fn apply_child(
    base: &mut Element,
    child: &Element,
    report: &mut ModificationReport,
    budget: &mut Budget,
) -> Result<(), String> {
    let Some(id) = id_of(child) else {
        report.wholesale_children = report.wholesale_children.saturating_add(1);
        return replace_child(base, child, report, budget);
    };
    if merge_by_id(base, id, child, budget)? > 0 {
        report.id_children = report.id_children.saturating_add(1);
        return Ok(());
    }
    let Some(at) = base
        .children
        .iter()
        .position(|existing| existing.name.eq_ignore_ascii_case(&child.name))
    else {
        report.unmatched_ids.push(id.to_owned());
        return Ok(());
    };
    let mut matched = 0_usize;
    let mut missed = Vec::new();
    if let Some(target) = base.children.get_mut(at) {
        let mut addressed = Vec::new();
        top_ids(child, &mut addressed);
        for part in addressed {
            let Some(part_id) = id_of(part) else {
                continue;
            };
            if merge_by_id(target, part_id, part, budget)? > 0 {
                matched = matched.saturating_add(1);
            } else {
                missed.push(part_id.to_owned());
            }
        }
    }
    if matched > 0 {
        report.id_children = report.id_children.saturating_add(1);
        report.unmatched_ids.extend(missed);
    } else {
        report.replaced_ids.push(id.to_owned());
        report.wholesale_children = report.wholesale_children.saturating_add(1);
        replace_child(base, child, report, budget)?;
    }
    Ok(())
}

/// Collects the descendants of `element` that have an `id`, not looking below one that has.
fn top_ids<'a>(element: &'a Element, found: &mut Vec<&'a Element>) {
    for child in &element.children {
        if id_of(child).is_some() {
            found.push(child);
        } else {
            top_ids(child, found);
        }
    }
}

fn replace_children(
    base: &mut Element,
    source: &Element,
    report: &mut ModificationReport,
    budget: &mut Budget,
) -> Result<(), String> {
    for child in &source.children {
        report.vehicle_children = report.vehicle_children.saturating_add(1);
        replace_child(base, child, report, budget)?;
    }
    Ok(())
}

fn replace_child(
    base: &mut Element,
    child: &Element,
    report: &mut ModificationReport,
    budget: &mut Budget,
) -> Result<(), String> {
    if child.name.eq_ignore_ascii_case("Modifications") {
        return Ok(());
    }
    budget.charge_copy(child)?;
    if let Some(slot) = base
        .children
        .iter_mut()
        .find(|existing| existing.name.eq_ignore_ascii_case(&child.name))
    {
        slot.clone_from(child);
    } else {
        report.unmatched_children.push(child.name.clone());
        base.children.push(child.clone());
    }
    Ok(())
}

fn merge_by_id(
    element: &mut Element,
    id: &str,
    patch: &Element,
    budget: &mut Budget,
) -> Result<usize, String> {
    let mut matched = 0_usize;
    for child in &mut element.children {
        if id_of(child) == Some(id) {
            matched = matched.saturating_add(1);
            for (name, value) in &patch.attributes {
                budget.charge_set(child.attributes.len(), name, value)?;
                child.set_attribute(name, value);
            }
            for grandchild in &patch.children {
                budget.charge_copy(grandchild)?;
            }
            child.children.clone_from(&patch.children);
        } else if !child.name.eq_ignore_ascii_case("Modifications") {
            matched = matched.saturating_add(merge_by_id(child, id, patch, budget)?);
        }
    }
    Ok(matched)
}

/// Applies the `Elem` overrides to every element of `base` whose `id` they name; returns the
/// `idRef`s that no element has.
///
/// # Errors
///
/// Returns why the budget is spent; `base` is then partly changed and must be discarded.
pub(super) fn apply_elems(
    base: &mut Element,
    elems: &[Elem],
    budget: &mut Budget,
) -> Result<Vec<String>, String> {
    if elems.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_id: HashMap<&str, Vec<&Elem>> = HashMap::new();
    for elem in elems {
        by_id.entry(elem.id_ref.as_str()).or_default().push(elem);
    }
    let mut seen = HashSet::new();
    set_elems(base, &by_id, &mut seen, budget)?;
    let mut missed: Vec<String> = by_id
        .keys()
        .filter(|id| !seen.contains(**id))
        .map(|id| (*id).to_owned())
        .collect();
    missed.sort_unstable();
    Ok(missed)
}

fn set_elems<'e>(
    element: &mut Element,
    by_id: &HashMap<&'e str, Vec<&Elem>>,
    seen: &mut HashSet<&'e str>,
    budget: &mut Budget,
) -> Result<(), String> {
    if let Some(id) = id_of(element)
        && let Some((key, elems)) = by_id.get_key_value(id)
    {
        seen.insert(*key);
        for elem in elems {
            budget.charge_set(element.attributes.len(), &elem.name, &elem.value)?;
            element.set_attribute(&elem.name, &elem.value);
        }
    }
    for child in &mut element.children {
        if !child.name.eq_ignore_ascii_case("Modifications") {
            set_elems(child, by_id, seen, budget)?;
        }
    }
    Ok(())
}
