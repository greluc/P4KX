use std::collections::HashMap;

use super::ReferenceIndex;
use super::collect::Chunk;

/// Joins the chunks, which arrive in Record order, into one index of at most `limit` References.
/// The caller drops each chunk once it is pushed, so only one wave of chunks lives at a time.
pub(super) struct Assembler {
    index: ReferenceIndex,
    shape_ids: HashMap<Box<str>, u32>,
    limit: usize,
}

impl Assembler {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            index: ReferenceIndex::default(),
            shape_ids: HashMap::new(),
            limit,
        }
    }

    /// Returns whether References were left out, so that no later chunk can add any.
    pub(super) const fn is_full(&self) -> bool {
        self.index.truncated
    }

    /// Appends the References of `chunk`, those past the limit are dropped and mark the index as
    /// truncated.
    pub(super) fn push(&mut self, chunk: &Chunk) {
        if self.index.truncated {
            return;
        }
        self.index.unreadable += chunk.unreadable;
        let room = self.limit.saturating_sub(self.index.sources.len());
        let take = chunk.sources.len().min(room);
        if take < chunk.sources.len() {
            self.index.truncated = true;
        }
        let base = self.index.sources.len();
        let remap = remap_shapes(&mut self.index, &mut self.shape_ids, chunk);
        let Some(index_base) = u32::try_from(self.index.indices.len()).ok() else {
            self.index.truncated = true;
            return;
        };
        let indices_end = chunk
            .index_starts
            .get(take)
            .map_or(chunk.indices.len(), |end| *end as usize);
        let index = &mut self.index;
        index.sources.extend_from_slice(head(&chunk.sources, take));
        index.targets.extend_from_slice(head(&chunk.targets, take));
        index.shapes.extend(
            head(&chunk.shapes, take)
                .iter()
                .filter_map(|local| remap.get(*local as usize)),
        );
        let shifted = head(&chunk.index_starts, take)
            .iter()
            .map(|start| start.saturating_add(index_base));
        index.index_starts.extend(shifted);
        index
            .indices
            .extend_from_slice(chunk.indices.get(..indices_end).unwrap_or_default());
        for (local, guid) in &chunk.unresolved {
            if (*local as usize) < take
                && let Some(entry) = u32::try_from(base + *local as usize).ok()
            {
                index.unresolved.push((entry, *guid));
            }
        }
    }

    /// Builds the lookup tables of the index over `record_count` Records.
    pub(super) fn finish(mut self, record_count: usize) -> ReferenceIndex {
        self.index.finish(record_count);
        self.index
    }
}

fn head<T>(items: &[T], count: usize) -> &[T] {
    items.get(..count).unwrap_or_default()
}

/// Adds the shapes of `chunk` the index lacks and returns the index id of each local shape.
fn remap_shapes(
    index: &mut ReferenceIndex,
    known: &mut HashMap<Box<str>, u32>,
    chunk: &Chunk,
) -> Vec<u32> {
    let mut local: Vec<(&str, u32)> = chunk
        .shape_ids
        .iter()
        .map(|(text, id)| (&**text, *id))
        .collect();
    local.sort_unstable_by_key(|(_, id)| *id);
    local
        .into_iter()
        .map(|(text, _)| {
            if let Some(id) = known.get(text) {
                return *id;
            }
            let id = u32::try_from(index.shape_ends.len()).unwrap_or(u32::MAX);
            index.shape_text.push_str(text);
            index
                .shape_ends
                .push(u32::try_from(index.shape_text.len()).unwrap_or(u32::MAX));
            index
                .shape_arrays
                .push(u16::try_from(text.matches("[]").count()).unwrap_or(u16::MAX));
            known.insert(text.into(), id);
            id
        })
        .collect()
}
