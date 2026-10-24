// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeSet, HashMap};

use p4kx_assembly::{
    Attachment, EntityNode, LoadoutError, PortDefinition, PortNode, Target, VehicleImplementation,
    VehicleItemPort,
};
use p4kx_extraction::is_placeholder_key;

use crate::contract_loadout::{LoadoutPort, PortType};
use crate::guid_text::CatalogGuid;

const VEHICLE_SOURCE: &str = "vehicle";

/// Turns the tree of an Entity into catalog ports and collects the codes of the issues found.
pub(crate) struct PortReader {
    issues: BTreeSet<&'static str>,
}

impl PortReader {
    /// Returns a reader that has found no issue yet.
    pub(crate) const fn new() -> Self {
        Self {
            issues: BTreeSet::new(),
        }
    }

    /// Returns the codes of the issues found so far, sorted, and forgets them.
    pub(crate) fn take_issues(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.issues).into_iter().collect()
    }

    /// Returns the ports of `node` as catalog ports: the ports of its tree, then the item ports
    /// of its vehicle implementation that no port of the tree has.
    pub(crate) fn ports(&mut self, node: &EntityNode) -> Vec<LoadoutPort> {
        self.issues
            .extend(node.issues().iter().map(LoadoutError::kind));
        let vehicle: HashMap<String, &VehicleItemPort> = node
            .vehicle()
            .into_iter()
            .flat_map(VehicleImplementation::item_ports)
            .filter_map(|part| Some((part.name().to_ascii_lowercase(), part.item_port()?)))
            .collect();
        let mut ports: Vec<LoadoutPort> = node
            .ports()
            .iter()
            .map(|port| {
                let xml = vehicle.get(&port.name().to_ascii_lowercase()).copied();
                self.read_port(port, xml)
            })
            .collect();
        if let Some(implementation) = node.vehicle() {
            let named: BTreeSet<String> = node
                .ports()
                .iter()
                .map(|port| port.name().to_ascii_lowercase())
                .collect();
            for part in implementation.item_ports() {
                if let Some(port) = part.item_port()
                    && !named.contains(&part.name().to_ascii_lowercase())
                {
                    ports.push(vehicle_port(part.name(), port));
                }
            }
        }
        ports
    }

    fn read_port(&mut self, port: &PortNode, xml: Option<&VehicleItemPort>) -> LoadoutPort {
        let shape = match (port.definition(), xml) {
            (Some(definition), _) => read_definition(definition),
            (None, Some(xml)) => read_vehicle(xml),
            (None, None) => LoadoutPort::default(),
        };
        let mut shape = LoadoutPort {
            name: port.name().to_owned(),
            source: port.source().code(),
            visibility: port.visibility().code(),
            ..shape
        };
        match port.attachment() {
            Attachment::Empty => {
                shape.ports = port
                    .nested()
                    .iter()
                    .map(|nested| self.read_port(nested, None))
                    .collect();
            }
            Attachment::Entity(child) => {
                shape.item_guid = Some(CatalogGuid(child.guid()));
                shape.item_class_name = Some(child.class_name().to_owned());
                shape.docked_vehicle = child.is_docked_vehicle();
                if child.is_expanded() {
                    shape.ports = self.ports(child);
                }
            }
            Attachment::Unresolved(unresolved) => {
                self.issues.insert(unresolved.error().kind());
                shape.unresolved = Some(unresolved.error().kind());
                match unresolved.target() {
                    Target::Reference(guid) => shape.item_guid = Some(CatalogGuid(*guid)),
                    Target::ClassName(name) => shape.item_class_name = Some(name.clone()),
                    Target::Unreadable => {}
                }
            }
        }
        shape
    }
}

fn read_definition(definition: &PortDefinition) -> LoadoutPort {
    LoadoutPort {
        min_size: definition.min_size(),
        max_size: definition.max_size(),
        types: definition
            .types()
            .iter()
            .map(|kind| PortType {
                r#type: kind.name().to_owned(),
                sub_types: kind.sub_types().to_vec(),
            })
            .collect(),
        flags: words(definition.flags()),
        display_name_key: Some(definition.display_name())
            .filter(|key| !key.is_empty() && !is_placeholder_key(key))
            .map(str::to_owned),
        port_tags: tag_words(definition.port_tags()),
        required_tags: tag_words(definition.required_tags()),
        ..LoadoutPort::default()
    }
}

fn read_vehicle(port: &VehicleItemPort) -> LoadoutPort {
    LoadoutPort {
        min_size: port.min_size(),
        max_size: port.max_size(),
        types: port
            .types()
            .iter()
            .map(|kind| PortType {
                r#type: kind.clone(),
                sub_types: Vec::new(),
            })
            .collect(),
        flags: words(port.flags()),
        port_tags: tag_words(port.port_tags()),
        required_tags: tag_words(port.required_tags()),
        ..LoadoutPort::default()
    }
}

fn vehicle_port(name: &str, port: &VehicleItemPort) -> LoadoutPort {
    LoadoutPort {
        name: name.to_owned(),
        source: VEHICLE_SOURCE,
        visibility: port.visibility().code(),
        ..read_vehicle(port)
    }
}

fn tag_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| word.trim_start_matches('$'))
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
