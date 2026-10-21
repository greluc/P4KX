use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, TryLockError};
use std::thread;
use std::time::Duration;

use p4kx_export::RecordFilePlans;
use p4kx_extraction::DataForgeEntry;
use p4kx_extraction::ReferenceIndex;
use p4kx_extraction::TagTree;
use p4kx_extraction::query::QuerySchema;

use crate::EntryIndex;
use crate::model::describe;

/// How often a consumer waiting for the build of the index of References looks at its stop flag.
const TURN_POLL: Duration = Duration::from_millis(25);

/// The DataForge of one open Archive as every consumer reads it: its Entry, read once, the files
/// of its Records, planned once per form, and the Query completion schema, built once.
pub(crate) struct ForgeSource {
    pub(crate) entry: Arc<DataForgeEntry>,
    pub(crate) plans: RecordFilePlans,
    schema: OnceLock<Arc<QuerySchema>>,
    schema_turn: Mutex<()>,
    schema_builds: AtomicUsize,
    references: OnceLock<Arc<ReferenceIndex>>,
    references_turn: Mutex<()>,
    references_builds: AtomicUsize,
    tags: OnceLock<Arc<TagTree>>,
    tags_turn: Mutex<()>,
}

impl ForgeSource {
    fn new(entry: DataForgeEntry) -> Self {
        Self {
            entry: Arc::new(entry),
            plans: RecordFilePlans::default(),
            schema: OnceLock::new(),
            schema_turn: Mutex::new(()),
            schema_builds: AtomicUsize::new(0),
            references: OnceLock::new(),
            references_turn: Mutex::new(()),
            references_builds: AtomicUsize::new(0),
            tags: OnceLock::new(),
            tags_turn: Mutex::new(()),
        }
    }

    /// Returns the completion schema if it was built.
    pub(crate) fn schema_if_built(&self) -> Option<Arc<QuerySchema>> {
        self.schema.get().cloned()
    }

    /// Returns the completion schema, building it from the DataForge already read unless it was
    /// built before. The build runs outside the [`OnceLock`], because it uses rayon.
    ///
    /// # Errors
    ///
    /// Returns why the DataForge cannot be opened.
    pub(crate) fn schema(&self) -> Result<Arc<QuerySchema>, ReadFailure> {
        let _turn = self
            .schema_turn
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(schema) = self.schema.get() {
            return Ok(Arc::clone(schema));
        }
        let forge = self
            .entry
            .open()
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        self.schema_builds.fetch_add(1, Ordering::Relaxed);
        let built = Arc::new(QuerySchema::of(&forge));
        Ok(Arc::clone(self.schema.get_or_init(|| built)))
    }
}

impl ForgeSource {
    /// Returns the tag tree of the DataForge already read, building it unless it was built before.
    /// The build runs outside the [`OnceLock`], and one at a time, so that it happens once per read.
    ///
    /// # Errors
    ///
    /// Returns why the DataForge cannot be opened.
    pub(crate) fn tags(&self) -> Result<Arc<TagTree>, ReadFailure> {
        let _turn = self
            .tags_turn
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(tags) = self.tags.get() {
            return Ok(Arc::clone(tags));
        }
        let forge = self
            .entry
            .open()
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        let built = Arc::new(TagTree::build(&forge));
        Ok(Arc::clone(self.tags.get_or_init(|| built)))
    }

    /// Returns the index of the References between the Records, building it from the DataForge
    /// already read unless it was built before, or `None` once `stop` returns `true`, while
    /// building or while waiting for the build of another consumer. The build runs outside the
    /// [`OnceLock`], because it uses rayon, and one at a time, so that it happens once per read.
    ///
    /// # Errors
    ///
    /// Returns why the DataForge cannot be opened.
    pub(crate) fn references(
        &self,
        stop: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Arc<ReferenceIndex>>, ReadFailure> {
        if let Some(references) = self.references.get() {
            return Ok(Some(Arc::clone(references)));
        }
        let _turn = loop {
            match self.references_turn.try_lock() {
                Ok(turn) => break turn,
                Err(TryLockError::Poisoned(poisoned)) => break poisoned.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    if stop() {
                        return Ok(None);
                    }
                    thread::sleep(TURN_POLL);
                }
            }
        };
        if let Some(references) = self.references.get() {
            return Ok(Some(Arc::clone(references)));
        }
        let forge = self
            .entry
            .open()
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        let Some(built) = ReferenceIndex::build_cancellable(&forge, stop) else {
            return Ok(None);
        };
        self.references_builds.fetch_add(1, Ordering::Relaxed);
        let built = Arc::new(built);
        Ok(Some(Arc::clone(self.references.get_or_init(|| built))))
    }
}

impl fmt::Debug for ForgeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForgeSource")
            .field("entry", &self.entry.path())
            .field("plans", &self.plans)
            .field("schema", &self.schema.get().is_some())
            .field("schema_builds", &self.schema_builds)
            .field("references", &self.references.get().is_some())
            .field("references_builds", &self.references_builds)
            .field("tags", &self.tags.get().is_some())
            .finish_non_exhaustive()
    }
}

/// Why the DataForge of an Archive could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    /// The Archive changed on disk before or while it was read, for the reason given.
    ArchiveChanged(String),
    /// The Entry cannot be read, for the reason given.
    Unreadable(String),
}

