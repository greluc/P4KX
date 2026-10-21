use std::collections::{HashMap, HashSet};

use crate::annotations::TAG_TYPE;
use crate::tags::{
    MAX_TAG_DEPTH, MAX_TAG_NAME_BYTES, MAX_TAG_PATH_BYTES, MAX_TAG_TEXT_BYTES, Node,
    TAG_DATABASE_TYPE, TAG_PATH_SEPARATOR, TagTreeIssues,
};
use crate::{DataForge, Guid, TreeBuilder, Value};

const NO_LEGACY_ID: u32 = u32::MAX;

/// The tags of a DataForge laid out in pre-order, before they are indexed.
pub(crate) struct Layout {
    pub(crate) nodes: Vec<Node>,
    pub(crate) roots: Vec<u32>,
    pub(crate) issues: TagTreeIssues,
}

/// A Tag Record read from the DataForge before the tree is laid out.
struct Raw<'f> {
    guid: Guid,
    name: &'f str,
    legacy_id: Option<u32>,
    children: Vec<Guid>,
}

/// Lays out the tags of `forge`: the root tags the `TagDatabase` Records list, everything below
/// them, then the Tag Records nothing lists.
pub(crate) fn layout(forge: &DataForge<'_>) -> Layout {
    let mut issues = TagTreeIssues::default();
    let (raws, database_roots) = read(forge, &mut issues);
    let positions: HashMap<Guid, usize> = raws
        .iter()
        .enumerate()
        .map(|(position, raw)| (raw.guid, position))
        .collect();
    let mut walk = Walk {
        raws: &raws,
        positions: &positions,
        visited: vec![false; raws.len()],
        nodes: Vec::with_capacity(raws.len()),
        roots: Vec::new(),
        text_bytes: 0,
        issues,
    };
    for guid in &database_roots {
        match positions.get(guid) {
            Some(&position) => walk.root(position, true),
            None => walk.issues.dangling += 1,
        }
    }
    walk.unlisted();
    let Walk {
        nodes,
        roots,
        mut issues,
        ..
    } = walk;
    issues.unlisted = nodes.iter().filter(|node| !node.listed).count();
    Layout {
        nodes,
        roots,
        issues,
    }
}

/// Reads the Tag Records and the root tags every `TagDatabase` lists. Of Tag Records that share a
/// GUID the first counts.
fn read<'f>(forge: &'f DataForge<'f>, issues: &mut TagTreeIssues) -> (Vec<Raw<'f>>, Vec<Guid>) {
    let mut builder = TreeBuilder::new();
    let mut raws = Vec::new();
    let mut seen = HashSet::new();
    let mut roots = Vec::new();
    for record in forge.records() {
        let Ok(type_name) = record.type_name() else {
            continue;
        };
        if type_name != TAG_TYPE && type_name != TAG_DATABASE_TYPE {
            continue;
        }
        let Ok(tree) = builder.build(&record) else {
            issues.unreadable += 1;
            continue;
        };
        if type_name == TAG_DATABASE_TYPE {
            references(tree.field("tags"), &mut roots, issues);
            continue;
        }
        if !seen.insert(record.guid()) {
            issues.duplicate += 1;
            continue;
        }
        let name = match tree.field("tagName") {
            Some(&Value::String(name)) => name,
            _ => "",
        };
        let legacy_id = match tree.field("legacyGUID") {
            Some(Value::UInt32(id)) if *id != NO_LEGACY_ID => Some(*id),
            _ => None,
        };
        let mut children = Vec::new();
        references(tree.field("children"), &mut children, issues);
        raws.push(Raw {
            guid: record.guid(),
            name,
            legacy_id,
            children,
        });
    }
    (raws, roots)
}

/// Appends the GUIDs of the References in the array `value`; anything else counts as dangling.
fn references(value: Option<&Value<'_>>, out: &mut Vec<Guid>, issues: &mut TagTreeIssues) {
    let Some(Value::Array(items)) = value else {
        return;
    };
    out.reserve(items.len());
    for item in items {
        match item {
            Value::Reference(reference) => out.push(reference.guid()),
            Value::Null => {}
            _ => issues.dangling += 1,
        }
    }
}

struct Walk<'a, 'f> {
    raws: &'a [Raw<'f>],
    positions: &'a HashMap<Guid, usize>,
    visited: Vec<bool>,
    nodes: Vec<Node>,
    roots: Vec<u32>,
    text_bytes: usize,
    issues: TagTreeIssues,
}

/// A tag on the way down: its node, its Tag Record and the next child to visit.
struct Frame {
    node: u32,
    raw: usize,
    next: usize,
}

