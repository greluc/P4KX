use std::collections::HashSet;
use std::sync::Arc;

use p4kx_dataforge::Guid;

use crate::entities::describe;
use crate::merge::{merge_by_port, port_key};
use crate::port::PortDefinition;
use crate::resolver::LoadoutResolver;
use crate::spec::{EntitySpec, EntrySpec, LoadoutSpec, TargetSpec};
use crate::{
    Attachment, EntityNode, LoadoutError, MAX_LOADOUT_DEPTH, MAX_LOADOUT_PORTS, PortNode,
    PortSource, Target, Unresolved,
};

/// The state of one resolution: the Entities on the current branch and the ports so far.
pub(crate) struct Walk {
    ancestors: Vec<Guid>,
    nesting: usize,
    ports: usize,
}

impl Walk {
    pub(crate) fn new(root: Guid) -> Self {
        Self {
            ancestors: vec![root],
            nesting: 0,
            ports: 0,
        }
    }

    /// Returns the depth of the current branch: attached Entities plus nested Loadout levels.
    const fn depth(&self) -> usize {
        self.ancestors.len().saturating_add(self.nesting)
    }

    /// Counts one more port, or returns `false` when the tree already holds
    /// [`MAX_LOADOUT_PORTS`].
    const fn reserve(&mut self) -> bool {
        if self.ports >= MAX_LOADOUT_PORTS {
            return false;
        }
        self.ports = self.ports.saturating_add(1);
        true
    }
}

/// One port of an Entity before it is resolved.
struct Pending<'s> {
    name: &'s str,
    source: PortSource,
    definition: Option<Arc<PortDefinition>>,
    target: &'s TargetSpec,
    loadout: Option<&'s LoadoutSpec>,
}

impl LoadoutResolver<'_> {
    /// Builds the node of an Entity whose own Loadout is overridden per port by `overrides`.
    pub(crate) fn node(
        &mut self,
        spec: &EntitySpec,
        overrides: Option<&LoadoutSpec>,
        walk: &mut Walk,
    ) -> EntityNode {
        let mut issues = Vec::new();
        let entries = self.merged_entries(spec, overrides, &mut issues);
        let mut ports = Vec::new();
        for pending in pending_ports(spec, &entries) {
            if !walk.reserve() {
                issues.push(LoadoutError::TooManyPorts);
                break;
            }
            ports.push(self.port_node(pending, walk, &mut issues));
        }
        EntityNode {
            guid: spec.guid,
            record_path: spec.record_path.clone(),
            name: spec.name.clone(),
            ports,
            issues,
        }
    }

    fn port_node(
        &mut self,
        pending: Pending<'_>,
        walk: &mut Walk,
        issues: &mut Vec<LoadoutError>,
    ) -> PortNode {
        let attachment = self.attach(pending.target, pending.loadout, walk);
        let nested = if matches!(attachment, Attachment::Empty) {
            self.nested_ports(pending.loadout, walk, issues)
        } else {
            Vec::new()
        };
        PortNode {
            name: pending.name.to_owned(),
            source: pending.source,
            definition: pending.definition,
            attachment,
            nested,
        }
    }

    /// Resolves the entries of the Loadout of an empty port into ports of their own; whatever
    /// cannot be resolved or kept is reported in `issues`.
    fn nested_ports(
        &mut self,
        loadout: Option<&LoadoutSpec>,
        walk: &mut Walk,
        issues: &mut Vec<LoadoutError>,
    ) -> Vec<PortNode> {
        let Some(loadout) = loadout else {
            return Vec::new();
        };
        if walk.depth() >= MAX_LOADOUT_DEPTH {
            issues.push(LoadoutError::TooDeep);
            return Vec::new();
        }
        let entries = match self.entries(Some(loadout)) {
            Ok(entries) => entries,
            Err(error) => {
                issues.push(error);
                return Vec::new();
            }
        };
        issues.extend(entries.issues.iter().cloned());
        let mut ports = Vec::new();
        walk.nesting = walk.nesting.saturating_add(1);
        for entry in entries.list.iter() {
            if !walk.reserve() {
                issues.push(LoadoutError::TooManyPorts);
                break;
            }
            let attachment = self.attach(&entry.target, entry.loadout.as_ref(), walk);
            let nested = if matches!(attachment, Attachment::Empty) {
                self.nested_ports(entry.loadout.as_ref(), walk, issues)
            } else {
                Vec::new()
            };
            ports.push(PortNode {
                name: entry.port.clone(),
                source: PortSource::Override,
                definition: None,
                attachment,
                nested,
            });
        }
        walk.nesting = walk.nesting.saturating_sub(1);
        ports
    }

    /// Returns the entries of the Entity's own Loadout, each replaced in place by the entry of
    /// the same port in `overrides`, with the other overriding entries appended.
    fn merged_entries(
        &mut self,
        spec: &EntitySpec,
        overrides: Option<&LoadoutSpec>,
        issues: &mut Vec<LoadoutError>,
    ) -> Vec<(EntrySpec, PortSource)> {
        let mut entries = Vec::new();
        match self.entries(spec.loadout.as_ref()) {
            Ok(own) => {
                issues.extend(own.issues.iter().cloned());
                entries.extend(
                    own.list
                        .iter()
                        .map(|entry| (entry.clone(), PortSource::Loadout)),
                );
            }
            Err(error) => issues.push(error),
        }
        if overrides.is_some() {
            match self.entries(overrides) {
                Ok(given) => {
                    issues.extend(given.issues.iter().cloned());
                    let given = given.list.iter().map(|e| (e.clone(), PortSource::Override));
                    merge_by_port(&mut entries, given, |(entry, _)| &entry.port);
                }
                Err(error) => issues.push(error),
            }
        }
        entries
    }

    /// Resolves what one port holds.
    fn attach(
        &mut self,
        target: &TargetSpec,
        loadout: Option<&LoadoutSpec>,
        walk: &mut Walk,
    ) -> Attachment {
        let named = match target {
            TargetSpec::None => return Attachment::Empty,
            TargetSpec::Invalid(reason) => {
                return unresolved(
                    Target::Unreadable,
                    LoadoutError::InvalidReference { reason: *reason },
                );
            }
            TargetSpec::Reference(guid) => Target::Reference(*guid),
            TargetSpec::ClassName(name) => Target::ClassName(name.clone()),
        };
        let Some(record) = self.record_of(target) else {
            let error = match target {
                TargetSpec::Reference(guid) => LoadoutError::DanglingReference { guid: *guid },
                _ => LoadoutError::UnknownClass {
                    name: class_text(target),
                },
            };
            return unresolved(named, error);
        };
        let guid = record.guid();
        if let Some(error) = limit(walk, guid) {
            return unresolved(named, error);
        }
        let spec = self.spec(guid);
        let spec = match spec.as_ref() {
            Ok(spec) => spec,
            Err(error) => return unresolved(named, error.clone()),
        };
        walk.ancestors.push(guid);
        let mut child = self.node(spec, loadout, walk);
        walk.ancestors.pop();
        if let Some(ambiguity) = self.ambiguity(target) {
            child.issues.insert(0, ambiguity);
        }
        Attachment::Entity(Box::new(child))
    }

    /// Returns the ambiguity of a class name that several Entities have, if it is one.
    fn ambiguity(&self, target: &TargetSpec) -> Option<LoadoutError> {
        let TargetSpec::ClassName(name) = target else {
            return None;
        };
        let candidates = self.entities.class_candidates(name);
        (candidates.len() > 1).then(|| LoadoutError::AmbiguousClass {
            name: name.clone(),
            candidates: candidates.iter().map(describe).collect(),
        })
    }
}

