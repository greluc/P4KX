use std::collections::hash_map::Entry as MapEntry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Entry, Guid};
use rayon::prelude::*;

use crate::CatalogError;
use crate::starmap_transform::{Rotation, Transform};
use crate::starmap_xml::{Node, attribute, first_below};

const SYSTEM_FOLDER: &str = "data\\objectcontainers\\pu\\system\\";
const SOCPAK: &str = ".socpak";
const SYSTEM_SUFFIX: &str = "system.socpak";
const CHILDREN: &str = "ChildObjectContainers";
const CHILD: &str = "Child";

/// The largest container XML that is read, 128 MiB.
const MAX_XML_LEN: u64 = 128 << 20;

/// The most Child elements one star system may place, itself and its external containers.
const MAX_CHILDREN: usize = 1 << 22;

/// The deepest chain of Child elements that is followed.
const MAX_DEPTH: usize = 64;

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

/// The placements of the Starmap Records in the system containers of an Archive.
#[derive(Debug, Default)]
pub(crate) struct Placements {
    by_record: HashMap<Guid, Placement>,
    pub(crate) counts: ContainerCounts,
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
        for system in read {
            placements.counts.unreadable += system.unreadable;
            placements.counts.malformed += system.malformed;
            for (record, placement) in system.placed {
                match placements.by_record.entry(record) {
                    MapEntry::Occupied(_) => {
                        repeated.insert(record);
                    }
                    MapEntry::Vacant(slot) => {
                        slot.insert(placement);
                    }
                }
            }
        }
        placements.counts.ambiguous = repeated.len();
        Ok(placements)
    }
}

/// The socpak Entries of an Archive by normalised path.
struct Index<'a> {
    entries: &'a [Entry],
    by_path: HashMap<String, usize>,
}

impl<'a> Index<'a> {
    fn new(archive: &'a Archive) -> Self {
        let entries = archive.entries();
        let by_path = entries
            .iter()
            .enumerate()
            .map(|(at, entry)| (key(entry.path()), at))
            .filter(|(path, _)| path.ends_with(SOCPAK))
            .collect();
        Self { entries, by_path }
    }

    fn find(&self, path: &str) -> Option<usize> {
        let mut wanted = key(path);
        if !wanted.starts_with("data\\") {
            wanted.insert_str(0, "data\\");
        }
        self.by_path.get(&wanted).copied()
    }

    /// Returns the folder name and the Entry of the main container of every star system, ordered
    /// by folder name: `<folder>system.socpak` of a folder below `ObjectContainers\PU\system`,
    /// else the first `*system.socpak` of the folder by path.
    fn systems(&self) -> Vec<(Arc<str>, usize)> {
        let mut folders: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
        for (path, at) in &self.by_path {
            let Some(rest) = path.strip_prefix(SYSTEM_FOLDER) else {
                continue;
            };
            if let Some((folder, file)) = rest.split_once('\\')
                && !file.contains('\\')
                && file.ends_with(SYSTEM_SUFFIX)
            {
                folders.entry(folder).or_default().insert(file, *at);
            }
        }
        folders
            .into_iter()
            .filter_map(|(folder, files)| {
                let own = format!("{folder}{SYSTEM_SUFFIX}");
                let at = files
                    .get(own.as_str())
                    .or_else(|| files.values().next())
                    .copied()?;
                Some((Arc::from(folder), at))
            })
            .collect()
    }
}

/// Returns `path` in lower case with `\` as separator and no leading separator.
fn key(path: &str) -> String {
    path.trim_start_matches(['/', '\\'])
        .chars()
        .map(|c| {
            if c == '/' {
                '\\'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

#[derive(Default)]
struct SystemRead {
    placed: Vec<(Guid, Placement)>,
    unreadable: usize,
    malformed: usize,
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
        let Some(bytes) = self.xml(entry) else {
            self.read.unreadable += 1;
            return;
        };
        self.open.push(entry);
        let walked = match cryxml::read(&bytes) {
            Ok(Xml::CryXml(document)) => {
                self.children(document.root(), at, under, external, 0);
                true
            }
            Ok(Xml::Text(text)) => std::str::from_utf8(text)
                .ok()
                .map(|text| text.trim_start_matches('\u{feff}'))
                .and_then(|text| roxmltree::Document::parse(text).ok())
                .map(|document| self.children(document.root_element(), at, under, external, 0))
                .is_some(),
            Err(_) => false,
        };
        self.open.pop();
        if !walked {
            self.read.unreadable += 1;
        }
    }

    fn xml(&self, entry: usize) -> Option<Vec<u8>> {
        let socpak = self.index.entries.get(entry)?;
        let container = self.archive.open_container(socpak).ok()?;
        let stem = key(socpak.path());
        let stem = stem.rsplit('\\').next()?.strip_suffix(SOCPAK)?;
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
            })?;
        if xml.uncompressed_size() > MAX_XML_LEN {
            return None;
        }
        let mut bytes = Vec::new();
        container
            .open_entry(xml)
            .ok()?
            .take(MAX_XML_LEN)
            .read_to_end(&mut bytes)
            .ok()?;
        Some(bytes)
    }

    fn children<'d, N: Node<'d>>(
        &mut self,
        node: N,
        parent: Transform,
        under: Option<Guid>,
        external: usize,
        depth: usize,
    ) {
        if depth >= MAX_DEPTH {
            self.read.malformed += 1;
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
            let Some((position, rotation)) = pose(child) else {
                self.read.malformed += 1;
                continue;
            };
            let world = parent.place(position, rotation);
            let record = attribute(child, "starMapRecord")
                .and_then(|text| text.trim().parse::<Guid>().ok())
                .filter(|guid| guid.stored() != [0; 16]);
            if let Some(record) = record {
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
            }
            let below = record.or(under);
            if attribute(child, "external") == Some("1")
                && external < MAX_EXTERNAL_DEPTH
                && let Some(path) = attribute(child, "name").filter(|path| !path.is_empty())
            {
                match self.index.find(path) {
                    Some(entry) => self.container(entry, world, below, external + 1),
                    None => self.read.unreadable += 1,
                }
            }
            self.children(child, world, below, external, depth + 1);
        }
    }
}

/// Returns the position and the rotation of a Child: `pos` as `x,y,z` (zero when absent) and
/// `rot` as the quaternion `w,x,y,z` (none when absent); `None` for a value that is malformed.
fn pose<'d>(child: impl Node<'d>) -> Option<([f64; 3], Rotation)> {
    let position = match attribute(child, "pos") {
        Some(text) => numbers::<3>(text)?,
        None => [0.0; 3],
    };
    let rotation = match attribute(child, "rot") {
        Some(text) => Rotation::from_parts(numbers::<4>(text)?)?,
        None => Rotation::IDENTITY,
    };
    Some((position, rotation))
}

fn numbers<const N: usize>(text: &str) -> Option<[f64; N]> {
    let mut parts = text.split(',');
    let mut values = [0.0; N];
    for slot in &mut values {
        let value: f64 = parts.next()?.trim().parse().ok()?;
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    parts.next().is_none().then_some(values)
}
