// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod budget;

use p4kx_extraction::{Archive, PathIndex};
use p4kx_gltf::{GltfError, Scene, SceneNode};
use p4kx_model::{Model, Transform};

use crate::placement::{node_transform, offset};
use crate::{
    Assembly, AssemblyError, EntityNode, Interior, InteriorForge, InteriorOptions,
    InteriorProgress, InteriorWatch, Issue, IssueKind, ModelProvider, ObjectContainerRef,
};
use budget::Budget;

/// The name of the scene node that groups the Interior below the Entity.
pub const INTERIOR_NODE: &str = "Interior";

/// One container of an Entity's Interior: the reference, where it sits and what it holds.
#[derive(Debug, Clone)]
pub struct EntityContainer {
    pub(crate) reference: ObjectContainerRef,
    pub(crate) placement: Transform,
    pub(crate) interior: Interior,
}

impl EntityContainer {
    /// Returns the object container the Entity's Record names.
    #[must_use]
    pub const fn reference(&self) -> &ObjectContainerRef {
        &self.reference
    }

    /// Returns the transform from the container's space to the Entity's: the bone of the
    /// Entity's model moved by the Record's offset, or the bone alone when it already carries
    /// the offset.
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

/// The rooms of an Entity: its object containers, each placed relative to the Entity.
///
/// Built by [`EntityInterior::build`] and put into an [`Assembly`] with
/// [`Assembly::with_interior`], which then writes it below an [`INTERIOR_NODE`] node of the
/// root Entity.
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

