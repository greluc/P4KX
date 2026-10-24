// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Archive, PathIndex};
use p4kx_gltf::{GltfError, Scene, SceneNode};
use p4kx_model::{Model, Transform};

use crate::placement::{node_transform, offset};
use crate::{
    Assembly, AssemblyError, EntityNode, Interior, InteriorOptions, Issue, IssueKind,
    MAX_INTERIOR_CONTAINERS, ObjectContainerRef,
};

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

    /// Returns the number of parts of all containers.
    #[must_use]
    pub fn parts(&self) -> usize {
        self.containers
            .iter()
            .map(|container| container.interior.parts().len())
            .sum()
    }

    /// Returns the number of model parts of all containers, readable or not.
    #[must_use]
    pub fn model_parts(&self) -> usize {
        self.containers
            .iter()
            .map(|container| container.interior.model_parts())
            .sum()
    }

    /// Returns the number of parts of all containers that have a model.
    #[must_use]
    pub fn modelled_parts(&self) -> usize {
        self.containers
            .iter()
            .map(|container| container.interior.modelled_parts())
            .sum()
    }

    /// Builds the Interior of the Entity `tree` of `assembly`, whose root part carries the model
    /// the containers' bones are looked up in: each object container the Entity's Record names
    /// is looked up in `archive`, built like [`Interior::build`] and placed at its bone of the
    /// root model, moved by the Record's offset.
    ///
    /// A container that cannot be found or opened, a bone the root model lacks (the container
    /// then sits at the Entity's origin) and everything an Interior cannot resolve are listed in
    /// [`Self::issues`]. At most [`MAX_INTERIOR_CONTAINERS`] containers are opened.
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
        let index = PathIndex::new(archive);
        let root = assembly.root().and_then(|part| part.model());
        let mut built = Self::default();
        for reference in tree.object_containers() {
            if built.containers.len() >= MAX_INTERIOR_CONTAINERS {
                built.note(
                    reference,
                    IssueKind::TooManyContainers {
                        limit: MAX_INTERIOR_CONTAINERS,
                    },
                );
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
            let interior = Interior::build(&[archive, &opened], entry.path(), options)?;
            built.add(reference, placement, interior);
        }
        Ok(built)
    }

    fn place(&mut self, reference: &ObjectContainerRef, root: Option<&Model>) -> Transform {
        let anchor = match reference.bone() {
            "" => Transform::IDENTITY,
            bone => root
                .and_then(|model| node_transform(model, bone))
                .unwrap_or_else(|| {
                    self.note(
                        reference,
                        IssueKind::HelperNotFound {
                            helper: bone.to_owned(),
                        },
                    );
                    Transform::IDENTITY
                }),
        };
        let moved = offset(reference.position(), reference.rotation());
        if same(&anchor, &moved) {
            anchor
        } else {
            anchor.then(&moved)
        }
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

/// The largest difference of a translation, in metres, and of a rotation entry between two
/// transforms that still counts as the same transform: the bone of the hull and the Record's
/// offset then place the container once.
const SAME_SHIFT: f32 = 0.05;
const SAME_TURN: f32 = 0.01;

fn same(a: &Transform, b: &Transform) -> bool {
    a.rows.iter().zip(&b.rows).all(|(row_a, row_b)| {
        row_a.iter().zip(row_b).enumerate().all(|(column, (x, y))| {
            let limit = if column == 3 { SAME_SHIFT } else { SAME_TURN };
            (x - y).abs() <= limit
        })
    })
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
