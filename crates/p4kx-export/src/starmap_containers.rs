use std::collections::hash_map::Entry as MapEntry;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Entry, Guid};
use rayon::prelude::*;

use crate::CatalogError;
use crate::starmap_index::{Index, SOCPAK, SYSTEM_FOLDER, key};
use crate::starmap_transform::{Rotation, Transform};
use crate::starmap_xml::{Node, attribute, first_below, pose};

const CHILDREN: &str = "ChildObjectContainers";
const CHILD: &str = "Child";

/// The largest container XML that is read, 128 MiB.
const MAX_XML_LEN: u64 = 128 << 20;

/// The most Child elements one star system may place, itself and its external containers.
const MAX_CHILDREN: usize = 1 << 22;

/// The deepest chain of Child elements that is followed.
const MAX_DEPTH: usize = 64;

/// The most rotations the report quotes.
const ROTATION_SAMPLES: usize = 40;

/// The deepest chain of external containers that is opened.
const MAX_EXTERNAL_DEPTH: usize = 8;

/// Where one Starmap Record is placed: the first Child element that names it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement {
    pub(crate) system: Arc<str>,
    pub(crate) position: [f64; 3],
    pub(crate) entity_name: String,
    pub(crate) placed_under: Option<Guid>,
}

/// What reading the system containers found, besides the placements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ContainerCounts {
    pub(crate) systems: usize,
    pub(crate) unreadable: usize,
    pub(crate) malformed: usize,
    pub(crate) ambiguous: usize,
}

/// What reading the system containers saw, for a person to check the placements by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StarmapReport {
    /// One line per container read: its path, the Child elements below it (nested external
    /// containers included), those naming a starmap record and those naming an external container.
    pub containers: Vec<String>,
    /// One line per container or placement that could not be used, with the reason.
    pub issues: Vec<String>,
    /// One line per record that more than one Child element places, up to 100.
    pub repeated: Vec<String>,
    /// The raw `rot` values of the first Children with a rotation other than the identity, up to
    /// 40, each with its container and entity name.
    pub rotations: Vec<String>,
    /// The socpak containers below ObjectContainers/PU/system that no Child names, up to 200,
    /// the last line giving the total.
    pub unreferenced: Vec<String>,
}

/// The placements of the Starmap Records in the system containers of an Archive.
#[derive(Debug, Default)]
pub(crate) struct Placements {
    by_record: HashMap<Guid, Placement>,
    pub(crate) counts: ContainerCounts,
    pub(crate) report: StarmapReport,
}

impl Placements {
    pub(crate) fn get(&self, record: Guid) -> Option<&Placement> {
        self.by_record.get(&record)
    }

    /// Reads the container of every star system of `archive` and the external containers it
    /// names, composing each Child's position with the positions and rotations of the Children
    /// above it. A record placed more than once keeps its first placement in document order,
    /// the systems taken in the order of their folder names.
    pub(crate) fn read(
        archive: &Archive,
        cancel: Option<&CancelToken>,
    ) -> Result<Self, CatalogError> {
        let index = Index::new(archive);
        let systems = index.systems();
        let read: Vec<SystemRead> = systems
            .par_iter()
            .map(|(name, entry)| System::run(archive, &index, name, *entry, cancel))
            .collect();
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return Err(CatalogError::Cancelled);
        }
        let mut placements = Self::default();
        placements.counts.systems = systems.len();
        let mut repeated: HashSet<Guid> = HashSet::new();
        let mut opened: HashSet<usize> = HashSet::new();
        for system in read {
            opened.extend(system.opened.iter().copied());
            placements.counts.unreadable += system.unreadable;
            placements.counts.malformed += system.malformed;
            placements
                .report
                .containers
                .extend(system.report.containers);
            placements.report.issues.extend(system.report.issues);
            placements.report.rotations.extend(system.report.rotations);
            for (record, placement) in system.placed {
                match placements.by_record.entry(record) {
                    MapEntry::Occupied(held) => {
                        if repeated.insert(record) && placements.report.repeated.len() < 100 {
                            placements.report.repeated.push(format!(
                                "{record}: kept {}/{} {:?}, also {}/{}",
                                held.get().system,
                                held.get().entity_name,
                                held.get().position,
                                placement.system,
                                placement.entity_name
                            ));
                        }
                    }
                    MapEntry::Vacant(slot) => {
                        slot.insert(placement);
                    }
                }
            }
        }
        placements.counts.ambiguous = repeated.len();
        let mut unreferenced: Vec<String> = index
            .by_path
            .iter()
            .filter(|(path, at)| path.starts_with(SYSTEM_FOLDER) && !opened.contains(at))
            .map(|(path, _)| path.clone())
            .collect();
        unreferenced.sort_unstable();
        let total = unreferenced.len();
        unreferenced.truncate(200);
        unreferenced.push(format!("{total} in total"));
        placements.report.unreferenced = unreferenced;
        Ok(placements)
    }
}

