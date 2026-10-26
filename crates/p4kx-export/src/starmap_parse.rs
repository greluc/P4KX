// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Guid};
use rayon::prelude::*;

use crate::starmap_index::Index;
use crate::starmap_read::main_xml;
use crate::starmap_report::clip;
use crate::starmap_transform::Rotation;
use crate::starmap_xml::{Node, attribute, first_below, pose};

const CHILDREN: &str = "ChildObjectContainers";
const CHILD: &str = "Child";

/// The position and rotation of a Child, or why it has none.
pub(crate) enum Pose {
    Valid {
        position: [f64; 3],
        rotation: Rotation,
    },
    Malformed(String),
}

/// One Child element of a container.
pub(crate) struct Child {
    pub(crate) pose: Pose,
    pub(crate) record: Option<Guid>,
    /// The `entityName`, kept only for a Child that names a record.
    pub(crate) entity_name: String,
    /// The path of the external container the Child names.
    pub(crate) external: Option<String>,
    pub(crate) below: Level,
}

/// The Children of one element.
#[derive(Default)]
pub(crate) struct Level {
    pub(crate) children: Vec<Child>,
    /// The list lies beyond the depth limit and was not read.
    pub(crate) capped: bool,
    /// The element holds more than one `ChildObjectContainers`; only the first was read.
    pub(crate) extra_lists: bool,
}

/// What reading one container found.
pub(crate) enum Body {
    Unreadable(String),
    Unparsed,
    Parsed {
        root: Level,
        fallback: Option<String>,
    },
}

/// Returns the Children of `node`, down to `depth_limit` nested lists; `depth` is the number of
/// lists above it.
fn level<'d, N: Node<'d>>(node: N, depth: usize, depth_limit: usize) -> Level {
    if depth >= depth_limit {
        return Level {
            capped: true,
            ..Level::default()
        };
    }
    let Some(list) = first_below(node, CHILDREN) else {
        return Level::default();
    };
    let extra_lists = node
        .children()
        .filter(|sibling| sibling.name().eq_ignore_ascii_case(CHILDREN))
        .nth(1)
        .is_some();
    let children = list
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case(CHILD))
        .map(|child| read_child(child, depth, depth_limit))
        .collect();
    Level {
        children,
        capped: false,
        extra_lists,
    }
}

fn read_child<'d, N: Node<'d>>(child: N, depth: usize, depth_limit: usize) -> Child {
    let record = attribute(child, "starMapRecord")
        .and_then(|text| text.trim().parse::<Guid>().ok())
        .filter(|guid| guid.stored() != [0; 16]);
    let external = (attribute(child, "external") == Some("1"))
        .then(|| attribute(child, "name").filter(|path| !path.is_empty()))
        .flatten()
        .map(str::to_owned);
    let pose = match pose(child) {
        Some((position, rotation)) => Pose::Valid { position, rotation },
        None => Pose::Malformed(format!(
            "malformed or out of range pos {} rot {} of {}",
            clip(attribute(child, "pos").unwrap_or_default()),
            clip(attribute(child, "rot").unwrap_or_default()),
            clip(attribute(child, "entityName").unwrap_or_default())
        )),
    };
    Child {
        pose,
        entity_name: if record.is_some() {
            attribute(child, "entityName")
                .unwrap_or_default()
                .to_owned()
        } else {
            String::new()
        },
        record,
        external,
        below: level(child, depth + 1, depth_limit),
    }
}

/// Reads and parses the container at `entry`.
fn parse(archive: &Archive, index: &Index<'_>, entry: usize, depth_limit: usize) -> Body {
    let Some(socpak) = index.entries.get(entry) else {
        return Body::Unreadable("no such entry".to_owned());
    };
    let main = match main_xml(archive, socpak) {
        Ok(main) => main,
        Err(reason) => return Body::Unreadable(reason),
    };
    let root = match cryxml::read(&main.bytes) {
        Ok(Xml::CryXml(document)) => level(document.root(), 0, depth_limit),
        Ok(Xml::Text(text)) => {
            let parsed = std::str::from_utf8(text)
                .ok()
                .map(|text| text.trim_start_matches('\u{feff}'))
                .and_then(|text| roxmltree::Document::parse(text).ok());
            match parsed {
                Some(document) => level(document.root_element(), 0, depth_limit),
                None => return Body::Unparsed,
            }
        }
        Err(_) => return Body::Unparsed,
    };
    Body::Parsed {
        root,
        fallback: main.fallback,
    }
}

fn externals<'l>(level: &'l Level, out: &mut Vec<&'l str>) {
    for child in &level.children {
        out.extend(child.external.as_deref());
        externals(&child.below, out);
    }
}

/// Reads every container that the containers `roots` name, directly or through other containers
/// within `chain` steps, once each, the containers of one step in parallel.
pub(crate) fn read_all(
    archive: &Archive,
    index: &Index<'_>,
    roots: &[usize],
    cancel: Option<&CancelToken>,
    chain: usize,
    depth_limit: usize,
) -> HashMap<usize, Body> {
    let mut bodies: HashMap<usize, Body> = HashMap::new();
    let mut frontier: Vec<usize> = roots
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    for _ in 0..=chain {
        if frontier.is_empty() || cancel.is_some_and(CancelToken::is_cancelled) {
            break;
        }
        let read: Vec<(usize, Body)> = frontier
            .par_iter()
            .map(|&entry| (entry, parse(archive, index, entry, depth_limit)))
            .collect();
        let mut next: HashSet<usize> = HashSet::new();
        let mut paths = Vec::new();
        for (_, body) in &read {
            if let Body::Parsed { root, .. } = body {
                paths.clear();
                externals(root, &mut paths);
                next.extend(paths.iter().filter_map(|path| index.find(path)));
            }
        }
        bodies.extend(read);
        frontier = next
            .into_iter()
            .filter(|entry| !bodies.contains_key(entry))
            .collect();
    }
    bodies
}
