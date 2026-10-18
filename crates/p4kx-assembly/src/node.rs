use p4kx_dataforge::Guid;

use crate::resolver::LoadoutResolver;
use crate::spec::{EntitySpec, EntrySpec, LoadoutSpec, TargetSpec};
use crate::{
    Attachment, EntityNode, LoadoutError, MAX_LOADOUT_DEPTH, MAX_LOADOUT_PORTS, PortNode,
    PortSource, Target, Unresolved,
};

/// The state of one resolution: the Entities on the current branch and the ports so far.
pub(crate) struct Walk {
    ancestors: Vec<Guid>,
    ports: usize,
}

impl Walk {
    pub(crate) fn new(root: Guid) -> Self {
        Self {
            ancestors: vec![root],
            ports: 0,
        }
    }
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
        let mut entries: Vec<(EntrySpec, PortSource)> = Vec::new();
        match self.entries(spec.loadout.as_ref()) {
            Ok(own) => entries.extend(own.iter().map(|entry| (entry.clone(), PortSource::Loadout))),
            Err(error) => issues.push(error),
        }
        if overrides.is_some() {
            match self.entries(overrides) {
                Ok(given) => override_entries(&mut entries, &given),
                Err(error) => issues.push(error),
            }
        }
        let mut ports = Vec::with_capacity(entries.len());
        for (entry, source) in &entries {
            walk.ports = walk.ports.saturating_add(1);
            let definition = spec
                .ports
                .iter()
                .find(|port| port.name.eq_ignore_ascii_case(&entry.port))
                .map(|port| port.definition.clone());
            ports.push(PortNode {
                name: entry.port.clone(),
                source: *source,
                definition,
                attachment: self.attach(&entry.target, entry.loadout.as_ref(), walk),
            });
        }
        for port in &spec.ports {
            if entries
                .iter()
                .any(|(entry, _)| entry.port.eq_ignore_ascii_case(&port.name))
            {
                continue;
            }
            walk.ports = walk.ports.saturating_add(1);
            let source = if port.default_item == TargetSpec::None {
                PortSource::Unset
            } else {
                PortSource::PortDefault
            };
            ports.push(PortNode {
                name: port.name.clone(),
                source,
                definition: Some(port.definition.clone()),
                attachment: self.attach(&port.default_item, None, walk),
            });
        }
        EntityNode {
            guid: spec.guid,
            record_path: spec.record_path.clone(),
            name: spec.name.clone(),
            ports,
            issues,
        }
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
        let child = self.node(spec, loadout, walk);
        walk.ancestors.pop();
        Attachment::Entity(Box::new(child))
    }
}

/// Returns why the Entity `guid` may not be attached below the current branch, if it may not.
fn limit(walk: &Walk, guid: Guid) -> Option<LoadoutError> {
    if walk.ancestors.contains(&guid) {
        Some(LoadoutError::Cycle { guid })
    } else if walk.ancestors.len() > MAX_LOADOUT_DEPTH {
        Some(LoadoutError::TooDeep)
    } else if walk.ports >= MAX_LOADOUT_PORTS {
        Some(LoadoutError::TooManyPorts)
    } else {
        None
    }
}

/// Replaces the entry of each port `given` names, ignoring ASCII case, and appends the rest.
fn override_entries(entries: &mut Vec<(EntrySpec, PortSource)>, given: &[EntrySpec]) {
    for entry in given {
        let replacement = (entry.clone(), PortSource::Override);
        match entries
            .iter_mut()
            .find(|(old, _)| old.port.eq_ignore_ascii_case(&entry.port))
        {
            Some(old) => *old = replacement,
            None => entries.push(replacement),
        }
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
