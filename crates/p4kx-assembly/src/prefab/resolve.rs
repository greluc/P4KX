// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use p4kx_model::Transform;

use super::set::Lookup;
use super::{
    MAX_PREFAB_DEPTH, MAX_PREFAB_ISSUES, MAX_PREFAB_OBJECTS, MAX_PREFAB_VISITS, PrefabError,
    PrefabFile, PrefabIssue, PrefabIssueKind, PrefabSet, ResolvedObject, ResolvedPrefab,
};

impl PrefabSet<'_> {
    /// Resolves the prefab `prefab` of the library `library`: its objects with their transforms
    /// to the prefab's space, every nested prefab expanded in place of the object that instances
    /// it with the object's transform composed with those of its objects.
    ///
    /// A prefab that contains itself, that is nested deeper than [`MAX_PREFAB_DEPTH`] or that no
    /// library holds is not expanded: its object stays in the list and an issue says why.
    ///
    /// # Errors
    ///
    /// Returns [`PrefabError`] when the library or the prefab does not exist, the library cannot
    /// be read or the Archive needs a key.
    pub fn resolve(&mut self, library: &str, prefab: &str) -> Result<ResolvedPrefab, PrefabError> {
        let file = self.library(library)?;
        let Some(index) = file.find(prefab) else {
            return Err(PrefabError::NoSuchPrefab {
                library: library.to_owned(),
                name: prefab.to_owned(),
            });
        };
        let mut walk = Walk {
            set: self,
            objects: Vec::new(),
            issues: Vec::new(),
            visits: 0,
            prefabs: 0,
            stack: Vec::new(),
            reported: HashSet::new(),
            mismatched: HashSet::new(),
            stopped: false,
        };
        let root: Arc<str> = Arc::from("");
        walk.expand(&file, index, Transform::IDENTITY, &root)?;
        let name = file
            .library()
            .prefabs
            .get(index)
            .map_or_else(|| prefab.to_owned(), |found| found.name.clone());
        Ok(ResolvedPrefab {
            file: walk.set.shown(file.slot()),
            library: file.library().name.clone(),
            name,
            objects: walk.objects,
            prefabs: walk.prefabs,
            issues: walk.issues,
        })
    }
}

struct Walk<'s, 'a> {
    set: &'s mut PrefabSet<'a>,
    objects: Vec<ResolvedObject>,
    issues: Vec<PrefabIssue>,
    visits: usize,
    prefabs: usize,
    stack: Vec<(usize, usize)>,
    reported: HashSet<(usize, usize)>,
    mismatched: HashSet<(usize, usize, String)>,
    stopped: bool,
}

impl Walk<'_, '_> {
    fn note(&mut self, container: &str, name: &str, kind: PrefabIssueKind) {
        if self.issues.len() > MAX_PREFAB_ISSUES {
            return;
        }
        let kind = if self.issues.len() == MAX_PREFAB_ISSUES {
            PrefabIssueKind::TooManyIssues {
                limit: MAX_PREFAB_ISSUES,
            }
        } else {
            kind
        };
        self.issues.push(PrefabIssue {
            container: container.to_owned(),
            name: name.to_owned(),
            kind,
        });
    }

    fn report_prefab(&mut self, file: &PrefabFile, index: usize, container: &str) {
        let Some(prefab) = file.library().prefabs.get(index) else {
            return;
        };
        if !self.reported.insert((file.slot(), index)) {
            return;
        }
        if prefab.invalid > 0 {
            let kind = PrefabIssueKind::ObjectsLeftOut {
                prefab: prefab.name.clone(),
                count: prefab.invalid,
            };
            self.note(container, "", kind);
        }
        let attached = prefab
            .objects
            .iter()
            .filter(|object| object.attached)
            .count();
        if attached > 0 {
            let kind = PrefabIssueKind::Attached {
                prefab: prefab.name.clone(),
                count: attached,
            };
            self.note(container, "", kind);
        }
    }

    fn expand(
        &mut self,
        file: &Arc<PrefabFile>,
        index: usize,
        world: Transform,
        container: &Arc<str>,
    ) -> Result<(), PrefabError> {
        let Some(prefab) = file.library().prefabs.get(index) else {
            return Ok(());
        };
        self.prefabs += 1;
        self.report_prefab(file, index, container);
        self.stack.push((file.slot(), index));
        for (at, object) in prefab.objects.iter().enumerate() {
            if self.stopped {
                break;
            }
            self.visits += 1;
            if self.visits > MAX_PREFAB_VISITS {
                self.stopped = true;
                let kind = PrefabIssueKind::TooManyVisits {
                    limit: MAX_PREFAB_VISITS,
                };
                self.note(container, "", kind);
                break;
            }
            let placed = world.then(&object.transform);
            let nested = match &object.nested {
                Some(reference) => self.nested(container, &object.name, reference)?,
                None => None,
            };
            match nested {
                Some((inner, target)) => {
                    let path: Arc<str> = if container.is_empty() {
                        Arc::from(object.name.as_str())
                    } else {
                        Arc::from(format!("{container}/{}", object.name))
                    };
                    self.expand(&inner, target, placed, &path)?;
                }
                None => self.push(file, index, at, placed, container),
            }
        }
        self.stack.pop();
        Ok(())
    }

    fn push(
        &mut self,
        file: &Arc<PrefabFile>,
        prefab: usize,
        object: usize,
        world: Transform,
        container: &Arc<str>,
    ) {
        if self.objects.len() >= MAX_PREFAB_OBJECTS {
            if !self.stopped {
                self.stopped = true;
                let kind = PrefabIssueKind::TooManyObjects {
                    limit: MAX_PREFAB_OBJECTS,
                };
                self.note(container, "", kind);
            }
            return;
        }
        self.objects.push(ResolvedObject {
            file: Arc::clone(file),
            prefab,
            object,
            world,
            container: Arc::clone(container),
            depth: self.stack.len().saturating_sub(1),
        });
    }

    fn nested(
        &mut self,
        container: &str,
        name: &str,
        reference: &p4kx_model::PrefabRef,
    ) -> Result<Option<(Arc<PrefabFile>, usize)>, PrefabError> {
        let (file, target) = match self.set.find_nested(reference)? {
            Lookup::Found(file, target) => (file, target),
            Lookup::Missing(unreadable) => {
                if let Some((library, reason)) = unreadable {
                    let kind = PrefabIssueKind::LibraryUnreadable { library, reason };
                    self.note(container, name, kind);
                }
                let kind = PrefabIssueKind::Missing {
                    prefab: reference.name.clone(),
                };
                self.note(container, name, kind);
                return Ok(None);
            }
        };
        if self.stack.contains(&(file.slot(), target)) {
            let kind = PrefabIssueKind::Cycle {
                prefab: reference.name.clone(),
            };
            self.note(container, name, kind);
            return Ok(None);
        }
        if self.stack.len() >= MAX_PREFAB_DEPTH {
            let kind = PrefabIssueKind::TooDeep {
                limit: MAX_PREFAB_DEPTH,
            };
            self.note(container, name, kind);
            return Ok(None);
        }
        if let Some(found) = file.library().prefabs.get(target)
            && !reference.guid.is_empty()
            && !found.id.is_empty()
            && !found.id.eq_ignore_ascii_case(&reference.guid)
            && self
                .mismatched
                .insert((file.slot(), target, reference.guid.to_ascii_lowercase()))
        {
            let kind = PrefabIssueKind::GuidMismatch {
                prefab: reference.name.clone(),
                named: reference.guid.clone(),
                found: found.id.clone(),
            };
            self.note(container, name, kind);
        }
        Ok(Some((file, target)))
    }
}