impl ReadFailure {
    /// Returns the reason.
    pub(crate) fn into_reason(self) -> String {
        match self {
            Self::ArchiveChanged(reason) | Self::Unreadable(reason) => reason,
        }
    }
}

/// The one DataForge cache of an open Archive, shared by the Record tree, the Record preview, the
/// Query tab and the Record export.
///
/// The first consumer that needs DataForge reads it; the others wait for that read instead of
/// reading it again. A read is kept even when the consumer that started it was cancelled, until
/// the Archive changes on disk. [`clear`](Self::clear) never waits for a read.
#[derive(Debug, Default)]
pub(crate) struct ForgeCache {
    reading: Mutex<()>,
    source: Mutex<Option<Arc<ForgeSource>>>,
    generation: AtomicU64,
    reads: AtomicUsize,
    schema_asked: AtomicU64,
    schema_attempted: AtomicU64,
}

impl ForgeCache {
    /// Returns whether DataForge was read; `false` while it is being read.
    pub(crate) fn is_read(&self) -> bool {
        self.source
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    /// Returns how often DataForge was read from the Archive, whether or not the read succeeded.
    pub(crate) fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    /// Returns the DataForge of `index`, reading it unless it was read before; `reading` is
    /// called before a read starts.
    ///
    /// # Errors
    ///
    /// Returns why DataForge cannot be read, or that the Archive changed on disk before or while
    /// it was read.
    pub(crate) fn read(
        &self,
        index: &EntryIndex,
        reading: impl FnOnce(),
    ) -> Result<Arc<ForgeSource>, ReadFailure> {
        let _turn = self.reading.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(source) = self.cached() {
            return Ok(source);
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        let generation = self.generation.load(Ordering::Acquire);
        let changed = |error| ReadFailure::ArchiveChanged(describe(&error));
        index.watch().check().map_err(changed)?;
        reading();
        let entry = DataForgeEntry::read(index.archive())
            .map_err(|error| ReadFailure::Unreadable(describe(&error)))?;
        index.watch().check().map_err(changed)?;
        let read = Arc::new(ForgeSource::new(entry));
        if self.store(generation, &read) {
            Ok(read)
        } else {
            Err(ReadFailure::ArchiveChanged(
                "the DataForge cache was cleared while it was read".to_owned(),
            ))
        }
    }

    /// Returns the completion schema once it was built from the DataForge held now.
    pub(crate) fn schema(&self) -> Option<Arc<QuerySchema>> {
        self.cached()?.schema_if_built()
    }

    /// Returns how often the index of References was built for the DataForge held now.
    pub(crate) fn reference_builds(&self) -> usize {
        self.cached()
            .map_or(0, |source| source.references_builds.load(Ordering::Relaxed))
    }

    /// Returns how often the completion schema was built for the DataForge held now.
    pub(crate) fn schema_builds(&self) -> usize {
        self.cached()
            .map_or(0, |source| source.schema_builds.load(Ordering::Relaxed))
    }

    /// Returns whether the completion schema is missing, no build runs and none was started
    /// since the cache was last cleared.
    pub(crate) fn schema_unattempted(&self) -> bool {
        let current = self.generation.load(Ordering::Acquire).wrapping_add(1);
        self.schema_asked.load(Ordering::Acquire) != current
            && self.schema_attempted.load(Ordering::Acquire) != current
            && self.schema().is_none()
    }

    /// Builds the completion schema of `index` on a background thread, reading its DataForge
    /// unless it was read before, and calls `wake` once it is there. Nothing starts while a build
    /// of the current DataForge runs or after one succeeded; after a failed build or spawn the
    /// next call tries again. A build still running for a DataForge that was cleared meanwhile
    /// does not hold the call back, and its schema is dropped with that DataForge.
    pub(crate) fn prepare_schema(index: &Arc<EntryIndex>, wake: impl Fn() + Send + 'static) {
        let cache = index.forge();
        let current = cache.generation.load(Ordering::Acquire).wrapping_add(1);
        if cache.schema().is_some() || cache.schema_asked.swap(current, Ordering::AcqRel) == current
        {
            return;
        }
        cache.schema_attempted.store(current, Ordering::Release);
        let thread_index = Arc::clone(index);
        let spawned = thread::Builder::new()
            .name("query-schema".to_owned())
            .spawn(move || {
                let cache = thread_index.forge();
                let built = cache
                    .read(&thread_index, || {})
                    .and_then(|source| source.schema());
                let _ = cache.schema_asked.compare_exchange(
                    current,
                    0,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                if built.is_ok() {
                    wake();
                }
            });
        if spawned.is_err() {
            let _ = cache.schema_asked.compare_exchange(
                current,
                0,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    /// Forgets the DataForge read, because the Archive changed on disk, and makes a read still
    /// running discard its result; consumers that hold the DataForge keep their copy until they
    /// end. Returns at once, whatever reads.
    pub(crate) fn clear(&self) {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        self.generation.fetch_add(1, Ordering::AcqRel);
        *source = None;
    }

    fn store(&self, generation: u64, read: &Arc<ForgeSource>) -> bool {
        let mut source = self.source.lock().unwrap_or_else(PoisonError::into_inner);
        let current = self.generation.load(Ordering::Acquire) == generation;
        if current {
            *source = Some(Arc::clone(read));
        }
        current
    }

    fn cached(&self) -> Option<Arc<ForgeSource>> {
        self.source
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests;
