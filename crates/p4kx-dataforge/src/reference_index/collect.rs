use std::collections::HashMap;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{DataForge, Field, Guid, Record, TreeBuilder, Value, ValueError};

use super::UNRESOLVED;

/// The Records each parallel task reads.
pub(super) const RECORDS_PER_TASK: u32 = 512;

/// The References of a run of Records in the order of the Records, with field shapes and array
/// indices stored in shared buffers.
#[derive(Debug, Default)]
pub(super) struct Chunk {
    pub(super) sources: Vec<u32>,
    pub(super) targets: Vec<u32>,
    pub(super) shapes: Vec<u32>,
    pub(super) index_starts: Vec<u32>,
    pub(super) indices: Vec<u32>,
    pub(super) unresolved: Vec<(u32, [u8; 16])>,
    pub(super) shape_ids: HashMap<Box<str>, u32>,
    pub(super) unreadable: u32,
}

impl Chunk {
    fn shape(&mut self, text: &str) -> u32 {
        if let Some(id) = self.shape_ids.get(text) {
            return *id;
        }
        let id = u32::try_from(self.shape_ids.len()).unwrap_or(u32::MAX);
        self.shape_ids.insert(text.into(), id);
        id
    }
}

/// Reads the References of the Records in `range` into one chunk, stopping once `budget` has
/// counted more than `limit` References over all chunks.
pub(super) fn collect<'f>(
    forge: &'f DataForge<'f>,
    builder: &mut TreeBuilder<'f>,
    range: Range<u32>,
    budget: &AtomicUsize,
    limit: usize,
) -> Chunk {
    let mut chunk = Chunk::default();
    let mut walker = Walker {
        forge,
        source: 0,
        own: Guid::default(),
        path: String::new(),
        indices: Vec::new(),
        chunk: &mut chunk,
    };
    for index in range {
        if budget.load(Ordering::Relaxed) > limit {
            break;
        }
        let Some(record) = forge.record(index) else {
            continue;
        };
        let before = walker.chunk.sources.len();
        if !walker.record(builder, &record) {
            walker.chunk.unreadable += 1;
        }
        let added = walker.chunk.sources.len().saturating_sub(before);
        budget.fetch_add(added, Ordering::Relaxed);
    }
    chunk
}

struct Walker<'w, 'f> {
    forge: &'f DataForge<'f>,
    source: u32,
    own: Guid,
    path: String,
    indices: Vec<u32>,
    chunk: &'w mut Chunk,
}

impl<'f> Walker<'_, 'f> {
    fn record(&mut self, builder: &mut TreeBuilder<'f>, record: &Record<'f>) -> bool {
        let Ok(tree) = builder.build(record) else {
            return false;
        };
        self.source = record.index();
        self.own = tree.guid();
        self.path.clear();
        self.indices.clear();
        self.fields(tree.fields());
        true
    }

    fn fields(&mut self, fields: &[Field<'_>]) {
        for field in fields {
            let mark = self.path.len();
            if mark > 0 {
                self.path.push('.');
            }
            self.path.push_str(field.name());
            self.value(field.value());
            self.path.truncate(mark);
        }
    }

    fn value(&mut self, value: &Value<'_>) {
        match value {
            Value::Struct(structure) => self.fields(structure.fields()),
            Value::Array(items) => {
                let mark = self.path.len();
                for (index, item) in (0_u32..).zip(items.iter()) {
                    self.path.push_str("[]");
                    self.indices.push(index);
                    self.value(item);
                    self.indices.pop();
                    self.path.truncate(mark);
                }
            }
            Value::Reference(reference) if reference.guid() != self.own => {
                let target = self
                    .forge
                    .record_by_guid(reference.guid())
                    .map_or(UNRESOLVED, |record| record.index());
                self.push(target, reference.guid());
            }
            Value::Error(ValueError::DanglingReference { guid }) => self.push(UNRESOLVED, *guid),
            _ => {}
        }
    }

    fn push(&mut self, target: u32, guid: Guid) {
        let shape = self.chunk.shape(&self.path);
        let start = u32::try_from(self.chunk.indices.len()).unwrap_or(u32::MAX);
        let entry = u32::try_from(self.chunk.sources.len()).unwrap_or(u32::MAX);
        self.chunk.sources.push(self.source);
        self.chunk.targets.push(target);
        self.chunk.shapes.push(shape);
        self.chunk.index_starts.push(start);
        self.chunk.indices.extend_from_slice(&self.indices);
        if target == UNRESOLVED {
            self.chunk.unresolved.push((entry, guid.stored()));
        }
    }
}
