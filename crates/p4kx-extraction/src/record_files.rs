use std::cmp::Ordering;
use std::collections::HashMap;

use p4kx_dataforge::{DataForge, Record};
use rayon::slice::ParallelSliceMut;

use crate::entry_path::relative_path;

const EXTENSION: &str = ".xml";

/// The file below the target folder that each Record of a DataForge is written to (ADR-0008).
///
/// A Record alone at its record path goes to `<record path>.xml`. Records that share a record
/// path go into the folder `<record path>/`, each to `<name>.xml`; when such a file collides with
/// another one ignoring case, or a name is no plain file name, the GUID is used as well. The plan
/// depends on the whole DataForge only, so a Record always gets the same file whatever is
/// selected.
pub(crate) struct RecordFiles {
    files: Box<[Option<String>]>,
}

struct Planned {
    file: String,
    shared: bool,
}

impl RecordFiles {
    /// Plans the file of every Record of `forge`.
    pub(crate) fn plan(forge: &DataForge<'_>) -> Self {
        let records: Vec<Record<'_>> = forge.records().collect();
        let keys: Vec<(Option<&str>, Option<&str>)> = records
            .iter()
            .map(|record| (record.record_path().ok(), record.name().ok()))
            .collect();
        let path = |index: &u32| keys.get(*index as usize).and_then(|key| key.0);
        let name = |index: &u32| keys.get(*index as usize).and_then(|key| key.1);
        let mut order: Vec<u32> = records.iter().map(Record::index).collect();
        order.par_sort_unstable_by(|left, right| {
            compare(path(left), path(right))
                .then_with(|| compare(name(left), name(right)))
                .then(left.cmp(right))
        });
        let mut planned: Vec<Option<Planned>> = Vec::new();
        planned.resize_with(records.len(), || None);
        for group in order.chunk_by(|left, right| compare(path(left), path(right)).is_eq()) {
            for (position, index) in group.iter().enumerate() {
                let (Some(record), Some(slot), Some(record_path)) = (
                    records.get(*index as usize),
                    planned.get_mut(*index as usize),
                    path(index),
                ) else {
                    continue;
                };
                *slot = Some(if group.len() == 1 {
                    Planned {
                        file: format!("{record_path}{EXTENSION}"),
                        shared: false,
                    }
                } else {
                    let collides = |other: Option<&u32>| {
                        other.is_some_and(|other| compare(name(index), name(other)).is_eq())
                    };
                    let unique = !collides(position.checked_sub(1).and_then(|at| group.get(at)))
                        && !collides(group.get(position + 1));
                    Planned {
                        file: shared_file(record_path, name(index), record, unique),
                        shared: true,
                    }
                });
            }
        }
        separate_shared(&mut planned, &records, &keys);
        let mut folders = Folders::default();
        for planned in planned.iter_mut().flatten() {
            folders.canonicalise(&mut planned.file);
        }
        Self {
            files: planned
                .into_iter()
                .map(|planned| planned.map(|planned| planned.file))
                .collect(),
        }
    }

    /// Returns the planned file of `record`, relative to the target with `/` as separator.
    ///
    /// `record` must come from the DataForge the plan was made for.
    pub(crate) fn file(&self, record: &Record<'_>) -> Option<&str> {
        self.files
            .get(record.index() as usize)
            .and_then(Option::as_deref)
    }
}

/// Gives every shared Record whose file collides with another planned file ignoring case its
/// GUID, so a Record alone at its record path keeps `<record path>.xml`.
fn separate_shared(
    planned: &mut [Option<Planned>],
    records: &[Record<'_>],
    keys: &[(Option<&str>, Option<&str>)],
) {
    let mut order: Vec<usize> = (0..planned.len()).collect();
    order.par_sort_unstable_by(|left, right| {
        compare(file(planned, *left), file(planned, *right)).then(left.cmp(right))
    });
    let mut renamed = Vec::new();
    for group in
        order.chunk_by(|left, right| compare(file(planned, *left), file(planned, *right)).is_eq())
    {
        if group.len() > 1 {
            renamed.extend(group.iter().copied());
        }
    }
    for index in renamed {
        let (Some(Some(slot)), Some(record), Some((Some(record_path), name))) =
            (planned.get_mut(index), records.get(index), keys.get(index))
        else {
            continue;
        };
        if slot.shared {
            slot.file = shared_file(record_path, *name, record, false);
        }
    }
}

/// The spelling of every folder: the first Record in definition order that needs a folder names
/// it, so folders that differ only in case are one folder with one name whatever runs first.
#[derive(Default)]
struct Folders {
    spelling: HashMap<String, String>,
}

impl Folders {
    fn canonicalise(&mut self, file: &mut String) {
        let Some((folder, name)) = file.rsplit_once('/') else {
            return;
        };
        let mut key = String::new();
        let mut spelled = String::new();
        for (position, part) in folder.split('/').enumerate() {
            if position > 0 {
                key.push('/');
                spelled.push('/');
            }
            key.extend(fold(part));
            if let Some(spelling) = self.spelling.get(&key) {
                spelled.clone_from(spelling);
            } else {
                spelled.push_str(part);
                self.spelling.insert(key.clone(), spelled.clone());
            }
        }
        if spelled != folder {
            *file = format!("{spelled}/{name}");
        }
    }
}
fn file(planned: &[Option<Planned>], index: usize) -> Option<&str> {
    planned
        .get(index)
        .and_then(Option::as_ref)
        .map(|planned| planned.file.as_str())
}

fn shared_file(record_path: &str, name: Option<&str>, record: &Record<'_>, unique: bool) -> String {
    let guid = record.guid();
    match name {
        Some(name) if is_file_name(name) && unique => format!("{record_path}/{name}{EXTENSION}"),
        Some(name) if is_file_name(name) => format!("{record_path}/{name}.{guid}{EXTENSION}"),
        _ => format!("{record_path}/{guid}{EXTENSION}"),
    }
}

fn is_file_name(name: &str) -> bool {
    !name.contains(['/', '\\']) && relative_path(name).is_some()
}

/// Returns `path` folded the way the file system of the target compares names: lower case,
/// with `/` for `\`.
pub(crate) fn fold(path: &str) -> impl Iterator<Item = char> + '_ {
    path.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == '\\' { '/' } else { c })
}

fn compare(left: Option<&str>, right: Option<&str>) -> Ordering {
    fold(left.unwrap_or_default()).cmp(fold(right.unwrap_or_default()))
}
