// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_extraction::{Archive, PathKeys};
use p4kx_model::{
    ChildContainer, IncludedObjects, MaterialStatus, ObjectContainer, ObjectContainerError,
    Transform,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::files::{
    FileFailure, child_candidates, file_name, find_first, is_plain, path_key, read_file, stem,
};
use super::{
    InteriorOptions, InteriorPart, InteriorPartKind, MAX_INTERIOR_CONTAINERS, MAX_INTERIOR_DEPTH,
};
use crate::{AssemblyError, Issue, IssueKind};

/// The Archives an Interior is read from: the ones the caller opened, outermost first, and the
/// containers nested in them that the walk opened.
pub(super) struct Layers<'a> {
    given: &'a [&'a Archive],
    opened: Vec<Archive>,
    keys: Vec<PathKeys>,
}

impl<'a> Layers<'a> {
    fn new(given: &'a [&'a Archive]) -> Self {
        Self {
            given,
            opened: Vec::new(),
            keys: given.iter().map(|archive| PathKeys::new(archive)).collect(),
        }
    }

    pub(super) fn get(&self, layer: usize) -> Option<&Archive> {
        match layer.checked_sub(self.given.len()) {
            None => self.given.get(layer).copied(),
            Some(at) => self.opened.get(at),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.given.len() + self.opened.len()
    }

    fn push(&mut self, archive: Archive) -> usize {
        self.keys.push(PathKeys::new(&archive));
        self.opened.push(archive);
        self.len() - 1
    }
}

/// A model part waiting for its model: where to look for the file, outermost layer first.
pub(super) struct Placement {
    pub(super) part: usize,
    pub(super) chain: Arc<[usize]>,
    pub(super) container: Arc<str>,
}

/// What the walk found, before any model is read.
pub(super) struct Walked<'a> {
    pub(super) parts: Vec<InteriorPart>,
    pub(super) placements: Vec<Placement>,
    pub(super) issues: Vec<Issue>,
    pub(super) layers: Layers<'a>,
    pub(super) containers: usize,
}

struct Todo {
    layer: usize,
    chain: Arc<[usize]>,
    part: usize,
    path: Arc<str>,
    stem: String,
    ids: Vec<String>,
    depth: usize,
}

struct StreamScan {
    name: String,
    objects: Option<IncludedObjects>,
}

struct Scan {
    children: Vec<ChildContainer>,
    streams: Vec<StreamScan>,
    issues: Vec<IssueKind>,
    fatal: Option<AssemblyError>,
}

fn identity(layer: usize, path: &str) -> String {
    format!("{layer}|{}", path_key(path))
}

fn container_part(
    name: String,
    parent: Option<usize>,
    local: Transform,
    world: Transform,
) -> InteriorPart {
    InteriorPart {
        name,
        kind: InteriorPartKind::Container,
        parent,
        local,
        world,
        model_path: None,
        model: None,
        materials: Arc::new(MaterialStatus::Unnamed),
    }
}

/// Walks the containers of the Interior: the root container `layers.last()` that was opened from
/// the Entry `entry` of the layer below it, and every container nested in it.
pub(super) fn walk<'a>(
    given: &'a [&'a Archive],
    entry: &str,
    options: &InteriorOptions,
) -> Result<Walked<'a>, AssemblyError> {
    let mut walker = Walker {
        layers: Layers::new(given),
        parts: Vec::new(),
        placements: Vec::new(),
        issues: Vec::new(),
        containers: 0,
        full: false,
        max_parts: options.max_parts,
    };
    let Some(root_layer) = given.len().checked_sub(1) else {
        return Ok(walker.finish());
    };
    let name = stem(entry).to_owned();
    walker.parts.push(container_part(
        name.clone(),
        None,
        Transform::IDENTITY,
        Transform::IDENTITY,
    ));
    let below = root_layer.checked_sub(1).unwrap_or(usize::MAX);
    let mut stack = vec![Todo {
        layer: root_layer,
        chain: (0..=root_layer).collect(),
        part: 0,
        path: name.as_str().into(),
        stem: name,
        ids: vec![identity(below, entry)],
        depth: 0,
    }];
    while let Some(todo) = stack.pop() {
        walker.containers += 1;
        let scan = walker.scan(&todo);
        if let Some(error) = scan.fatal {
            return Err(error);
        }
        walker.report(&todo, scan.issues);
        walker.add_streams(&todo, scan.streams);
        let mut nested = walker.add_children(&todo, scan.children)?;
        nested.reverse();
        stack.extend(nested);
    }
    Ok(walker.finish())
}

struct Walker<'a> {
    layers: Layers<'a>,
    parts: Vec<InteriorPart>,
    placements: Vec<Placement>,
    issues: Vec<Issue>,
    containers: usize,
    full: bool,
    max_parts: usize,
}

