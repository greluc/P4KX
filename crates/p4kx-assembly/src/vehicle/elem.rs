// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::markup::Element;

/// A Modification of the base implementation: the patch file it names and its `Elem` overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Modification {
    pub(super) patch_file: Option<String>,
    pub(super) elems: Vec<ElemOverride>,
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
                Some(ElemOverride {
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

/// One `Elem` of a Modification: the attribute `name` of every element with the `id` `id_ref`
/// is set to `value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElemOverride {
    pub(super) id_ref: String,
    pub(super) name: String,
    pub(super) value: String,
}

impl ElemOverride {
    /// Returns the `id` of the elements the override addresses.
    #[must_use]
    pub fn id_ref(&self) -> &str {
        &self.id_ref
    }

    /// Returns the name of the attribute the override sets.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value the override sets.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}
