// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_dataforge::{Guid, RecordTree, StructValue, Value, ValueError};

use crate::merge::port_key;
use crate::port::{PortDefinition, read_definition};
use crate::read::{as_struct, struct_field, struct_items, text, unsigned};
use crate::{Geometry, GeometryVariant, ItemFacts, MaterialVariant, ObjectContainerRef};

const GEOMETRY_RESOURCE: &str = "SGeometryResourceParams";
const DEFAULT_LOADOUT: &str = "SEntityComponentDefaultLoadoutParams";
const PORT_CONTAINER: &str = "SItemPortContainerComponentParams";
const XML_LOADOUT: &str = "SItemPortLoadoutXMLParams";
const OUTFIT_LOADOUT: &str = "SItemPortLoadoutOutfitParams";
const MAX_OUTFIT_LAYERS: usize = 16;
const VEHICLE_PARAMS: &str = "VehicleComponentParams";
const ATTACHABLE: &str = "SAttachableComponentParams";
const INVENTORY_CONTAINER: &str = "SCItemInventoryContainerComponentParams";
const DOCKED_VEHICLE_TYPE: &str = "NOITEM_Vehicle";

/// What a Loadout entry or a port default names to attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetSpec {
    /// Nothing.
    None,
    /// The Record of this GUID.
    Reference(Guid),
    /// The Entity of this class name.
    ClassName(String),
    /// A Reference that cannot be read.
    Invalid(ValueError),
}

/// A Loadout as an Entity Record or a Loadout entry states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoadoutSpec {
    /// Entries in the Record.
    Manual(Arc<[EntrySpec]>),
    /// An XML Loadout file, by the path the Record names.
    Xml(String),
    /// Loadouts laid over each other in order, an outfit (`SItemPortLoadoutOutfitParams`): its
    /// base Loadout, then the Loadouts of the outfits that always apply. An entry without an
    /// Entity adds its nested entries to those of the earlier entry of the same port.
    Layered(Arc<[Self]>),
}

/// What supplied the attachment of a port: an Entity Record, or an XML Loadout file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadoutOrigin {
    /// The Loadout entries or the port definitions of this Entity Record, by record path.
    Record(Arc<str>),
    /// The XML Loadout file with this path, the `loadoutPath` a Record names or an include.
    File(Arc<str>),
}

impl LoadoutOrigin {
    /// Returns the record path or the file path.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Record(path) | Self::File(path) => path,
        }
    }

    /// Returns whether the origin is an XML Loadout file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

/// One entry of a Loadout: a port, what it holds and the Loadout given to that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntrySpec {
    pub(crate) origin: LoadoutOrigin,
    pub(crate) port: String,
    pub(crate) target: TargetSpec,
    pub(crate) loadout: Option<LoadoutSpec>,
}

/// One item port an Entity Record defines.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PortSpec {
    pub(crate) name: String,
    pub(crate) definition: Arc<PortDefinition>,
    pub(crate) default_item: TargetSpec,
}

/// The vehicle implementation a Record names and the Modification it selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VehicleSpec {
    pub(crate) definition: String,
    pub(crate) modification: String,
    pub(crate) landing_system: Option<Guid>,
}

/// What the Loadout tree needs of one Entity Record.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntitySpec {
    pub(crate) guid: Guid,
    pub(crate) record_path: String,
    pub(crate) origin: LoadoutOrigin,
    pub(crate) name: String,
    pub(crate) ports: Vec<PortSpec>,
    pub(crate) port_flags: Vec<String>,
    pub(crate) port_index: HashMap<String, usize>,
    pub(crate) loadout: Option<LoadoutSpec>,
    pub(crate) geometry: Option<Geometry>,
    pub(crate) variants: Vec<GeometryVariant>,
    pub(crate) vehicle: Option<VehicleSpec>,
    pub(crate) docked_vehicle: bool,
    pub(crate) attach_type: String,
    pub(crate) attach_sub_type: String,
    pub(crate) attach_tags: Vec<String>,
    pub(crate) material_variants: Vec<MaterialVariant>,
    pub(crate) size: Option<u32>,
    pub(crate) container: Option<Guid>,
    pub(crate) object_containers: Arc<[ObjectContainerRef]>,
}

