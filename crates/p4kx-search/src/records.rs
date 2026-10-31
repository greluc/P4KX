// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt::Write as _;

use p4kx_extraction::{DataForge, Field, TreeBuilder, Value};

use crate::excerpt::Excerpt;
use crate::waves::{Chunk, collect};
use crate::{Found, Needle, RecordHit, SearchOptions};

const RECORDS_PER_TASK: usize = 256;
const STOP_POLL_MASK: usize = 0xF;

/// Searches the string, localisation key and enum values of every Record of `forge`.
///
/// A Record is read as its canonical tree, so a field path is the one the Record preview and a
/// Query use. Records are visited in Record order in parallel waves; a Record reports at most
/// [`SearchOptions::per_item`] hits, and the search ends once more than
/// [`SearchOptions::limit`] hits exist. A Record whose tree cannot be built is counted and
/// skipped. Returns `None` once `stop` has returned `true`; it is polled from the worker threads.
#[must_use]
pub fn search_records<'f>(
    forge: &'f DataForge<'f>,
    needle: &Needle,
    options: &SearchOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Option<Found<RecordHit>> {
    let order = forge.path_order();
    let tasks = order.len().div_ceil(RECORDS_PER_TASK);
    collect(tasks, options.hit_limit(), stop, |task| {
        let start = task * RECORDS_PER_TASK;
        let slice = order
            .get(start..order.len().min(start + RECORDS_PER_TASK))
            .unwrap_or_default();
        scan(forge, slice, needle, options, stop)
    })
}

fn scan<'f>(
    forge: &'f DataForge<'f>,
    records: &[u32],
    needle: &Needle,
    options: &SearchOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Chunk<RecordHit> {
    let mut chunk = Chunk::default();
    let mut builder = TreeBuilder::new();
    let mut walker = Walker {
        needle,
        per_item: options.item_limit(),
        path: String::new(),
        found: Vec::new(),
    };
    let limit = options.hit_limit();
    for (position, &index) in records.iter().enumerate() {
        if position & STOP_POLL_MASK == 0 && stop() {
            break;
        }
        let Some(record) = forge.record(index) else {
            continue;
        };
        if (options.filter.is_some() || !options.exclusion.is_empty())
            && !record.record_path().is_ok_and(|path| options.accepts(path))
        {
            continue;
        }
        let Ok(tree) = builder.build(&record) else {
            chunk.stats.records_unreadable += 1;
            continue;
        };
        chunk.stats.records_scanned += 1;
        walker.found.clear();
        walker.path.clear();
        walker.fields(tree.fields());
        for (field, excerpt) in walker.found.drain(..) {
            chunk.hits.push(RecordHit {
                record_index: index,
                guid: tree.guid(),
                record_path: tree.record_path().to_owned(),
                name: tree.name().to_owned(),
                type_name: tree.type_name().to_owned(),
                field,
                excerpt,
            });
        }
        if chunk.hits.len() > limit {
            break;
        }
    }
    chunk
}

struct Walker<'n> {
    needle: &'n Needle,
    per_item: usize,
    path: String,
    found: Vec<(String, Excerpt)>,
}

impl Walker<'_> {
    fn fields(&mut self, fields: &[Field<'_>]) -> bool {
        for field in fields {
            let mark = self.path.len();
            if mark > 0 {
                self.path.push('.');
            }
            self.path.push_str(field.name());
            let more = self.value(field.value());
            self.path.truncate(mark);
            if !more {
                return false;
            }
        }
        true
    }

    fn value(&mut self, value: &Value<'_>) -> bool {
        match value {
            Value::String(text) | Value::Locale(text) | Value::Enum(text) => self.text(text),
            Value::Struct(structure) => self.fields(structure.fields()),
            Value::Array(items) => {
                let mark = self.path.len();
                for (index, item) in items.iter().enumerate() {
                    let _written = write!(self.path, "[{index}]");
                    let more = self.value(item);
                    self.path.truncate(mark);
                    if !more {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn text(&mut self, text: &str) -> bool {
        if let Some(matched) = self.needle.find(text) {
            self.found
                .push((self.path.clone(), Excerpt::around(text, matched)));
        }
        self.found.len() < self.per_item
    }
}