#[derive(Default)]
struct SystemRead {
    placed: Vec<(Guid, Placement)>,
    unreadable: usize,
    malformed: usize,
    opened: Vec<usize>,
    report: StarmapReport,
}

/// The walk through one star system.
struct System<'a> {
    archive: &'a Archive,
    index: &'a Index<'a>,
    cancel: Option<&'a CancelToken>,
    name: Arc<str>,
    read: SystemRead,
    budget: usize,
    open: Vec<usize>,
    seen: [usize; 3],
}

impl<'a> System<'a> {
    fn run(
        archive: &'a Archive,
        index: &'a Index<'a>,
        name: &Arc<str>,
        entry: usize,
        cancel: Option<&'a CancelToken>,
    ) -> SystemRead {
        let mut system = Self {
            archive,
            index,
            cancel,
            name: Arc::clone(name),
            read: SystemRead::default(),
            budget: MAX_CHILDREN,
            open: Vec::new(),
            seen: [0; 3],
        };
        system.container(entry, Transform::IDENTITY, None, 0);
        system.read
    }

    /// Reads the container at `entry` placed at `at`; its Children name `under` as the record
    /// they hang below.
    fn container(&mut self, entry: usize, at: Transform, under: Option<Guid>, external: usize) {
        if self.cancel.is_some_and(CancelToken::is_cancelled) || self.open.contains(&entry) {
            return;
        }
        let label = self
            .index
            .entries
            .get(entry)
            .map_or_else(String::new, |socpak| socpak.path().to_owned());
        let bytes = match self.xml(entry) {
            Ok(bytes) => bytes,
            Err(reason) => {
                self.read.unreadable += 1;
                self.read.report.issues.push(format!("{label}: {reason}"));
                return;
            }
        };
        let before = self.seen;
        self.read.opened.push(entry);
        self.open.push(entry);
        let walked = match cryxml::read(&bytes) {
            Ok(Xml::CryXml(document)) => {
                self.children(document.root(), at, under, external, 0, &label);
                true
            }
            Ok(Xml::Text(text)) => std::str::from_utf8(text)
                .ok()
                .map(|text| text.trim_start_matches('\u{feff}'))
                .and_then(|text| roxmltree::Document::parse(text).ok())
                .map(|document| {
                    self.children(document.root_element(), at, under, external, 0, &label);
                })
                .is_some(),
            Err(_) => false,
        };
        self.open.pop();
        if walked {
            self.read.report.containers.push(format!(
                "{label}: {} children, {} name a record, {} external",
                self.seen[0] - before[0],
                self.seen[1] - before[1],
                self.seen[2] - before[2]
            ));
        } else {
            self.read.unreadable += 1;
            self.read
                .report
                .issues
                .push(format!("{label}: the XML cannot be parsed"));
        }
    }