impl EntitySpec {
    /// Reads the ports and the default Loadout from the components of an Entity's tree.
    pub(crate) fn read(tree: &RecordTree<'_>) -> Self {
        let components = components_of(tree);
        let record: Arc<str> = Arc::from(tree.record_path());
        let loadout = components
            .iter()
            .find(|component| component.type_name() == DEFAULT_LOADOUT)
            .and_then(|component| struct_field(component, "loadout"))
            .and_then(|loadout| read_loadout(loadout, &record));
        let (ports, port_flags) = read_ports(&components);
        let mut port_index = HashMap::with_capacity(ports.len());
        for (at, port) in ports.iter().enumerate() {
            port_index.entry(port_key(&port.name)).or_insert(at);
        }
        let geometry_component = components
            .iter()
            .find(|component| component.type_name() == GEOMETRY_RESOURCE);
        let geometry = geometry_component.and_then(|component| Geometry::read(component));
        let variants = geometry_component
            .map(|component| Geometry::read_variants(component))
            .unwrap_or_default();
        let vehicle = components
            .iter()
            .find(|component| component.type_name() == VEHICLE_PARAMS)
            .map(|component| read_vehicle(component));
        let (attach_type, container) = item_facts(&components);
        let definition = attach_def(&components);
        Self {
            attach_sub_type: definition
                .map_or("", |definition| text(definition, "SubType"))
                .to_owned(),
            attach_tags: definition
                .map(|definition| tag_words(text(definition, "Tags")))
                .unwrap_or_default(),
            material_variants: geometry_component
                .map(|component| MaterialVariant::read(component))
                .unwrap_or_default(),
            size: definition.and_then(|definition| unsigned(definition, "Size")),
            docked_vehicle: attach_type.starts_with(DOCKED_VEHICLE_TYPE),
            attach_type: attach_type.to_owned(),
            container,
            object_containers: crate::object_containers::read(&components).into(),
            guid: tree.guid(),
            record_path: tree.record_path().to_owned(),
            origin: LoadoutOrigin::Record(record),
            name: tree.name().to_owned(),
            ports,
            port_flags,
            port_index,
            loadout,
            geometry,
            variants,
            vehicle,
        }
    }

    /// Returns the first port this Entity defines with the name `name`, ignoring ASCII case.
    pub(crate) fn port(&self, name: &str) -> Option<&PortSpec> {
        self.port_index
            .get(&port_key(name))
            .and_then(|at| self.ports.get(*at))
    }

    /// Returns what the Entity offers a port.
    pub(crate) fn item_facts(&self) -> ItemFacts {
        ItemFacts {
            class_name: crate::entities::class_name_of(&self.name)
                .unwrap_or(&self.name)
                .to_owned(),
            item_type: self.attach_type.clone(),
            sub_type: self.attach_sub_type.clone(),
            size: self.size,
        }
    }

    /// Returns every target the Entity's own Loadout and port defaults name, nested ones
    /// included.
    pub(crate) fn targets(&self) -> Vec<&TargetSpec> {
        let mut targets: Vec<&TargetSpec> =
            self.ports.iter().map(|port| &port.default_item).collect();
        if let Some(LoadoutSpec::Manual(entries)) = &self.loadout {
            collect_targets(entries, &mut targets);
        }
        targets
    }
}

/// Reads the item ports of the port containers among `components`, and the flags the containers
/// give all their ports.
fn read_ports(components: &[&StructValue<'_>]) -> (Vec<PortSpec>, Vec<String>) {
    let containers = || {
        components
            .iter()
            .filter(|component| component.type_name() == PORT_CONTAINER)
    };
    let ports = containers()
        .flat_map(|component| struct_items(component, "Ports"))
        .map(|port| PortSpec {
            name: text(port, "Name").to_owned(),
            definition: Arc::new(read_definition(port)),
            default_item: struct_field(port, "defaultItem")
                .and_then(|item| item.field("entityClass"))
                .map_or(TargetSpec::None, reference_target),
        })
        .collect();
    let flags = containers()
        .flat_map(|component| text(component, "PortFlags").split_whitespace())
        .map(str::to_owned)
        .collect();
    (ports, flags)
}

fn components_of<'v, 'f>(tree: &'v RecordTree<'f>) -> Vec<&'v StructValue<'f>> {
    match tree.field("Components") {
        Some(Value::Array(items)) => items.iter().filter_map(as_struct).collect(),
        _ => Vec::new(),
    }
}

/// Reads what an Entity Record offers a port, with its class name taken from `name`.
pub(crate) fn read_item_facts(tree: &RecordTree<'_>) -> ItemFacts {
    let components = components_of(tree);
    let definition = attach_def(&components);
    let name = tree.name();
    ItemFacts {
        class_name: crate::entities::class_name_of(name)
            .unwrap_or(name)
            .to_owned(),
        item_type: definition
            .map_or("", |definition| text(definition, "Type"))
            .to_owned(),
        sub_type: definition
            .map_or("", |definition| text(definition, "SubType"))
            .to_owned(),
        size: definition.and_then(|definition| unsigned(definition, "Size")),
    }
}

fn read_vehicle(component: &StructValue<'_>) -> VehicleSpec {
    VehicleSpec {
        definition: text(component, "vehicleDefinition").trim().to_owned(),
        modification: text(component, "modification").trim().to_owned(),
        landing_system: match component.field("landingSystem").map(reference_target) {
            Some(TargetSpec::Reference(guid)) => Some(guid),
            _ => None,
        },
    }
}

/// Returns the `AttachDef` of the attachable component, if the components have one.
fn attach_def<'v, 'f>(components: &[&'v StructValue<'f>]) -> Option<&'v StructValue<'f>> {
    components
        .iter()
        .find(|component| component.type_name() == ATTACHABLE)
        .and_then(|component| struct_field(component, "AttachDef"))
}

/// Returns the `AttachDef.Type` and the inventory container Record the components name.
fn item_facts<'f>(components: &[&StructValue<'f>]) -> (&'f str, Option<Guid>) {
    let attach_type = attach_def(components).map_or("", |definition| text(definition, "Type"));
    let container = components
        .iter()
        .find(|component| component.type_name() == INVENTORY_CONTAINER)
        .and_then(|component| component.field("containerParams"))
        .and_then(|value| match reference_target(value) {
            TargetSpec::Reference(guid) => Some(guid),
            _ => None,
        });
    (attach_type, container)
}

