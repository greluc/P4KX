// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;

use p4kx_dataforge::{DataForge, Record};
use rayon::slice::ParallelSliceMut;

use crate::RenameCause;
use crate::part_escape::{OVERLONG_FOLDER, escape_record_path};
use crate::portable_name::{check_part, fold};

/// The file below the target folder that each Record of a DataForge is written to (ADR-0008).
///
/// A Record alone at its record path goes to `<record path><extension>`. Records that share a
/// record path go into the folder `<record path>/`, each to `<name><extension>`; when such a file
/// collides with another one ignoring case, or a name is no valid file name on Linux and Windows
/// (ADR-0248), the GUID is used as well. A part of the record path that is no such name is
/// escaped. The plan depends on the whole DataForge only, so a Record always gets the same file
/// whatever is selected.
pub(crate) struct RecordFiles {
    files: Box<[Option<String>]>,
    renames: Box<[(u32, RenameCause)]>,
}

struct Planned {
    file: String,
    shared: bool,
}

/// What the plan reads of one Record: its escaped record path and its name.
struct Key<'a> {
    path: Option<Cow<'a, str>>,
    name: Option<&'a str>,
}

impl<'a> Key<'a> {
    /// Reads the key of `record`; a record path that cannot be escaped is replaced by the folder
    /// `%long`, and every change a rule forced is added to `renames`.
    fn of(record: &Record<'a>, renames: &mut Vec<(u32, RenameCause)>) -> Self {
        let path = record
            .record_path()
            .ok()
            .map(|raw| match escape_record_path(raw) {
                Ok((path, broken)) => {
                    renames.extend(broken.map(|rule| (record.index(), RenameCause::Path(rule))));
                    path
                }
                Err(rule) => {
                    renames.push((record.index(), RenameCause::Path(rule)));
                    Cow::Owned(format!("{OVERLONG_FOLDER}/{}", record.guid()))
                }
            });
        Self {
            path,
            name: record.name().ok(),
        }
    }
}

impl RecordFiles {
    /// Plans the file of every Record of `forge`, ending in `extension`, e.g. `.xml`.
    pub(crate) fn plan(forge: &DataForge<'_>, extension: &str) -> Self {
        let records: Vec<Record<'_>> = forge.records().collect();
        let mut renames = Vec::new();
        let keys: Vec<Key<'_>> = records
            .iter()
            .map(|record| Key::of(record, &mut renames))
            .collect();
        let path = |index: &u32| {
            keys.get(*index as usize)
                .and_then(|key| key.path.as_deref())
        };
        let name = |index: &u32| keys.get(*index as usize).and_then(|key| key.name);
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
                        file: format!("{record_path}{extension}"),
                        shared: false,
                    }
                } else {
                    if let Some(rule) = name(index).and_then(|name| check_part(name).err()) {
                        renames.push((*index, RenameCause::Name(rule)));
                    }
                    let collides = |other: Option<&u32>| {
                        other.is_some_and(|other| compare(name(index), name(other)).is_eq())
                    };
                    let unique = !collides(position.checked_sub(1).and_then(|at| group.get(at)))
                        && !collides(group.get(position + 1));
                    Planned {
                        file: shared_file(record_path, name(index), record, unique, extension),
                        shared: true,
                    }
                });
            }
        }
        separate_shared(&mut planned, &records, &keys, extension);
        finish(&mut planned, &records, extension, &mut renames);
        renames.sort_by_key(|(index, _)| *index);
        Self {
            files: planned
                .into_iter()
                .map(|planned| planned.map(|planned| planned.file))
                .collect(),
            renames: renames.into(),
        }
    }

    /// Returns why the file of `record` does not follow its name or record path, if it does not.
    ///
    /// `record` must come from the DataForge the plan was made for.
    pub(crate) fn renames(&self, record: &Record<'_>) -> impl Iterator<Item = RenameCause> {
        let index = record.index();
        let start = self.renames.partition_point(|(at, _)| *at < index);
        self.renames
            .get(start..)
            .unwrap_or_default()
            .iter()
            .take_while(move |(at, _)| *at == index)
            .map(|(_, cause)| *cause)
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

/// Moves every Record whose file name is still too long into `%long/<GUID>`, and spells the
/// folders of all files alike.
fn finish(
    planned: &mut [Option<Planned>],
    records: &[Record<'_>],
    extension: &str,
    renames: &mut Vec<(u32, RenameCause)>,
) {
    let mut folders = Folders::default();
    for (index, planned) in planned.iter_mut().enumerate() {
        let (Some(planned), Some(record)) = (planned, records.get(index)) else {
            continue;
        };
        if let Some(rule) = last_part(&planned.file).and_then(|part| check_part(part).err()) {
            planned.file = format!("{OVERLONG_FOLDER}/{}{extension}", record.guid());
            renames.push((record.index(), RenameCause::Path(rule)));
        }
        folders.canonicalise(&mut planned.file);
    }
}

/// Gives every shared Record whose file collides with another planned file ignoring case its
/// GUID, so a Record alone at its record path keeps `<record path><extension>`.
fn separate_shared(
    planned: &mut [Option<Planned>],
    records: &[Record<'_>],
    keys: &[Key<'_>],
    extension: &str,
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
        let (Some(Some(slot)), Some(record), Some(key)) =
            (planned.get_mut(index), records.get(index), keys.get(index))
        else {
            continue;
        };
        if slot.shared
            && let Some(record_path) = key.path.as_deref()
        {
            slot.file = shared_file(record_path, key.name, record, false, extension);
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

fn shared_file(
    record_path: &str,
    name: Option<&str>,
    record: &Record<'_>,
    unique: bool,
    extension: &str,
) -> String {
    let guid = record.guid();
    match name {
        Some(name) if check_part(name).is_ok() && unique => {
            format!("{record_path}/{name}{extension}")
        }
        Some(name) if check_part(name).is_ok() => {
            format!("{record_path}/{name}.{guid}{extension}")
        }
        _ => format!("{record_path}/{guid}{extension}"),
    }
}

fn last_part(file: &str) -> Option<&str> {
    file.rsplit('/').next()
}

fn compare(left: Option<&str>, right: Option<&str>) -> Ordering {
    fold(left.unwrap_or_default()).cmp(fold(right.unwrap_or_default()))
}
