use std::collections::{BTreeSet, HashMap};

use p4kx_assembly::{
    Attachment, EntityNode, LoadoutError, PortDefinition, PortNode, Target, VehicleImplementation,
    VehicleItemPort,
};
use p4kx_extraction::is_placeholder_key;

use crate::contract_loadout::{LoadoutPort, PortType};
use crate::guid_text::CatalogGuid;
use crate::tag_names::TagNames;

const VEHICLE_SOURCE: &str = "vehicle";

/// Turns the tree of an Entity into catalog ports and collects the codes of the issues found.
pub(crate) struct PortReader<'t> {
    tags: &'t TagNames,
    buffer: String,
    issues: BTreeSet<&'static str>,
}

impl<'t> PortReader<'t> {
    /// Returns a reader whose tag names resolve through `tags`.
    pub(crate) const fn new(tags: &'t TagNames) -> Self {
        Self {
            tags,
            buffer: String::new(),
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
                    ports.push(self.vehicle_port(part.name(), port));
                }
            }
        }
        ports
    }

    fn read_port(&mut self, port: &PortNode, xml: Option<&VehicleItemPort>) -> LoadoutPort {
        let shape = match (port.definition(), xml) {
            (Some(definition), _) => self.read_definition(definition),
            (None, Some(xml)) => self.read_vehicle(xml),
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

    fn tagged(&mut self, port_tags: &str, required_tags: &str) -> LoadoutPort {
        let port_tags = tag_words(port_tags);
        let required_tags = tag_words(required_tags);
        LoadoutPort {
            port_tag_guids: self.tags.resolve_all(&port_tags, &mut self.buffer),
            required_tag_guids: self.tags.resolve_all(&required_tags, &mut self.buffer),
            port_tags,
            required_tags,
            ..LoadoutPort::default()
        }
    }

    fn read_definition(&mut self, definition: &PortDefinition) -> LoadoutPort {
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
            ..self.tagged(definition.port_tags(), definition.required_tags())
        }
    }

    fn read_vehicle(&mut self, port: &VehicleItemPort) -> LoadoutPort {
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
            ..self.tagged(port.port_tags(), port.required_tags())
        }
    }

    fn vehicle_port(&mut self, name: &str, port: &VehicleItemPort) -> LoadoutPort {
        LoadoutPort {
            name: name.to_owned(),
            source: VEHICLE_SOURCE,
            visibility: port.visibility().code(),
            ..self.read_vehicle(port)
        }
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
