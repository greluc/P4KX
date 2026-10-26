// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod budget;
mod lookup;

use p4kx_extraction::{Archive, PathIndex};
use p4kx_gltf::{GltfError, Scene, SceneNode};
use p4kx_model::{Model, Transform};

use crate::placement::{node_transform, offset};
use crate::{
    Assembly, AssemblyError, EntityNode, Interior, InteriorForge, InteriorOptions,
    InteriorProgress, InteriorWatch, Issue, IssueKind, ObjectContainerRef,
};
use budget::Budget;
use lookup::{check_tree, find, near, node_path};

/// The name of the scene node that groups the Interior below the Entity.
pub const INTERIOR_NODE: &str = "Interior";

/// One container of an Entity's Interior: the reference, where it sits and what it holds.
#[derive(Debug, Clone)]
pub struct EntityContainer {
    pub(crate) part: usize,
    pub(crate) reference: ObjectContainerRef,
    pub(crate) placement: Transform,
    pub(crate) interior: Interior,
}

impl EntityContainer {
    /// Returns the index in [`Assembly::parts`] of the Entity that names the container: 0 for the
    /// root Entity, otherwise an attached one. The container sits relative to that part.
    #[must_use]
    pub const fn part(&self) -> usize {
        self.part
    }

    /// Returns the object container the Entity's Record names.
    #[must_use]
    pub const fn reference(&self) -> &ObjectContainerRef {
        &self.reference
    }

    /// Returns the transform from the container's space to that of its Entity, the part
    /// [`Self::part`]: the bone of the Entity's model moved by the Record's offset, or the bone
    /// alone when it already carries the offset.
    #[must_use]
    pub const fn placement(&self) -> &Transform {
        &self.placement
    }

    /// Returns the container's Interior.
    #[must_use]
    pub const fn interior(&self) -> &Interior {
        &self.interior
    }
}

/// The rooms of an Assembly: the object containers of the root Entity and of the attached
/// Entities, each placed relative to the Entity that names it.
///
/// Built by [`EntityInterior::build`] and put into an [`Assembly`] with
/// [`Assembly::with_interior`], which then writes it below an [`INTERIOR_NODE`] node of each
/// Entity that has containers.
#[derive(Debug, Clone, Default)]
pub struct EntityInterior {
    pub(crate) containers: Vec<EntityContainer>,
    pub(crate) issues: Vec<Issue>,
}

impl EntityInterior {
    /// Returns the containers that could be opened, in the order of the Record.
    #[must_use]
    pub fn containers(&self) -> &[EntityContainer] {
        &self.containers
    }