impl<'a> Walker<'a> {
    fn finish(self) -> Walked<'a> {
        Walked {
            parts: self.parts,
            placements: self.placements,
            issues: self.issues,
            layers: self.layers,
            containers: self.containers,
        }
    }

    fn note(&mut self, path: &str, name: &str, kind: IssueKind) {
        self.issues.push(Issue {
            path: path.to_owned(),
            name: name.to_owned(),
            kind,
        });
    }

    fn report(&mut self, todo: &Todo, issues: Vec<IssueKind>) {
        for kind in issues {
            self.note(&todo.path, &todo.stem, kind);
        }
    }

    fn room(&mut self, todo: &Todo) -> bool {
        if self.parts.len() >= self.max_parts {
            if !self.full {
                self.full = true;
                let limit = self.max_parts;
                self.note(&todo.path, &todo.stem, IssueKind::TooManyParts { limit });
            }
            return false;
        }
        true
    }

    fn scan(&self, todo: &Todo) -> Scan {
        let (Some(archive), Some(keys)) = (
            self.layers.get(todo.layer),
            self.layers.keys.get(todo.layer),
        ) else {
            return Scan {
                children: Vec::new(),
                streams: Vec::new(),
                issues: Vec::new(),
                fatal: None,
            };
        };
        scan_container(archive, keys, &todo.stem)
    }

    fn add_streams(&mut self, todo: &Todo, streams: Vec<StreamScan>) {
        for stream in streams {
            if !self.room(todo) {
                return;
            }
            let Some(parent) = self.parts.get(todo.part) else {
                return;
            };
            let world = parent.world;
            let at = self.parts.len();
            let mut part = container_part(
                file_name(&stream.name).to_owned(),
                Some(todo.part),
                Transform::IDENTITY,
                world,
            );
            part.kind = InteriorPartKind::Stream;
            self.parts.push(part);
            let Some(objects) = stream.objects else {
                continue;
            };
            if objects.invalid > 0 {
                let count = objects.invalid;
                self.note(
                    &todo.path,
                    file_name(&stream.name),
                    IssueKind::ObjectsLeftOut { count },
                );
            }
            if let Some(kind) = objects.unsupported {
                self.note(
                    &todo.path,
                    file_name(&stream.name),
                    IssueKind::UnsupportedObjects { kind },
                );
            }
            for object in &objects.objects {
                if !self.room(todo) {
                    return;
                }
                let Some(path) = objects.models.get(object.model) else {
                    continue;
                };
                self.placements.push(Placement {
                    part: self.parts.len(),
                    chain: Arc::clone(&todo.chain),
                    container: Arc::clone(&todo.path),
                });
                self.parts.push(InteriorPart {
                    name: stem(path).to_owned(),
                    kind: InteriorPartKind::Model,
                    parent: Some(at),
                    local: object.transform,
                    world: world.then(&object.transform),
                    model_path: Some(path.clone()),
                    model: None,
                    materials: Arc::new(MaterialStatus::Unnamed),
                });
            }
        }
    }

    fn add_children(
        &mut self,
        todo: &Todo,
        children: Vec<ChildContainer>,
    ) -> Result<Vec<Todo>, AssemblyError> {
        let mut nested = Vec::new();
        for child in children {
            let name = if child.name.is_empty() {
                child.entity_name.clone()
            } else {
                child.name.clone()
            };
            let Some(local) = child.transform().filter(|_| !child.malformed) else {
                let reason =
                    "pos and rot are no finite numbers or the rotation has no length".to_owned();
                self.note(
                    &todo.path,
                    &name,
                    IssueKind::InvalidPlacement {
                        name: name.clone(),
                        reason,
                    },
                );
                continue;
            };
            if todo.depth + 1 >= MAX_INTERIOR_DEPTH {
                self.note(
                    &todo.path,
                    &name,
                    IssueKind::TooDeep {
                        limit: MAX_INTERIOR_DEPTH,
                    },
                );
                continue;
            }
            if self.containers + nested.len() >= MAX_INTERIOR_CONTAINERS {
                let reason = format!("more than {MAX_INTERIOR_CONTAINERS} containers");
                self.note(
                    &todo.path,
                    &name,
                    IssueKind::ContainerUnreadable {
                        name: name.clone(),
                        reason,
                    },
                );
                continue;
            }
            if !self.room(todo) {
                break;
            }
            if let Some(next) = self.enter(todo, &child, &name, local)? {
                nested.push(next);
            }
        }
        Ok(nested)
    }

    fn enter(
        &mut self,
        todo: &Todo,
        child: &ChildContainer,
        name: &str,
        local: Transform,
    ) -> Result<Option<Todo>, AssemblyError> {
        let found = if is_plain(&child.name) {
            todo.chain.iter().rev().find_map(|&layer| {
                let archive = self.layers.get(layer)?;
                let keys = self.layers.keys.get(layer)?;
                let candidates = child_candidates(&child.name, layer == 0);
                find_first(&keys.index(archive), &candidates).map(|entry| (layer, entry))
            })
        } else {
            None
        };
        let Some((layer, entry)) = found else {
            let kind = IssueKind::ContainerMissing {
                name: child.name.clone(),
            };
            self.note(&todo.path, name, kind);
            return Ok(None);
        };
        let entry_path = entry.path().to_owned();
        let id = identity(layer, &entry_path);
        if todo.ids.contains(&id) {
            self.note(
                &todo.path,
                name,
                IssueKind::ContainerCycle {
                    name: child.name.clone(),
                },
            );
            return Ok(None);
        }
        let opened = self
            .layers
            .get(layer)
            .map(|archive| archive.open_container(entry));
        let archive = match opened {
            Some(Ok(archive)) => archive,
            Some(Err(error)) if error.is_key_required() || error.is_wrong_key() => {
                return Err(AssemblyError::Provider(Box::new(error)));
            }
            Some(Err(error)) => {
                let kind = IssueKind::ContainerUnreadable {
                    name: child.name.clone(),
                    reason: error.to_string(),
                };
                self.note(&todo.path, name, kind);
                return Ok(None);
            }
            None => return Ok(None),
        };
        let layer = self.layers.push(archive);
        let world = self
            .parts
            .get(todo.part)
            .map_or(local, |parent| parent.world.then(&local));
        let part = self.parts.len();
        self.parts.push(container_part(
            stem(name).to_owned(),
            Some(todo.part),
            local,
            world,
        ));
        let mut chain = todo.chain.to_vec();
        chain.push(layer);
        let mut ids = todo.ids.clone();
        ids.push(id);
        Ok(Some(Todo {
            layer,
            chain: chain.into(),
            part,
            path: format!("{}/{}", todo.path, stem(name)).into(),
            stem: stem(&entry_path).to_owned(),
            ids,
            depth: todo.depth + 1,
        }))
    }
}

