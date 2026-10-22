use std::collections::hash_map::Entry as MapEntry;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, CancelToken, Entry, Guid};
use rayon::prelude::*;

use crate::CatalogError;
use crate::starmap_index::{Index, SOCPAK, key, unreferenced};
use crate::starmap_report::{ContainerCounts, Placement, StarmapReport, apart, clip};
use crate::starmap_transform::Transform;
use crate::starmap_xml::{Node, attribute, first_below, pose};

const CHILDREN: &str = "ChildObjectContainers";
const CHILD: &str = "Child";

/// The largest container XML that is read, 128 MiB.
const MAX_XML_LEN: u64 = 128 << 20;

/// The most lines the report keeps per star system of the containers read, and of the issues.
const REPORT_LINES: usize = 300;

/// The longest line the report keeps.
const LINE_LEN: usize = 300;

/// What bounds the walk through a star system.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// The most Child elements one star system may place, itself and its external containers.
    pub(crate) children: usize,
    /// The deepest chain of Child elements that is followed.
    pub(crate) depth: usize,
    /// The deepest chain of external containers that is opened.
    pub(crate) external: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            children: 1 << 22,
            depth: 64,
            external: 8,
        }
    }
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
    /// the systems taken in the order of their folder names; one that more than one Child places
    /// at places more than `SAME_PLACE` metres apart has no position.
    pub(crate) fn read(
        archive: &Archive,
        cancel: Option<&CancelToken>,
    ) -> Result<Self, CatalogError> {
        Self::read_within(archive, cancel, Limits::default())
    }

    pub(crate) fn read_within(
        archive: &Archive,
        cancel: Option<&CancelToken>,
        limits: Limits,
    ) -> Result<Self, CatalogError> {
        let index = Index::new(archive);
        let systems = index.systems();
        let read: Vec<SystemRead> = systems
            .par_iter()
            .map(|(name, entry)| System::run(archive, &index, name, *entry, cancel, limits))
            .collect();
        if cancel.is_some_and(CancelToken::is_cancelled) {
            return Err(CatalogError::Cancelled);
        }
        let mut placements = Self::default();
        placements.counts.systems = systems.len();
        let mut repeated: HashSet<Guid> = HashSet::new();
        let mut opened: HashSet<usize> = HashSet::new();
        let mut named = 0;
        for system in read {
            opened.extend(system.opened.iter().copied());
            named += system.named;
            placements.counts.unreadable += system.unreadable;
            placements.counts.missing += system.missing;
            placements.counts.malformed += system.malformed;
            placements
                .report
                .containers
                .extend(system.report.containers);
            placements.report.issues.extend(system.report.issues);
            for (record, placement) in system.placed {
                placements.add(record, placement, &mut repeated);
            }
        }
        placements.counts.ambiguous = repeated.len();
        placements
            .report
            .containers
            .push(format!("{named} containers name starmap records"));
        for record in &repeated {
            placements.by_record.remove(record);
        }
        placements.report.unreferenced = unreferenced(&index, &opened);
        Ok(placements)
    }

    /// Keeps the first placement of `record`; a later one more than `SAME_PLACE` metres from it
    /// puts the record into `repeated`, to be left unplaced.
    fn add(&mut self, record: Guid, placement: Placement, repeated: &mut HashSet<Guid>) {
        let MapEntry::Occupied(held) = self.by_record.entry(record) else {
            self.by_record.insert(record, placement);
            return;
        };
        let moved = apart(held.get().position, placement.position);
        if moved {
            repeated.insert(record);
        }
        if self.report.repeated.len() < 100 {
            self.report.repeated.push(format!(
                "{record}: {} {}/{} {:?}, also {}/{} {:?}",
                if moved {
                    "left unplaced:"
                } else {
                    "same place:"
                },
                held.get().system,
                clip(&held.get().entity_name),
                held.get().position,
                placement.system,
                clip(&placement.entity_name),
                placement.position
            ));
        }
    }
}

