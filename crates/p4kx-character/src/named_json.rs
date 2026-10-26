// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeSet;

use crate::json_tree::Node;
use crate::{Chf, ChfError, NameTable};

/// The suffix of the output-only key that names the hash in the key before it.
pub const LABEL_SUFFIX: &str = "_label";

const LABELLED: [&str; 4] = [
    "gender_hash_label",
    "port_label",
    "attachment_label",
    "name_label",
];

/// How many hashes of a CHF [`Chf::to_json_named`] could name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameSummary {
    /// The hashes that got a label, counted where they occur.
    pub named: usize,
    /// The hashes that got none, counted where they occur.
    pub unresolved: usize,
    /// The distinct hashes that got none, in ascending order.
    pub unresolved_hashes: Vec<u32>,
}

#[derive(Default)]
struct Tally {
    named: usize,
    unresolved: BTreeSet<u32>,
    unresolved_total: usize,
}

impl Tally {
    fn label<'n>(&mut self, object: &mut Node, key: &str, lookup: impl Fn(u32) -> Option<&'n str>) {
        let Node::Object(fields) = object else {
            return;
        };
        let Some(index) = fields.iter().position(|(name, _)| name == key) else {
            return;
        };
        let Some(hash) = fields.get(index).and_then(|(_, value)| match value {
            Node::Unsigned(hash) => u32::try_from(*hash).ok(),
            _ => None,
        }) else {
            return;
        };
        if let Some(name) = lookup(hash) {
            self.named += 1;
            fields.insert(
                index + 1,
                (format!("{key}{LABEL_SUFFIX}"), Node::Text(name.to_owned())),
            );
        } else {
            self.unresolved_total += 1;
            self.unresolved.insert(hash);
        }
    }

    fn finish(self) -> NameSummary {
        NameSummary {
            named: self.named,
            unresolved: self.unresolved_total,
            unresolved_hashes: self.unresolved.into_iter().collect(),
        }
    }
}

fn label_ports(port: &mut Node, names: &NameTable, tally: &mut Tally) {
    tally.label(port, "port", |hash| names.port(hash));
    if let Some(children) = port.field_mut("children") {
        for child in children.items_mut() {
            label_ports(child, names, tally);
        }
    }
}

fn label_material(material: &mut Node, names: &NameTable, tally: &mut Tally) {
    tally.label(material, "attachment", |hash| names.attachment(hash));
    let Some(subs) = material.field_mut("sub_materials") else {
        return;
    };
    for sub in subs.items_mut() {
        tally.label(sub, "name", |hash| names.sub_material(hash));
        for key in ["floats", "colors"] {
            if let Some(params) = sub.field_mut(key) {
                for param in params.items_mut() {
                    tally.label(param, "name", |hash| names.param(hash));
                }
            }
        }
    }
}

fn strip(node: &mut Node) {
    match node {
        Node::Object(fields) => {
            fields.retain(|(key, _)| !LABELLED.contains(&key.as_str()));
            for (_, value) in fields {
                strip(value);
            }
        }
        Node::List(items) => {
            for item in items {
                strip(item);
            }
        }
        _ => {}
    }
}

impl Chf {
    /// Returns the JSON form of [`Chf::to_json`] with an output-only label next to every hash that
    /// `names` can read: `gender_hash_label`, `port_label`, `attachment_label` and `name_label`
    /// follow the hash they name. [`Chf::from_json`] ignores them.
    ///
    /// # Errors
    ///
    /// [`ChfError::Json`] when serialisation fails.
    pub fn to_json_named(&self, names: &NameTable) -> Result<(String, NameSummary), ChfError> {
        let mut tree: Node = serde_json::from_str(&serde_json::to_string(self)?)?;
        let mut tally = Tally::default();
        if let Some(character) = tree.field_mut("character") {
            if let Some(dna) = character.field_mut("dna") {
                tally.label(dna, "gender_hash", |hash| names.rig(hash));
            }
            if let Some(root) = character.field_mut("item_ports") {
                label_ports(root, names, &mut tally);
            }
            if let Some(materials) = character.field_mut("materials") {
                for material in materials.items_mut() {
                    label_material(material, names, &mut tally);
                }
            }
        }
        Ok((serde_json::to_string_pretty(&tree)?, tally.finish()))
    }

    pub(crate) fn without_labels(json: &[u8]) -> Result<Vec<u8>, serde_json::Error> {
        let mut tree: Node = serde_json::from_slice(json)?;
        strip(&mut tree);
        serde_json::to_vec(&tree)
    }
}

pub(crate) fn has_labels(json: &[u8]) -> bool {
    let needle = LABEL_SUFFIX.as_bytes();
    json.windows(needle.len() + 1)
        .any(|window| window.starts_with(needle) && window.ends_with(b"\""))
}