    /// Builds the Interior of the Entity `tree` of `assembly`, whose root part carries the model
    /// the containers' bones are looked up in: each object container the Entity's Record names
    /// is looked up in `archive`, built like [`Interior::build`] and placed at its bone of the
    /// root model, moved by the Record's offset.
    ///
    /// A container that cannot be found or opened, a bone the root model lacks (the container
    /// then sits at the Entity's origin) and everything an Interior cannot resolve are listed in
    /// [`Self::issues`]. The limits of `options` hold for the Entity as a whole: the containers,
    /// the parts and the vertices of every container are counted together, and the vertices of
    /// the hull too; when one is used up, the remaining containers are left out with
    /// `too_many_containers`, `too_many_parts` or `too_many_vertices`.
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Provider`] when a file cannot be read for every container, e.g.
    /// because an Archive key is missing.
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
        Self::build_with_models(archive, assembly, tree, options, watch, (forge, None))
    }

    /// Builds the Interior like [`Self::build_with`], reading the models of `archive` through
    /// `models` when given, e.g. a [`CachedModels`](crate::CachedModels) over the provider the
    /// Assembly was built with, so that models read before are not read again, see
    /// [`Interior::build_with_models`].
    ///
    /// # Errors
    ///
    /// As [`Self::build_watched`].
    pub fn build_with_models(
        archive: &Archive,
        assembly: &Assembly,
        tree: &EntityNode,
        options: &InteriorOptions,
        watch: &InteriorWatch<'_>,
        (forge, models): (Option<&InteriorForge<'_>>, Option<&dyn ModelProvider>),
    ) -> Result<Self, AssemblyError> {
        let index = PathIndex::new(archive);
        let root = assembly.root().and_then(|part| part.model());
        let mut built = Self::default();
        let budget = Budget::new(assembly, options);
        let total = tree.object_containers().len();
        for (done, reference) in tree.object_containers().iter().enumerate() {
            if watch.is_cancelled() {
                return Err(AssemblyError::Cancelled);
            }
            watch.report(InteriorProgress::Container { done, total });
            if let Some(kind) = budget.exhausted(&built) {
                built.note(reference, kind);
                break;
            }
            let Some(entry) = find(&index, reference.file()) else {
                built.note(
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
                        reference,
                        IssueKind::ContainerUnreadable {
                            name: reference.file().to_owned(),
                            reason: error.to_string(),
                        },
                    );
                    continue;
                }
            };
            let placement = built.place(reference, root);
            let limited = budget.options(&built);
            let interior = Interior::build_with_models(
                &[archive, &opened],
                entry.path(),
                &limited,
                watch,
                (forge, models),
            )?;
            built.add(reference, placement, interior);
        }
        Ok(built)
    }

    fn place(&mut self, reference: &ObjectContainerRef, root: Option<&Model>) -> Transform {
        let finite = reference
            .position()
            .iter()
            .chain(&reference.rotation())
            .all(|value| value.is_finite());
        let moved = if finite {
            offset(reference.position(), reference.rotation())
        } else {
            self.note(
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
        let Some(anchor) = root.and_then(|model| node_transform(model, bone)) else {
            self.note(
                reference,
                IssueKind::HelperNotFound {
                    helper: bone.to_owned(),
                },
            );
            return moved;
        };
        let at = anchor.translation();
        if near(at, [0.0; 3]) {
            return anchor.then(&moved);
        }
        if finite && !near(reference.position(), [0.0; 3]) && !near(reference.position(), at) {
            self.note(
                reference,
                IssueKind::OffsetDiffers {
                    bone: bone.to_owned(),
                },
            );
        }
        anchor
    }

    fn add(&mut self, reference: &ObjectContainerRef, placement: Transform, interior: Interior) {
        let name = interior.root().map_or("", |part| part.name()).to_owned();
        self.issues
            .extend(interior.issues().iter().map(|issue| Issue {
                path: if issue.path.is_empty() {
                    name.clone()
                } else {
                    format!("{name}/{}", issue.path)
                },
                name: issue.name.clone(),
                kind: issue.kind.clone(),
            }));
        self.containers.push(EntityContainer {
            reference: reference.clone(),
            placement,
            interior,
        });
    }

    fn note(&mut self, reference: &ObjectContainerRef, kind: IssueKind) {
        self.issues.push(Issue {
            path: INTERIOR_NODE.to_owned(),
            name: reference.file().to_owned(),
            kind,
        });
    }

    pub(crate) fn add_to_scene<'s>(
        &'s self,
        scene: &mut Scene<'s>,
        root: usize,
    ) -> Result<(), GltfError> {
        if self.containers.is_empty() {
            return Ok(());
        }
        let group = scene.push(SceneNode::new(
            INTERIOR_NODE,
            Some(root),
            Transform::IDENTITY,
        ))?;
        for container in &self.containers {
            container
                .interior
                .add_to_scene(scene, Some(group), container.placement)?;
        }
        Ok(())
    }
}

/// The largest difference per axis, in metres, between two translations that still counts as
/// the same place.
const OFFSET_TOLERANCE: f32 = 0.05;

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y).abs() <= OFFSET_TOLERANCE)
}

fn find<'a>(index: &PathIndex<'a>, file: &str) -> Option<&'a p4kx_extraction::Entry> {
    let path = file.trim_start_matches(['/', '\\']);
    [format!("Data\\{path}"), path.to_owned()]
        .iter()
        .find_map(|candidate| index.find(candidate))
}

impl Assembly {
    /// Returns the Interior of the Entity, or `None` when none was added.
    #[must_use]
    pub const fn interior(&self) -> Option<&EntityInterior> {
        self.interior.as_ref()
    }

    /// Returns the Assembly with `interior` added: its parts are written below an
    /// [`INTERIOR_NODE`] node of the root, and its issues are appended to [`Self::issues`]. A
    /// second call replaces the first Interior and its issues.
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
mod tests {
    use super::*;
    use crate::ContainerSource;

    #[test]
    fn a_non_finite_offset_is_the_identity_and_an_issue() {
        let reference = ObjectContainerRef {
            file: "a.socpak".to_owned(),
            bone: String::new(),
            position: [f32::NAN, 0.0, 0.0],
            rotation: [0.0; 3],
            source: ContainerSource::Vehicle,
        };
        let mut built = EntityInterior::default();

        let placed = built.place(&reference, None);

        assert!(placed.is_identity());
        assert_eq!(built.issues.len(), 1);
        assert_eq!(built.issues[0].kind.code(), "invalid_offset");
    }
}