#[derive(Default)]
struct SystemRead {
    placed: Vec<(Guid, Placement)>,
    unreadable: usize,
    missing: usize,
    malformed: usize,
    named: usize,
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
    limits: Limits,
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
        limits: Limits,
    ) -> SystemRead {
        let mut system = Self {
            archive,
            index,
            cancel,
            name: Arc::clone(name),
            read: SystemRead::default(),
            limits,
            budget: limits.children,
            open: Vec::new(),
            seen: [0; 3],
        };
        system.container(entry, Transform::IDENTITY, None, 0);
        system.read
    }

    /// Keeps `line` in the report of the issues while there is room.
    fn issue(&mut self, line: &str) {
        if self.read.report.issues.len() < REPORT_LINES {
            self.read
                .report
                .issues
                .push(line.chars().take(LINE_LEN).collect());
        }
    }

    /// Counts a Child, or a limit, that left objects without a position, and reports it.
    fn skipped(&mut self, line: &str) {
        self.read.malformed += 1;
        self.issue(line);
    }

    /// Reads the container at `entry` placed at `at`; its Children name `under` as the record
    /// they hang below.
    fn container(&mut self, entry: usize, at: Transform, under: Option<Guid>, external: usize) {
        if self.cancel.is_some_and(CancelToken::is_cancelled) {
            return;
        }
        let label = self
            .index
            .entries
            .get(entry)
            .map_or_else(String::new, |socpak| socpak.path().to_owned());
        if self.open.contains(&entry) {
            self.skipped(&format!("{label}: names a container that is open above it"));
            return;
        }
        let bytes = match self.xml(entry) {
            Ok(bytes) => bytes,
            Err(reason) => {
                self.read.unreadable += 1;
                self.issue(&format!("{label}: {reason}"));
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
        if !walked {
            self.read.unreadable += 1;
            self.issue(&format!("{label}: the XML cannot be parsed"));
        } else if self.seen[1] > before[1] {
            self.read.named += 1;
            if self.read.report.containers.len() < REPORT_LINES {
                self.read.report.containers.push(format!(
                    "{label}: {} children, {} name a record, {} external",
                    self.seen[0] - before[0],
                    self.seen[1] - before[1],
                    self.seen[2] - before[2]
                ));
            }
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
        if depth >= self.limits.depth {
            let limit = self.limits.depth;
            self.skipped(&format!("{label}: Children nested deeper than {limit}"));
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
                let limit = self.limits.children;
                self.skipped(&format!("{label}: more than {limit} Children in a system"));
                return;
            }
            self.budget -= 1;
            self.seen[0] += 1;
            let Some(world) = pose(child)
                .map(|(position, rotation)| parent.place(position, rotation))
                .filter(|world| world.is_sane())
            else {
                self.skipped(&format!(
                    "{label}: malformed or out of range pos {} rot {} of {}",
                    clip(attribute(child, "pos").unwrap_or_default()),
                    clip(attribute(child, "rot").unwrap_or_default()),
                    clip(attribute(child, "entityName").unwrap_or_default())
                ));
                continue;
            };
            let record = self.note(child, world, under);
            let below = record.or(under);
            if attribute(child, "external") == Some("1")
                && let Some(path) = attribute(child, "name").filter(|path| !path.is_empty())
            {
                self.follow(path, world, below, external, label);
            }
            self.children(child, world, below, external, depth + 1, label);
        }
    }

    /// Opens the external container `path` below a Child placed at `world`.
    fn follow(
        &mut self,
        path: &str,
        world: Transform,
        below: Option<Guid>,
        external: usize,
        label: &str,
    ) {
        self.seen[2] += 1;
        if external >= self.limits.external {
            let limit = self.limits.external;
            self.skipped(&format!(
                "{label}: external containers nested deeper than {limit}: {}",
                clip(path)
            ));
        } else if let Some(entry) = self.index.find(path) {
            self.container(entry, world, below, external + 1);
        } else {
            self.read.missing += 1;
            self.issue(&format!(
                "{label}: the Archive holds no container {}",
                clip(path)
            ));
        }
    }

    /// Records the placement of `child` when it names a starmap record and returns the record.
    fn note<'d>(
        &mut self,
        child: impl Node<'d>,
        world: Transform,
        under: Option<Guid>,
    ) -> Option<Guid> {
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
