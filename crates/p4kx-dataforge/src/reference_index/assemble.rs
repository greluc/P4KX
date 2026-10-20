use std::collections::HashMap;

use super::ReferenceIndex;
use super::collect::Chunk;

/// Joins the chunks, which are in Record order, into one index of at most `limit` References.
pub(super) fn assemble(chunks: Vec<Chunk>, record_count: usize, limit: usize) -> ReferenceIndex {
    let total: usize = chunks.iter().map(|chunk| chunk.sources.len()).sum();
    let kept = total.min(limit);
    let mut index = ReferenceIndex {
        sources: Vec::with_capacity(kept),
        targets: Vec::with_capacity(kept),
        shapes: Vec::with_capacity(kept),
        index_starts: Vec::with_capacity(kept),
        truncated: total > limit,
        ..ReferenceIndex::default()
    };
    let mut shape_ids: HashMap<Box<str>, u32> = HashMap::new();
    for chunk in chunks {
        index.unreadable += chunk.unreadable;
        let room = kept.saturating_sub(index.sources.len());
        let take = chunk.sources.len().min(room);
        if take < chunk.sources.len() {
            index.truncated = true;
        }
        let base = index.sources.len();
        let remap = remap_shapes(&mut index, &mut shape_ids, &chunk);
        let Some(index_base) = u32::try_from(index.indices.len()).ok() else {
            index.truncated = true;
            break;
        };
        let indices_end = chunk
            .index_starts
            .get(take)
            .map_or(chunk.indices.len(), |end| *end as usize);
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
        if take < chunk.sources.len() {
            break;
        }
    }
    index.finish(record_count);
    index
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