    /// Returns what could not be resolved, the Entity's containers first, then those of each
    /// container's Interior; the paths of the latter start with the container's name.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    fn total(&self, count: fn(&Interior) -> usize) -> usize {
        self.containers.iter().map(|c| count(&c.interior)).sum()
    }

    /// Returns the number of parts of all containers.
    #[must_use]
    pub fn parts(&self) -> usize {
        self.total(|interior| interior.parts().len())
    }

    /// Returns the number of lights of all containers.
    #[must_use]
    pub fn lights(&self) -> usize {
        self.total(Interior::lights)
    }

    /// Returns the number of model parts of all containers, props included, readable or not.
    #[must_use]
    pub fn model_parts(&self) -> usize {
        self.total(Interior::model_parts)
    }

    /// Returns the number of parts of all containers that have a model.
    #[must_use]
    pub fn modelled_parts(&self) -> usize {
        self.total(Interior::modelled_parts)
    }

    /// Builds the Interior of `assembly`, which was built from the Entity `tree`: each object
    /// container the Record of the root Entity or of an attached Entity names is looked up in
    /// `archive`, built like [`Interior::build`] and placed at its bone of that Entity's model,
    /// moved by the Record's offset.
    ///
    /// A container that cannot be found or opened, a bone the Entity's model lacks (the container
    /// then sits at the Entity's origin) and everything an Interior cannot resolve are listed in
    /// [`Self::issues`]. The limits of `options` hold for the Assembly as a whole: the
    /// containers, the parts and the vertices of every container are counted together, and the
    /// vertices of the hull too; when one is used up, the remaining containers are left out with
    /// `too_many_containers`, `too_many_parts` or `too_many_vertices`.
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Provider`] when a file cannot be read for every container, e.g.
    /// because the Archive key is missing or wrong, and [`AssemblyError::TreeMismatch`] when
    /// `tree` is not the Entity `assembly` was built from.
    pub fn build(
        archive: &Archive,
        assembly: &Assembly,
        tree: &EntityNode,
        options: &InteriorOptions,
    ) -> Result<Self, AssemblyError> {
        Self::build_watched(archive, assembly, tree, options, &InteriorWatch::none())
    }

    /// Builds the Interior like [`Self::build`], asking `watch` before each container and while
    /// it reads the models whether to stop, and telling it how far it has come.
    ///
    /// # Errors
    ///
    /// As [`Self::build`], and [`AssemblyError::Cancelled`] when `watch` asks to stop.
    pub fn build_watched(
        archive: &Archive,
        assembly: &Assembly,
        tree: &EntityNode,
        options: &InteriorOptions,
        watch: &InteriorWatch<'_>,
    ) -> Result<Self, AssemblyError> {
        Self::build_with(archive, assembly, tree, options, watch, None)
    }

    /// Builds the Interior like [`Self::build_watched`], with the class Records of the
    /// containers' entities looked up in `forge`, see [`Interior::build_with`].
    ///
    /// # Errors
    ///
    /// As [`Self::build_watched`].
    pub fn build_with(
        archive: &Archive,
        assembly: &Assembly,
        tree: &EntityNode,
        options: &InteriorOptions,
        watch: &InteriorWatch<'_>,
        forge: Option<&InteriorForge<'_>>,
    ) -> Result<Self, AssemblyError> {
        check_tree(assembly, tree)?;
        let index = PathIndex::new(archive);
        let mut built = Self::default();
        let budget = Budget::new(assembly, options);
        let total = assembly.containers.iter().map(|list| list.len()).sum();
        let named = assembly
            .containers
            .iter()
            .enumerate()
            .flat_map(|(part, list)| list.iter().map(move |reference| (part, reference)));
        for (done, (part, reference)) in named.enumerate() {
            if watch.is_cancelled() {
                return Err(AssemblyError::Cancelled);
            }
            watch.report(InteriorProgress::Container { done, total });
            let owner = assembly.parts.get(part);
            let at = node_path(owner.map_or("", |owner| owner.path()));
            if let Some(kind) = budget.exhausted(&built) {
                built.note(&at, reference, kind);
                break;
            }
            let Some(entry) = find(&index, reference.file()) else {
                built.note(
                    &at,
                    reference,
                    IssueKind::ContainerMissing {
                        name: reference.file().to_owned(),
                    },
                );
                continue;
            };
            let opened = match archive.open_container(entry) {
                Ok(opened) => opened,
                Err(error) if error.is_key_required() || error.is_wrong_key() => {
                    return Err(AssemblyError::Provider(Box::new(error)));
                }
                Err(error) => {
                    built.note(
                        &at,
                        reference,
                        IssueKind::ContainerUnreadable {
                            name: reference.file().to_owned(),
                            reason: error.to_string(),
                        },
                    );
                    continue;
                }
            };
            let model = owner.and_then(|owner| owner.model());
            let placement = built.place(&at, reference, model);
            let limited = budget.options(&built);
            let interior =
                Interior::build_with(&[archive, &opened], entry.path(), &limited, watch, forge)?;
            let owner_path = owner.map_or("", |owner| owner.path());
            built.add(part, owner_path, reference, placement, interior);
        }
        Ok(built)
    }

    fn place(
        &mut self,
        at: &str,
        reference: &ObjectContainerRef,
        model: Option<&Model>,
    ) -> Transform {
        let finite = reference
            .position()
            .iter()
            .chain(&reference.rotation())
            .all(|value| value.is_finite());
        let moved = if finite {
            offset(reference.position(), reference.rotation())
        } else {
            self.note(
                at,
                reference,
                IssueKind::InvalidOffset {
                    name: reference.file().to_owned(),
                },
            );
            Transform::IDENTITY
        };
        let bone = reference.bone();
        if bone.is_empty() {
            return moved;
        }
        let Some(anchor) = model.and_then(|model| node_transform(model, bone)) else {
            self.note(
                at,
                reference,
                IssueKind::HelperNotFound {
                    helper: bone.to_owned(),
                },
            );
            return moved;
        };
        let anchored = anchor.translation();
        if near(anchored, [0.0; 3]) {
            return anchor.then(&moved);
        }
        if finite && !near(reference.position(), [0.0; 3]) && !near(reference.position(), anchored)
        {
            self.note(
                at,
                reference,
                IssueKind::OffsetDiffers {
                    bone: bone.to_owned(),
                },
            );
        }
        anchor
    }

    fn add(
        &mut self,
        part: usize,
        owner_path: &str,
        reference: &ObjectContainerRef,
        placement: Transform,
        interior: Interior,
    ) {
        let name = interior.root().map_or("", |part| part.name());
        let prefix = if owner_path.is_empty() {
            name.to_owned()
        } else {
            format!("{owner_path}/{name}")
        };
        self.issues
            .extend(interior.issues().iter().map(|issue| Issue {
                path: if issue.path.is_empty() {
                    prefix.clone()
                } else {
                    format!("{prefix}/{}", issue.path)
                },
                name: issue.name.clone(),
                kind: issue.kind.clone(),
            }));
        self.containers.push(EntityContainer {
            part,
            reference: reference.clone(),
            placement,
            interior,
        });
    }

    fn note(&mut self, at: &str, reference: &ObjectContainerRef, kind: IssueKind) {
        self.issues.push(Issue {
            path: at.to_owned(),
            name: reference.file().to_owned(),
            kind,
        });
    }

    pub(crate) fn add_to_scene<'s>(
        &'s self,
        scene: &mut Scene<'s>,
        parts: usize,
    ) -> Result<(), GltfError> {
        let mut group: Option<(usize, usize)> = None;
        for container in &self.containers {
            let node = match group {
                Some((part, node)) if part == container.part => node,
                _ => {
                    let parent = (container.part < parts).then_some(container.part);
                    let node =
                        scene.push(SceneNode::new(INTERIOR_NODE, parent, Transform::IDENTITY))?;
                    group = Some((container.part, node));
                    node
                }
            };
            container
                .interior
                .add_to_scene(scene, Some(node), container.placement)?;
        }
        Ok(())
    }
}

impl Assembly {
    /// Returns the Interior of the Assembly, or `None` when none was added.
    #[must_use]
    pub const fn interior(&self) -> Option<&EntityInterior> {
        self.interior.as_ref()
    }

    /// Returns the Assembly with `interior` added: its parts are written below an
    /// [`INTERIOR_NODE`] node of each Entity that names containers (a root node when the Assembly
    /// has no parts), and its issues are appended to [`Self::issues`]. A second call replaces the
    /// first Interior and its issues.
    #[must_use]
    pub fn with_interior(mut self, interior: EntityInterior) -> Self {
        if let Some(before) = self.interior.take() {
            let kept = self.issues.len().saturating_sub(before.issues.len());
            self.issues.truncate(kept);
        }
        self.issues.extend(interior.issues.iter().cloned());
        self.interior = Some(interior);
        self
    }
}

#[cfg(test)]
mod tests;
