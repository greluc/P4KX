use std::io::Read;

use p4kx_extraction::cryxml::{self, CryXmlError, Document, Element, Xml};
use p4kx_extraction::{Archive, PathIndex};
use thiserror::Error;

const MAX_VEHICLE_FILE_LEN: u64 = 16 << 20;
const IMPLEMENTATIONS: &str = "Data/Scripts/Entities/Vehicles/Implementations/Xml/";
const PORT: &str = "ItemPort";
const PAINTS: &str = "Paints";

/// An item port of paints, in a ship's Record or its vehicle implementation: the tags an item
/// needs to carry to fit it and the tags the port offers, as lower-case tag names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct PaintPort {
    id: Option<String>,
    pub(crate) port_tags: Vec<String>,
    pub(crate) required_tags: Vec<String>,
}

impl PaintPort {
    /// Returns the port with the space-separated tag lists `port_tags` and `required_tags`.
    pub(crate) fn new(port_tags: Option<&str>, required_tags: Option<&str>) -> Self {
        Self {
            id: None,
            port_tags: tags(port_tags),
            required_tags: tags(required_tags),
        }
    }
}

/// A vehicle implementation file cannot be used.
#[derive(Debug, Error)]
pub(crate) enum VehicleFileError {
    /// The file is text XML, which the catalog does not read.
    #[error("the file is text XML")]
    Text,
    /// The file is corrupt or no XML.
    #[error("the file cannot be parsed")]
    Corrupt(#[source] CryXmlError),
    /// The file is larger than the limit or cannot be read.
    #[error("the file cannot be read")]
    Unreadable,
}

/// The paint ports of a vehicle and whether a file it names could not be used.
#[derive(Debug, Clone, Default)]
pub(crate) struct VehiclePorts {
    pub(crate) ports: Vec<PaintPort>,
    pub(crate) unreadable: bool,
}

/// Reads the vehicle implementation files of an Archive.
pub(crate) struct VehicleFiles<'a> {
    archive: &'a Archive,
    index: PathIndex<'a>,
}

impl<'a> VehicleFiles<'a> {
    pub(crate) fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            index: PathIndex::new(archive),
        }
    }

    /// Returns the paint ports of the vehicle implementation at `definition` with the
    /// Modification `modification` applied: the paint ports of its patch file replace those of
    /// the base file, and its `Elem` entries override the tags of a port by `id`.
    pub(crate) fn paint_ports(&self, definition: &str, modification: Option<&str>) -> VehiclePorts {
        let bytes = match self.read(&entry_path(definition)) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return VehiclePorts::default(),
            Err(_ignored) => return unreadable(),
        };
        let Ok(document) = parse(&bytes) else {
            return unreadable();
        };
        let root = document.root();
        let mut result = VehiclePorts {
            ports: ports_of(root),
            unreadable: false,
        };
        let Some(name) = modification.filter(|name| !name.is_empty()) else {
            return result;
        };
        let Some(modification) = find_modification(root, name) else {
            return result;
        };
        if let Some(patch) = modification
            .attribute("patchFile")
            .filter(|p| !p.is_empty())
        {
            match self.patched(patch) {
                Ok(patched) if !patched.is_empty() => result.ports = patched,
                Ok(_) => {}
                Err(_) => result.unreadable = true,
            }
        }
        apply_elems(&mut result.ports, modification);
        result
    }

    fn patched(&self, patch: &str) -> Result<Vec<PaintPort>, VehicleFileError> {
        if patch.contains("..") || patch.contains(':') {
            return Err(VehicleFileError::Unreadable);
        }
        let name = format!("{IMPLEMENTATIONS}{}.xml", patch.trim_matches(['/', '\\']));
        let Some(bytes) = self.read(&name)? else {
            return Ok(Vec::new());
        };
        Ok(ports_of(parse(&bytes)?.root()))
    }

    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, VehicleFileError> {
        let Some(entry) = self.index.find(path) else {
            return Ok(None);
        };
        if entry.uncompressed_size() > MAX_VEHICLE_FILE_LEN {
            return Err(VehicleFileError::Unreadable);
        }
        let reader = self
            .archive
            .open_entry(entry)
            .map_err(|_| VehicleFileError::Unreadable)?;
        let mut bytes = Vec::new();
        reader
            .take(MAX_VEHICLE_FILE_LEN)
            .read_to_end(&mut bytes)
            .map_err(|_| VehicleFileError::Unreadable)?;
        Ok(Some(bytes))
    }
}

const fn unreadable() -> VehiclePorts {
    VehiclePorts {
        ports: Vec::new(),
        unreadable: true,
    }
}

fn entry_path(definition: &str) -> String {
    let starts_with_data = definition.get(..5).is_some_and(|start| {
        start.eq_ignore_ascii_case("data/") || start.eq_ignore_ascii_case("data\\")
    });
    if starts_with_data {
        definition.to_owned()
    } else {
        format!("Data/{}", definition.trim_start_matches(['/', '\\']))
    }
}

fn parse(bytes: &[u8]) -> Result<Document<'_>, VehicleFileError> {
    match cryxml::read(bytes).map_err(VehicleFileError::Corrupt)? {
        Xml::CryXml(document) => Ok(document),
        Xml::Text(_) => Err(VehicleFileError::Text),
    }
}

/// Returns every `ItemPort` below `root` that takes items of the type `Paints`.
fn ports_of(root: Element<'_>) -> Vec<PaintPort> {
    let mut ports = Vec::new();
    let mut pending = vec![root];
    while let Some(element) = pending.pop() {
        if element.name() == PORT && takes_paints(element) {
            ports.push(PaintPort {
                id: element.attribute("id").map(str::to_owned),
                port_tags: tags(element.attribute("portTags")),
                required_tags: tags(element.attribute("requiredTags")),
            });
        }
        pending.extend(element.children());
    }
    ports
}

fn takes_paints(port: Element<'_>) -> bool {
    port.children()
        .filter(|child| child.name() == "Types")
        .flat_map(Element::children)
        .any(|kind| {
            kind.name() == "Type"
                && kind
                    .attribute("type")
                    .is_some_and(|value| value.eq_ignore_ascii_case(PAINTS))
        })
}

fn tags(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}

fn find_modification<'d>(root: Element<'d>, name: &str) -> Option<Element<'d>> {
    root.children()
        .filter(|child| child.name() == "Modifications")
        .flat_map(Element::children)
        .find(|child| {
            child.name() == "Modification"
                && child
                    .attribute("name")
                    .is_some_and(|value| value.eq_ignore_ascii_case(name))
        })
}

fn apply_elems(ports: &mut [PaintPort], modification: Element<'_>) {
    let elems = modification
        .children()
        .filter(|child| child.name() == "Elems")
        .flat_map(Element::children)
        .filter(|elem| elem.name() == "Elem");
    for elem in elems {
        let (Some(id), Some(name), Some(value)) = (
            elem.attribute("idRef"),
            elem.attribute("name"),
            elem.attribute("value"),
        ) else {
            continue;
        };
        for port in ports
            .iter_mut()
            .filter(|port| port.id.as_deref() == Some(id))
        {
            if name.eq_ignore_ascii_case("requiredTags") {
                port.required_tags = tags(Some(value));
            } else if name.eq_ignore_ascii_case("portTags") {
                port.port_tags = tags(Some(value));
            }
        }
    }
}
