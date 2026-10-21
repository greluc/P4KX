use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_dataforge::{DataForge, Guid, Record, TreeBuilder};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::entities::is_entity;
use crate::spec::{EntitySpec, EntrySpec, LoadoutSpec, TargetSpec};
use crate::{
    Entities, EntityNode, LandingGear, LoadoutError, LoadoutFiles, MAX_LOADOUT_DEPTH,
    VehicleImplementation, vehicle, xml,
};

pub(crate) type SpecResult = Arc<Result<EntitySpec, LoadoutError>>;
pub(crate) type EntriesResult = Result<Entries, LoadoutError>;
pub(crate) type VehicleResult = Result<Arc<VehicleImplementation>, LoadoutError>;

/// The entries of a Loadout and why parts of it cannot be read.
#[derive(Debug, Clone)]
pub(crate) struct Entries {
    pub(crate) list: Arc<[EntrySpec]>,
    pub(crate) issues: Arc<[LoadoutError]>,
}

impl Entries {
    fn complete(list: Arc<[EntrySpec]>) -> Self {
        Self {
            list,
            issues: Arc::from([]),
        }
    }
}

/// Resolves the default Loadouts of Entities into [`EntityNode`] trees.
///
/// Records are read once per resolver and kept, so resolving several Entities with one resolver
/// reads each Record once; the Records an Entity reaches are read in parallel. Resolution is
/// cycle-safe and bounded by [`MAX_LOADOUT_DEPTH`](crate::MAX_LOADOUT_DEPTH) and
/// [`MAX_LOADOUT_PORTS`](crate::MAX_LOADOUT_PORTS); a port that cannot be resolved is kept in the
/// tree with the reason.
pub struct LoadoutResolver<'f> {
    pub(crate) entities: Entities<'f>,
    files: Option<&'f dyn LoadoutFiles>,
    specs: HashMap<Guid, SpecResult>,
    xml: HashMap<String, EntriesResult>,
    vehicles: HashMap<String, VehicleResult>,
    landing: HashMap<Guid, Arc<[LandingGear]>>,
    builder: TreeBuilder<'f>,
}

impl std::fmt::Debug for LoadoutResolver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadoutResolver")
            .field("entities", &self.entities)
            .field("files", &self.files.is_some())
            .field("records", &self.specs.len())
            .finish_non_exhaustive()
    }
}

impl<'f> LoadoutResolver<'f> {
    /// Creates a resolver over `forge` that reports XML Loadouts as not read.
    #[must_use]
    pub fn new(forge: &'f DataForge<'f>) -> Self {
        Self {
            entities: Entities::new(forge),
            files: None,
            specs: HashMap::new(),
            xml: HashMap::new(),
            vehicles: HashMap::new(),
            landing: HashMap::new(),
            builder: TreeBuilder::new(),
        }
    }

    /// Reads the XML Loadouts Records name from `files`.
    #[must_use]
    pub fn with_files(mut self, files: &'f dyn LoadoutFiles) -> Self {
        self.files = Some(files);
        self.xml.clear();
        self.vehicles.clear();
        self
    }

