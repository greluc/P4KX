// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_cryxml::{Element, Xml};

use crate::merge::merge_by_port;
use crate::spec::{EntrySpec, LoadoutSpec, TargetSpec};
use crate::{LoadoutError, LoadoutFiles, MAX_LOADOUT_DEPTH};

/// The most XML Loadouts one XML Loadout includes through nested `<Loadouts>`.
pub const MAX_XML_INCLUDE_DEPTH: usize = 8;

/// The most XML Loadout files one resolution of an XML Loadout reads, the Loadout itself and
/// everything it includes; further includes are left out with an issue.
pub const MAX_XML_INCLUDES: usize = 256;

/// One XML Loadout file: the Loadouts it includes and its own entries.
struct XmlLoadout {
    includes: Vec<String>,
    entries: Vec<EntrySpec>,
}

/// An element of text XML or `CryXml`, read alike.
trait Node: Copy {
    fn name(self) -> String;
    fn attribute(self, name: &str) -> Option<String>;
    fn children(self) -> Vec<Self>;
}

impl Node for Element<'_> {
    fn name(self) -> String {
        Element::name(self).to_owned()
    }

    fn attribute(self, name: &str) -> Option<String> {
        Element::attribute(self, name).map(str::to_owned)
    }

    fn children(self) -> Vec<Self> {
        Element::children(self).collect()
    }
}

impl Node for roxmltree::Node<'_, '_> {
    fn name(self) -> String {
        self.tag_name().name().to_owned()
    }

    fn attribute(self, name: &str) -> Option<String> {
        roxmltree::Node::attribute(&self, name).map(str::to_owned)
    }

    fn children(self) -> Vec<Self> {
        roxmltree::Node::children(&self)
            .filter(roxmltree::Node::is_element)
            .collect()
    }
}

/// The entries of an XML Loadout and why parts of it cannot be read.
pub(crate) struct XmlEntries {
    pub(crate) entries: Vec<EntrySpec>,
    pub(crate) issues: Vec<LoadoutError>,
}

/// Reads the XML Loadout at `path` with the Loadouts it includes: the entries of the included
/// Loadouts in order, each overridden by the later ones and by the file's own entries per port.
/// An include that cannot be read is an issue; the rest still loads.
pub(crate) fn load(files: &dyn LoadoutFiles, path: &str) -> Result<XmlEntries, LoadoutError> {
    let mut loader = Loader {
        files,
        stack: Vec::new(),
        done: HashMap::new(),
        loads: 0,
        issues: Vec::new(),
    };
    let entries = loader.load(path)?;
    Ok(XmlEntries {
        entries,
        issues: loader.issues,
    })
}

/// Reads one XML Loadout with its includes, each file once.
struct Loader<'l> {
    files: &'l dyn LoadoutFiles,
    stack: Vec<String>,
    done: HashMap<String, Result<Vec<EntrySpec>, LoadoutError>>,
    loads: usize,
    issues: Vec<LoadoutError>,
}

impl Loader<'_> {
    fn load(&mut self, path: &str) -> Result<Vec<EntrySpec>, LoadoutError> {
        if let Some(done) = self.done.get(&path.to_ascii_lowercase()) {
            return done.clone();
        }
        let failed = |reason: String| LoadoutError::XmlLoadout {
            path: path.to_owned(),
            reason,
        };
        if self
            .stack
            .iter()
            .any(|seen| seen.eq_ignore_ascii_case(path))
        {
            return Err(failed("it includes itself".to_owned()));
        }
        if self.stack.len() >= MAX_XML_INCLUDE_DEPTH {
            return Err(failed(format!(
                "it is included deeper than {MAX_XML_INCLUDE_DEPTH} Loadouts"
            )));
        }
        self.loads = self.loads.saturating_add(1);
        let loadout = match self
            .files
            .read(path)
            .map_err(|error| failed(error.to_string()))
            .and_then(|bytes| parse(&bytes).map_err(failed))
        {
            Ok(loadout) => loadout,
            Err(error) => {
                self.done
                    .insert(path.to_ascii_lowercase(), Err(error.clone()));
                return Err(error);
            }
        };
        self.stack.push(path.to_owned());
        let mut entries = Vec::new();
        let mut named = HashSet::new();
        for include in &loadout.includes {
            let key = include.to_ascii_lowercase();
            if !named.insert(key.clone()) {
                continue;
            }
            if self.loads >= MAX_XML_INCLUDES && !self.done.contains_key(&key) {
                self.issues.push(LoadoutError::XmlLoadout {
                    path: include.clone(),
                    reason: format!(
                        "more than {MAX_XML_INCLUDES} Loadouts are read; the rest are left out"
                    ),
                });
                break;
            }
            match self.load(include) {
                Ok(included) => merge_into(&mut entries, included),
                Err(error) => self.issues.push(error),
            }
        }
        self.stack.pop();
        merge_into(&mut entries, loadout.entries);
        self.done
            .insert(path.to_ascii_lowercase(), Ok(entries.clone()));
        Ok(entries)
    }
}

/// Replaces the entry of each port `overrides` names, ignoring ASCII case, and appends the rest.
fn merge_into(entries: &mut Vec<EntrySpec>, overrides: Vec<EntrySpec>) {
    merge_by_port(entries, overrides, |entry| &entry.port);
}
fn parse(bytes: &[u8]) -> Result<XmlLoadout, String> {
    match p4kx_cryxml::read(bytes).map_err(|error| error.to_string())? {
        Xml::CryXml(document) => read_root(document.root()),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).map_err(|error| error.to_string())?;
            let text = text.trim_start_matches('\u{feff}');
            crate::markup::check_depth(text)?;
            let document = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
            read_root(document.root_element())
        }
    }
}

fn read_root<N: Node>(root: N) -> Result<XmlLoadout, String> {
    if root.name() != "Loadout" {
        return Err(format!("its root element is {}, not Loadout", root.name()));
    }
    let mut includes = Vec::new();
    let mut entries = Vec::new();
    for child in root.children() {
        match child.name().as_str() {
            "Loadouts" => includes.extend(
                child
                    .children()
                    .into_iter()
                    .filter_map(|include| include.attribute("loadout"))
                    .filter(|path| !path.is_empty()),
            ),
            "Items" => entries.extend(read_items(child, 0)?),
            _ => {}
        }
    }
    Ok(XmlLoadout { includes, entries })
}

fn read_items<N: Node>(items: N, depth: usize) -> Result<Vec<EntrySpec>, String> {
    if depth >= MAX_LOADOUT_DEPTH {
        return Err(format!(
            "its items are nested deeper than {MAX_LOADOUT_DEPTH}"
        ));
    }
    let mut entries = Vec::new();
    for item in items.children() {
        if item.name() != "Item" {
            continue;
        }
        let class_name = item.attribute("itemName").unwrap_or_default();
        let mut nested = Vec::new();
        for child in item.children() {
            if child.name() == "Items" {
                nested.extend(read_items(child, depth.saturating_add(1))?);
            }
        }
        entries.push(EntrySpec {
            port: item.attribute("portName").unwrap_or_default(),
            target: if class_name.is_empty() {
                TargetSpec::None
            } else {
                TargetSpec::ClassName(class_name)
            },
            loadout: (!nested.is_empty()).then(|| LoadoutSpec::Manual(Arc::from(nested))),
        });
    }
    Ok(entries)
}