/// Returns the ports of an Entity in order: the merged Loadout entries, then the defined ports
/// no entry names.
fn pending_ports<'s>(
    spec: &'s EntitySpec,
    entries: &'s [(EntrySpec, PortSource)],
) -> impl Iterator<Item = Pending<'s>> {
    let named: HashSet<String> = entries
        .iter()
        .map(|(entry, _)| port_key(&entry.port))
        .collect();
    let from_entries = entries.iter().map(|(entry, source)| Pending {
        name: &entry.port,
        source: *source,
        definition: spec
            .port(&entry.port)
            .map(|port| Arc::clone(&port.definition)),
        target: &entry.target,
        loadout: entry.loadout.as_ref(),
    });
    let defined = spec
        .ports
        .iter()
        .filter(move |port| !named.contains(&port_key(&port.name)))
        .map(|port| Pending {
            name: &port.name,
            source: if port.default_item == TargetSpec::None {
                PortSource::Unset
            } else {
                PortSource::PortDefault
            },
            definition: Some(Arc::clone(&port.definition)),
            target: &port.default_item,
            loadout: None,
        });
    from_entries.chain(defined)
}

/// Returns why the Entity `guid` may not be attached below the current branch, if it may not.
fn limit(walk: &Walk, guid: Guid) -> Option<LoadoutError> {
    if walk.ancestors.contains(&guid) {
        Some(LoadoutError::Cycle { guid })
    } else if walk.depth() > MAX_LOADOUT_DEPTH {
        Some(LoadoutError::TooDeep)
    } else {
        None
    }
}

fn class_text(target: &TargetSpec) -> String {
    match target {
        TargetSpec::ClassName(name) => name.clone(),
        _ => String::new(),
    }
}

const fn unresolved(target: Target, error: LoadoutError) -> Attachment {
    Attachment::Unresolved(Unresolved { target, error })
}