fn collect_targets<'s>(entries: &'s [EntrySpec], targets: &mut Vec<&'s TargetSpec>) {
    for entry in entries {
        targets.push(&entry.target);
        if let Some(LoadoutSpec::Manual(nested)) = &entry.loadout {
            collect_targets(nested, targets);
        }
    }
}

/// Returns the words of a tag list as the game writes it: white space separated, a word such as
/// `$$hairVar_Brown++` stands for the tag `hairVar_Brown`.
fn tag_words(tags: &str) -> Vec<String> {
    tags.split_whitespace()
        .map(|word| word.trim_matches(['$', '+']))
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Reads an outfit: the base Loadout, then the Loadouts of the outfits without name tags; when
/// every outfit has name tags, those of the first.
fn read_outfit(outfit: &StructValue<'_>, record: &Arc<str>) -> Option<LoadoutSpec> {
    let base =
        struct_field(outfit, "BaseLoadout").and_then(|loadout| read_loadout(loadout, record));
    let outfits: Vec<&StructValue<'_>> = struct_items(outfit, "Outfits").collect();
    let chosen: Vec<&StructValue<'_>> = {
        let untagged: Vec<_> = outfits
            .iter()
            .copied()
            .filter(|outfit| !has_items(outfit, "OutfitNameTags"))
            .collect();
        if untagged.is_empty() {
            outfits.first().copied().into_iter().collect()
        } else {
            untagged
        }
    };
    let layers: Vec<LoadoutSpec> = base
        .into_iter()
        .chain(
            chosen
                .into_iter()
                .flat_map(|outfit| struct_items(outfit, "Loadouts"))
                .filter_map(|loadout| read_loadout(loadout, record)),
        )
        .take(MAX_OUTFIT_LAYERS)
        .collect();
    (!layers.is_empty()).then(|| LoadoutSpec::Layered(layers.into()))
}

fn has_items(structure: &StructValue<'_>, name: &str) -> bool {
    matches!(structure.field(name), Some(Value::Array(items)) if !items.is_empty())
}

/// Reads a manual, XML or outfit Loadout; `None` for any other struct.
fn read_loadout(loadout: &StructValue<'_>, record: &Arc<str>) -> Option<LoadoutSpec> {
    if loadout.type_name() == XML_LOADOUT {
        let path = text(loadout, "loadoutPath");
        return (!path.is_empty()).then(|| LoadoutSpec::Xml(path.to_owned()));
    }
    if loadout.type_name() == OUTFIT_LOADOUT {
        return read_outfit(loadout, record);
    }
    loadout.field("entries")?;
    Some(LoadoutSpec::Manual(
        struct_items(loadout, "entries")
            .map(|entry| read_entry(entry, record))
            .collect(),
    ))
}

fn read_entry(entry: &StructValue<'_>, record: &Arc<str>) -> EntrySpec {
    let reference = entry
        .field("entityClassReference")
        .map_or(TargetSpec::None, reference_target);
    let class_name = text(entry, "entityClassName");
    let target = match reference {
        TargetSpec::None if !class_name.is_empty() => TargetSpec::ClassName(class_name.to_owned()),
        other => other,
    };
    EntrySpec {
        origin: LoadoutOrigin::Record(Arc::clone(record)),
        port: text(entry, "itemPortName").to_owned(),
        target,
        loadout: struct_field(entry, "loadout").and_then(|loadout| read_loadout(loadout, record)),
    }
}

fn reference_target(value: &Value<'_>) -> TargetSpec {
    match value {
        Value::Reference(link) => TargetSpec::Reference(link.guid()),
        Value::Error(ValueError::DanglingReference { guid }) => TargetSpec::Reference(*guid),
        Value::Error(error) => TargetSpec::Invalid(*error),
        _ => TargetSpec::None,
    }
}