    fn xml(&self, entry: usize) -> Result<Vec<u8>, String> {
        let socpak = self.index.entries.get(entry).ok_or("no such entry")?;
        let container = self
            .archive
            .open_container(socpak)
            .map_err(|error| format!("cannot be opened as a container: {error:#}"))?;
        let stem = key(socpak.path());
        let stem = stem
            .rsplit('\\')
            .next()
            .and_then(|name| name.strip_suffix(SOCPAK))
            .ok_or("not a socpak name")?;
        let main = format!("{stem}.xml");
        let xml = container
            .entries()
            .iter()
            .find(|inner| key(inner.path()).rsplit('\\').next() == Some(main.as_str()))
            .or_else(|| {
                container.entries().iter().find(|inner| {
                    let name = key(inner.path());
                    Path::new(&name)
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
                        && !name.contains("_editor")
                        && !name.contains("metadata")
                })
            })
            .ok_or_else(|| {
                let names: Vec<&str> = container
                    .entries()
                    .iter()
                    .map(Entry::path)
                    .take(8)
                    .collect();
                format!("holds no main XML; its entries start with {names:?}")
            })?;
        if xml.uncompressed_size() > MAX_XML_LEN {
            return Err(format!(
                "its XML holds {} bytes, over the limit",
                xml.uncompressed_size()
            ));
        }
        let mut bytes = Vec::new();
        container
            .open_entry(xml)
            .map_err(|error| format!("its XML cannot be opened: {error:#}"))?
            .take(MAX_XML_LEN)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("its XML cannot be read: {error}"))?;
        Ok(bytes)
    }

    fn children<'d, N: Node<'d>>(
        &mut self,
        node: N,
        parent: Transform,
        under: Option<Guid>,
        external: usize,
        depth: usize,
        label: &str,
    ) {
        if depth >= MAX_DEPTH {
            self.read.malformed += 1;
            self.read
                .report
                .issues
                .push(format!("{label}: Children nested deeper than {MAX_DEPTH}"));
            return;
        }
        let Some(list) = first_below(node, CHILDREN) else {
            return;
        };
        for child in list
            .children()
            .filter(|child| child.name().eq_ignore_ascii_case(CHILD))
        {
            if self.budget == 0 {
                self.read.malformed += 1;
                return;
            }
            self.budget -= 1;
            self.seen[0] += 1;
            let Some((position, rotation)) = pose(child) else {
                self.read.malformed += 1;
                self.read.report.issues.push(format!(
                    "{label}: malformed pos {:?} or rot {:?} of {:?}",
                    attribute(child, "pos"),
                    attribute(child, "rot"),
                    attribute(child, "entityName")
                ));
                continue;
            };
            let world = parent.place(position, rotation);
            let record = self.note(child, rotation != Rotation::IDENTITY, world, under, label);
            let below = record.or(under);
            if attribute(child, "external") == Some("1")
                && external < MAX_EXTERNAL_DEPTH
                && let Some(path) = attribute(child, "name").filter(|path| !path.is_empty())
            {
                self.seen[2] += 1;
                if let Some(entry) = self.index.find(path) {
                    self.container(entry, world, below, external + 1);
                } else {
                    self.read.unreadable += 1;
                    self.read
                        .report
                        .issues
                        .push(format!("{label}: the Archive holds no container {path}"));
                }
            }
            self.children(child, world, below, external, depth + 1, label);
        }
    }

    /// Records the placement of `child` when it names a starmap record, and a sample of its
    /// rotation for the report; returns the record.
    fn note<'d>(
        &mut self,
        child: impl Node<'d>,
        turned: bool,
        world: Transform,
        under: Option<Guid>,
        label: &str,
    ) -> Option<Guid> {
        if turned
            && let Some(raw) = attribute(child, "rot")
            && self.read.report.rotations.len() < ROTATION_SAMPLES
        {
            self.read.report.rotations.push(format!(
                "{label} {:?}: rot {raw} pos {:?}",
                attribute(child, "entityName"),
                attribute(child, "pos")
            ));
        }
        let record = attribute(child, "starMapRecord")
            .and_then(|text| text.trim().parse::<Guid>().ok())
            .filter(|guid| guid.stored() != [0; 16])?;
        self.seen[1] += 1;
        self.read.placed.push((
            record,
            Placement {
                system: Arc::clone(&self.name),
                position: world.position,
                entity_name: attribute(child, "entityName")
                    .unwrap_or_default()
                    .to_owned(),
                placed_under: under,
            },
        ));
        Some(record)
    }
}