impl Walk<'_, '_> {
    /// Visits the root tag `position` and everything below it, unless it was visited already.
    fn root(&mut self, position: usize, listed: bool) {
        if self.visited.get(position).copied().unwrap_or(true) {
            self.issues.repeated += 1;
            return;
        }
        let Some(node) = self.enter(position, None, listed) else {
            return;
        };
        self.roots.push(node);
        let mut stack = vec![Frame {
            node,
            raw: position,
            next: 0,
        }];
        while let Some(frame) = stack.last_mut() {
            let child = self
                .raws
                .get(frame.raw)
                .and_then(|raw| raw.children.get(frame.next))
                .copied();
            let Some(child) = child else {
                let finished = stack.pop();
                let end = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
                if let Some(Frame { node, .. }) = finished
                    && let Some(done) = self.nodes.get_mut(node as usize)
                {
                    done.end = end;
                }
                continue;
            };
            frame.next += 1;
            let parent = frame.node;
            let Some(&position) = self.positions.get(&child) else {
                self.issues.dangling += 1;
                continue;
            };
            if self.visited.get(position).copied().unwrap_or(true) {
                self.issues.repeated += 1;
                continue;
            }
            if stack.len() >= MAX_TAG_DEPTH {
                self.issues.too_deep += 1;
                self.leave_out(position);
                continue;
            }
            if let Some(entered) = self.enter(position, Some(parent), listed)
                && let Some(parent) = self.nodes.get_mut(parent as usize)
            {
                parent.children.push(entered);
                stack.push(Frame {
                    node: entered,
                    raw: position,
                    next: 0,
                });
            }
        }
    }

    /// Adds the tag `position` below `parent` and returns its index; a tag whose name or path is
    /// too long, or that would take the text of the tree over its budget, is left out with
    /// everything below it.
    fn enter(&mut self, position: usize, parent: Option<u32>, listed: bool) -> Option<u32> {
        let raw = self.raws.get(position)?;
        let parent_node = parent.and_then(|parent| self.nodes.get(parent as usize));
        let prefix = parent_node.map_or(0, |parent| {
            parent.path.len() + TAG_PATH_SEPARATOR.len_utf8()
        });
        let path_len = prefix + raw.name.len();
        let index = u32::try_from(self.nodes.len()).ok();
        let fits = raw.name.len() <= MAX_TAG_NAME_BYTES
            && path_len <= MAX_TAG_PATH_BYTES
            && self.text_bytes.saturating_add(path_len) <= MAX_TAG_TEXT_BYTES;
        let (Some(index), true) = (index, fits) else {
            self.issues.too_long += 1;
            self.leave_out(position);
            return None;
        };
        *self.visited.get_mut(position)? = true;
        let mut path = String::with_capacity(path_len);
        if let Some(parent) = parent_node {
            path.push_str(&parent.path);
            path.push(TAG_PATH_SEPARATOR);
        }
        path.push_str(raw.name);
        let depth = parent_node.map_or(0, |parent| parent.depth.saturating_add(1));
        self.text_bytes += path_len;
        self.nodes.push(Node {
            guid: raw.guid,
            path: path.into(),
            name_start: u32::try_from(prefix).unwrap_or(u32::MAX),
            legacy_id: raw.legacy_id,
            parent,
            children: Vec::new(),
            end: index.saturating_add(1),
            depth,
            listed,
        });
        Some(index)
    }

    /// Marks the tag `position` and every tag only reachable through it as visited, so that none
    /// of them becomes a root of its own.
    fn leave_out(&mut self, position: usize) {
        let mut stack = vec![position];
        while let Some(position) = stack.pop() {
            match self.visited.get_mut(position) {
                Some(flag) if !*flag => *flag = true,
                _ => continue,
            }
            let Some(raw) = self.raws.get(position) else {
                continue;
            };
            stack.extend(
                raw.children
                    .iter()
                    .filter_map(|child| self.positions.get(child).copied()),
            );
        }
    }

    /// Makes every tag the roots did not reach a root of its own: first those no tag lists as a
    /// child, then what is left, which only a cycle can leave.
    fn unlisted(&mut self) {
        let mut has_parent = vec![false; self.raws.len()];
        for raw in self.raws {
            for child in &raw.children {
                if let Some(&position) = self.positions.get(child)
                    && let Some(flag) = has_parent.get_mut(position)
                {
                    *flag = true;
                }
            }
        }
        for orphans_first in [true, false] {
            for position in 0..self.raws.len() {
                let unvisited = self.visited.get(position) == Some(&false);
                let parentless = has_parent.get(position) == Some(&false);
                if unvisited && (parentless || !orphans_first) {
                    self.root(position, false);
                }
            }
        }
    }
}