    /// Returns the Entities, to find the one to resolve.
    #[must_use]
    pub const fn entities(&self) -> &Entities<'f> {
        &self.entities
    }

    /// Resolves the default Loadout of `entity` into its tree.
    ///
    /// # Errors
    ///
    /// Returns [`LoadoutError::NotAnEntity`] when the Record is no Entity and
    /// [`LoadoutError::UnreadableRecord`] when its tree cannot be built; every error below the
    /// root is kept in the tree instead.
    pub fn resolve(&mut self, entity: &Record<'f>) -> Result<EntityNode, LoadoutError> {
        self.prefetch(*entity);
        let spec = self.spec(entity.guid());
        let spec = spec.as_ref().as_ref().map_err(Clone::clone)?;
        let mut walk = crate::node::Walk::new(entity.guid());
        Ok(self.node(spec, None, &mut walk))
    }

    /// Returns the cached spec of the Record `guid`, reading it on a miss.
    pub(crate) fn spec(&mut self, guid: Guid) -> SpecResult {
        if let Some(spec) = self.specs.get(&guid) {
            return Arc::clone(spec);
        }
        let forge = self.entities.forge();
        let spec = Arc::new(match forge.record_by_guid(guid) {
            Some(record) => read_spec(&mut self.builder, &record),
            None => Err(LoadoutError::DanglingReference { guid }),
        });
        self.specs.insert(guid, Arc::clone(&spec));
        spec
    }

    /// Returns the entries of a Loadout, reading an XML Loadout once per resolver.
    pub(crate) fn entries(&mut self, loadout: Option<&LoadoutSpec>) -> EntriesResult {
        match loadout {
            None => Ok(Entries::complete(Arc::from([]))),
            Some(LoadoutSpec::Manual(entries)) => Ok(Entries::complete(Arc::clone(entries))),
            Some(LoadoutSpec::Xml(path)) => {
                let Some(files) = self.files else {
                    return Err(LoadoutError::NoLoadoutFiles { path: path.clone() });
                };
                self.xml
                    .entry(path.to_ascii_lowercase())
                    .or_insert_with(|| {
                        xml::load(files, path).map(|loaded| Entries {
                            list: Arc::from(loaded.entries),
                            issues: Arc::from(loaded.issues),
                        })
                    })
                    .clone()
            }
        }
    }

    /// Returns the vehicle implementation `spec` names with its Modification, read once per
    /// resolver, or `None` when the Entity names none or no files were given.
    pub(crate) fn vehicle_of(&mut self, spec: &EntitySpec) -> Option<VehicleResult> {
        let vehicle_spec = spec.vehicle.as_ref()?;
        let files = self.files?;
        let class_name = crate::entities::class_name_of(&spec.name).unwrap_or(&spec.name);
        if vehicle_spec.definition.is_empty() {
            return None;
        }
        let key = format!(
            "{}\0{}\0{}",
            vehicle_spec.definition.to_ascii_lowercase(),
            vehicle_spec.modification.to_ascii_lowercase(),
            class_name.to_ascii_lowercase()
        );
        let loaded = self.vehicles.entry(key).or_insert_with(|| {
            vehicle::load(
                files,
                class_name,
                &vehicle_spec.definition,
                &vehicle_spec.modification,
            )
            .map(Arc::new)
        });
        Some(loaded.clone())
    }

    /// Returns the gears of the landing gear Record the vehicle component of spec names, read once
    /// per resolver; empty when it names none or the Record cannot be read.
    pub(crate) fn landing_gear_of(&mut self, spec: &EntitySpec) -> Arc<[LandingGear]> {
        let Some(guid) = spec
            .vehicle
            .as_ref()
            .and_then(|vehicle| vehicle.landing_system)
        else {
            return Arc::from([]);
        };
        if let Some(gears) = self.landing.get(&guid) {
            return Arc::clone(gears);
        }
        let gears = self
            .entities
            .forge()
            .record_by_guid(guid)
            .and_then(|record| self.builder.build(&record).ok())
            .map_or_else(|| Arc::from([]), |tree| vehicle::gears(&tree));
        self.landing.insert(guid, Arc::clone(&gears));
        gears
    }

    /// Returns the Record a target names, or `None` when there is none.
    pub(crate) fn record_of(&self, target: &TargetSpec) -> Option<Record<'f>> {
        match target {
            TargetSpec::Reference(guid) => self.entities.forge().record_by_guid(*guid),
            TargetSpec::ClassName(name) => self.entities.by_class_name(name),
            TargetSpec::None | TargetSpec::Invalid(_) => None,
        }
    }

    /// Reads the specs of the Entities reachable from `root` through Record Loadouts and port
    /// defaults in parallel, level by level, as deep as a tree is resolved.
    fn prefetch(&mut self, root: Record<'f>) {
        let mut frontier = vec![root];
        for _ in 0..=MAX_LOADOUT_DEPTH {
            let (cached, missing): (Vec<_>, Vec<_>) = frontier
                .into_iter()
                .partition(|record| self.specs.contains_key(&record.guid()));
            let read: Vec<(Guid, SpecResult)> = missing
                .into_par_iter()
                .map_init(TreeBuilder::new, |builder, record| {
                    (record.guid(), Arc::new(read_spec(builder, &record)))
                })
                .collect();
            let mut level: Vec<SpecResult> = cached
                .iter()
                .filter_map(|record| self.specs.get(&record.guid()).cloned())
                .collect();
            for (guid, spec) in read {
                level.push(Arc::clone(&spec));
                self.specs.insert(guid, spec);
            }
            frontier = self.unread_targets(&level);
            if frontier.is_empty() {
                break;
            }
        }
    }

    /// Returns the Records the specs name that are not read yet, each once.
    fn unread_targets(&self, specs: &[SpecResult]) -> Vec<Record<'f>> {
        let mut seen = HashSet::new();
        specs
            .iter()
            .filter_map(|spec| spec.as_ref().as_ref().ok())
            .flat_map(EntitySpec::targets)
            .filter_map(|target| self.record_of(target))
            .filter(|record| !self.specs.contains_key(&record.guid()) && seen.insert(record.guid()))
            .collect()
    }
}
/// Reads the spec of one Record, which must be an Entity.
fn read_spec<'f>(
    builder: &mut TreeBuilder<'f>,
    record: &Record<'f>,
) -> Result<EntitySpec, LoadoutError> {
    let guid = record.guid();
    if !is_entity(record) {
        return Err(LoadoutError::NotAnEntity {
            guid,
            type_name: record
                .type_name()
                .unwrap_or("Record of an unreadable type")
                .to_owned(),
        });
    }
    builder
        .build(record)
        .map(|tree| EntitySpec::read(&tree))
        .map_err(|error| LoadoutError::UnreadableRecord {
            guid,
            reason: error.to_string(),
        })
}
