use std::collections::{BTreeSet, HashMap};

use p4kx_assembly::{
    Attachment, EntityNode, LoadoutError, PortDefinition, PortNode, Target, VehicleImplementation,
    VehicleItemPort,
};
use p4kx_extraction::is_placeholder_key;

use crate::contract_loadout::{LoadoutPort, PortType};
use crate::guid_text::CatalogGuid;

const VEHICLE_SOURCE: &str = "vehicle";

/// Returns the ports of `node` as catalog ports: the ports of its tree, then the item ports of its
/// vehicle implementation that no port of the tree has, with the codes of the issues found in
/// `node` and below it added to `issues`.
pub(crate) fn ports(node: &EntityNode, issues: &mut BTreeSet<&'static str>) -> Vec<LoadoutPort> {
    issues.extend(node.issues().iter().map(LoadoutError::kind));
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
            from_port(port, xml, issues)
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

fn from_port(
    port: &PortNode,
    xml: Option<&VehicleItemPort>,
    issues: &mut BTreeSet<&'static str>,
) -> LoadoutPort {
    let shape = port
        .definition()
        .map(from_definition)
        .or_else(|| xml.map(from_vehicle))
        .unwrap_or_default();
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
                .map(|nested| from_port(nested, None, issues))
                .collect();
        }
        Attachment::Entity(child) => {
            shape.item_guid = Some(CatalogGuid(child.guid()));
            shape.item_class_name = Some(child.class_name().to_owned());
            shape.docked_vehicle = child.is_docked_vehicle();
            if child.is_expanded() {
                shape.ports = ports(child, issues);
            }
        }
        Attachment::Unresolved(unresolved) => {
            issues.insert(unresolved.error().kind());
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

fn from_definition(definition: &PortDefinition) -> LoadoutPort {
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
        port_tags: words(definition.port_tags()),
        required_tags: words(definition.required_tags()),
        display_name_key: Some(definition.display_name())
            .filter(|key| !key.is_empty() && !is_placeholder_key(key))
            .map(str::to_owned),
        ..LoadoutPort::default()
    }
}

fn from_vehicle(port: &VehicleItemPort) -> LoadoutPort {
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
        port_tags: words(port.port_tags()),
        required_tags: words(port.required_tags()),
        ..LoadoutPort::default()
    }
}

fn vehicle_port(name: &str, port: &VehicleItemPort) -> LoadoutPort {
    LoadoutPort {
        name: name.to_owned(),
        source: VEHICLE_SOURCE,
        visibility: port.visibility().code(),
        ..from_vehicle(port)
    }
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| word.trim_start_matches('$'))
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