fn scan_container(archive: &Archive, keys: &PathKeys, stem: &str) -> Scan {
    let index = keys.index(archive);
    let mut scan = Scan {
        children: Vec::new(),
        streams: Vec::new(),
        issues: Vec::new(),
        fatal: None,
    };
    let candidates = [format!("{stem}.xml"), format!("{stem}\\{stem}.xml")];
    let mut names: Vec<String> = Vec::new();
    match find_first(&index, &candidates) {
        None => scan.issues.push(IssueKind::FileMissing {
            file: candidates[0].clone(),
        }),
        Some(entry) => match read_file(archive, entry) {
            Err(FileFailure::Fatal(error)) => scan.fatal = Some(AssemblyError::Provider(error)),
            Err(failure) => scan.issues.push(IssueKind::FileUnreadable {
                file: entry.path().to_owned(),
                reason: failure.reason(),
            }),
            Ok(bytes) => match ObjectContainer::parse(&bytes) {
                Ok(container) => {
                    names = container.streams;
                    scan.children = container.children;
                }
                Err(error) => scan.issues.push(IssueKind::FileUnreadable {
                    file: entry.path().to_owned(),
                    reason: describe(&error),
                }),
            },
        },
    }
    if names.is_empty() && scan.children.is_empty() && index.find(&format!("{stem}.soc")).is_some()
    {
        names.push(format!("{stem}.soc"));
    }
    let mut seen = std::collections::HashSet::new();
    names.retain(|name| seen.insert(path_key(name)));
    let read: Vec<(StreamScan, Vec<IssueKind>, Option<AssemblyError>)> = names
        .par_iter()
        .map(|name| read_stream(archive, &index, name))
        .collect();
    for (stream, issues, fatal) in read {
        scan.streams.push(stream);
        scan.issues.extend(issues);
        scan.fatal = scan.fatal.take().or(fatal);
    }
    scan
}

fn read_stream(
    archive: &Archive,
    index: &p4kx_extraction::PathIndex<'_>,
    name: &str,
) -> (StreamScan, Vec<IssueKind>, Option<AssemblyError>) {
    let mut stream = StreamScan {
        name: name.to_owned(),
        objects: None,
    };
    let Some(entry) = is_plain(name).then(|| index.find(name)).flatten() else {
        let file = name.to_owned();
        return (stream, vec![IssueKind::FileMissing { file }], None);
    };
    match read_file(archive, entry) {
        Err(FileFailure::Fatal(error)) => {
            (stream, Vec::new(), Some(AssemblyError::Provider(error)))
        }
        Err(failure) => {
            let issue = IssueKind::FileUnreadable {
                file: entry.path().to_owned(),
                reason: failure.reason(),
            };
            (stream, vec![issue], None)
        }
        Ok(bytes) => match IncludedObjects::read(&bytes) {
            Ok(objects) => {
                stream.objects = objects;
                (stream, Vec::new(), None)
            }
            Err(error) => {
                let issue = IssueKind::FileUnreadable {
                    file: entry.path().to_owned(),
                    reason: error.to_string(),
                };
                (stream, vec![issue], None)
            }
        },
    }
}

fn describe(error: &ObjectContainerError) -> String {
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(error);
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}
