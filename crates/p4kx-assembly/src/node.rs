// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use crate::entities::describe;
use crate::merge::{merge_by_port, port_key};
use crate::plan::Action;
use crate::port::PortDefinition;
use crate::resolver::LoadoutResolver;
use crate::spec::{EntitySpec, EntrySpec, LoadoutOrigin, LoadoutSpec, TargetSpec};
use crate::walk::Walk;
use crate::{
    Attachment, EntityNode, LoadoutError, MAX_LOADOUT_DEPTH, PortConstraints, PortNode, PortSource,
    PortVisibility, Target, Unresolved, VehicleImplementation,
};

/// One port of an Entity before it is resolved.
struct Pending<'s> {
    name: &'s str,
    source: PortSource,
    definition: Option<Arc<PortDefinition>>,
    target: &'s TargetSpec,
    loadout: Option<&'s LoadoutSpec>,
    origin: Option<&'s LoadoutOrigin>,
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
        let mut ports = Vec::new();
        let attached_vehicle = spec.docked_vehicle && !walk.is_root();
        let expanded = !(self.stop_at_docked && attached_vehicle);
        let vehicle = if expanded {
            self.vehicle_with_issues(spec, &mut issues)
        } else {
            None
        };
        let landing_gear = if expanded {
            self.landing_gear_of(spec, &mut issues)
        } else {
            Arc::from([])
        };
        let entries = if expanded {
            self.merged_entries(spec, overrides, &mut issues)
        } else {
            Vec::new()
        };
        let pending: Vec<Pending<'_>> = if expanded {
            pending_ports(spec, &entries, vehicle.as_deref()).collect()
        } else {
            Vec::new()
        };
        for pending in pending {
            if !walk.reserve() {
                issues.push(LoadoutError::TooManyPorts);
                break;
            }
            walk.enter(pending.name);
            let mut port = self.port_node(pending, walk, &mut issues, vehicle.as_deref());
            walk.leave();
            if let Some(vehicle) = &vehicle {
                port.visibility = vehicle.port_visibility(&port.name);
            }
            ports.push(port);
        }
        EntityNode {
            guid: spec.guid,
            record_path: spec.record_path.clone(),
            name: spec.name.clone(),
            ports,
            issues,
            geometry: spec.geometry.clone(),
            vehicle,
            landing_gear,
            docked_vehicle: attached_vehicle,
            loadout_file: match &spec.loadout {
                Some(LoadoutSpec::Xml(path)) => Some(Arc::clone(path)),
                _ => None,
            },
            expanded,
            attach_type: spec.attach_type.clone(),
            attach_sub_type: spec.attach_sub_type.clone(),
            size: spec.size,
            container: spec.container,
            object_containers: Arc::clone(&spec.object_containers),
        }
    }

    /// Returns the vehicle implementation of `spec`, pushing its issues to `issues`.
    fn vehicle_with_issues(
        &mut self,
        spec: &EntitySpec,
        issues: &mut Vec<LoadoutError>,
    ) -> Option<Arc<VehicleImplementation>> {
        match self.vehicle_of(spec) {
            Some(Ok(vehicle)) => {
                issues.extend(vehicle.issues.iter().cloned());
                Some(vehicle)
            }
            Some(Err(error)) => {
                issues.push(error);
                None
            }
            None => None,
        }
    }

    fn port_node(
        &mut self,
        pending: Pending<'_>,
        walk: &mut Walk,
        issues: &mut Vec<LoadoutError>,
        vehicle: Option<&VehicleImplementation>,
    ) -> PortNode {
        let constraints = pending
            .definition
            .as_deref()
            .and_then(PortConstraints::of_definition)
            .or_else(|| {
                vehicle
                    .and_then(|vehicle| vehicle.item_port(pending.name))
                    .and_then(PortConstraints::of_vehicle)
            })
            .map(Arc::new);
        let mut node = PortNode {
            name: pending.name.to_owned(),
            source: pending.source,
            definition: pending.definition,
            constraints,
            attachment: Attachment::Empty,
            nested: Vec::new(),
            visibility: PortVisibility::default(),
            origin: pending.origin.cloned(),
        };
        if self.change(&mut node, walk) {
            return node;
        }
        node.attachment = self.attach(pending.target, pending.loadout, walk);
        if matches!(node.attachment, Attachment::Empty) {
            node.nested = self.nested_ports(pending.loadout, walk, issues);
        }
        node
    }

    /// Applies the change of the plan to the port entered last, if it has one, and returns
    /// whether it holds what the plan says now.
    fn change(&mut self, node: &mut PortNode, walk: &mut Walk) -> bool {
        let Some(plan) = walk.plan.as_mut() else {
            return false;
        };
        match plan.action() {
            Action::Keep => false,
            Action::Exclude => {
                node.source = PortSource::Excluded;
                node.origin = None;
                true
            }
            Action::Replace(guid, facts) => {
                if let Some(constraints) = node.constraints()
                    && let Err(reason) = constraints.check(&facts)
                {
                    plan.reject(&facts, reason);
                    return false;
                }
                node.attachment = self.attach(&TargetSpec::Reference(guid), None, walk);
                node.source = PortSource::Replaced;
                node.origin = None;
                true
            }
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
            walk.enter(&entry.port);
            let mut node = PortNode {
                name: entry.port.clone(),
                source: PortSource::Override,
                definition: None,
                constraints: None,
                attachment: Attachment::Empty,
                nested: Vec::new(),
                visibility: PortVisibility::default(),
                origin: Some(entry.origin.clone()),
            };
            if !self.change(&mut node, walk) {
                node.attachment = self.attach(&entry.target, entry.loadout.as_ref(), walk);
                if matches!(node.attachment, Attachment::Empty) {
                    node.nested = self.nested_ports(entry.loadout.as_ref(), walk, issues);
                }
            }
            walk.leave();
            ports.push(node);
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
        if let Some(error) = walk.limit(guid) {
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
/// no entry names, then the item ports only the vehicle implementation declares, each part once
/// even where several share a name.
fn pending_ports<'s>(
    spec: &'s EntitySpec,
    entries: &'s [(EntrySpec, PortSource)],
    vehicle: Option<&'s VehicleImplementation>,
) -> impl Iterator<Item = Pending<'s>> {
    let named: HashSet<String> = entries
        .iter()
        .map(|(entry, _)| port_key(&entry.port))
        .collect();
    let mut taken = named.clone();
    taken.extend(spec.ports.iter().map(|port| port_key(&port.name)));
    let declared = vehicle
        .into_iter()
        .flat_map(VehicleImplementation::item_ports)
        .filter(move |part| !taken.contains(&port_key(part.name())))
        .map(|part| Pending {
            name: part.name(),
            source: PortSource::Empty,
            definition: None,
            target: &TargetSpec::None,
            loadout: None,
            origin: None,
        });
    let from_entries = entries.iter().map(|(entry, source)| Pending {
        name: &entry.port,
        source: *source,
        definition: spec
            .port(&entry.port)
            .map(|port| Arc::clone(&port.definition)),
        target: &entry.target,
        loadout: entry.loadout.as_ref(),
        origin: Some(&entry.origin),
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
            origin: (port.default_item != TargetSpec::None).then_some(&spec.origin),
        });
    from_entries.chain(defined).chain(declared)
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
