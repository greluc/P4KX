use p4kx_model::{MaterialStatus, Model, Transform};

use crate::GltfError;

/// A model placed in a [`Scene`] with the material library its material IDs index.
///
/// The library comes from the caller, not from [`Model::materials`], so that a scene can draw a
/// model with the library its Entity chooses and share one library among many models.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ScenePart<'m> {
    /// The model; its nodes hang below the scene node that holds the part.
    pub model: &'m Model,
    /// The material library of the model, see [`MaterialStatus::library`]; a status without a
    /// library draws the model with the engine's default material.
    pub materials: &'m MaterialStatus,
}

impl<'m> ScenePart<'m> {
    /// Places `model`, drawn with the library of `materials`.
    #[must_use]
    pub const fn new(model: &'m Model, materials: &'m MaterialStatus) -> Self {
        Self { model, materials }
    }
}

/// One node of a [`Scene`]: a name, a transform relative to its parent and optionally a model.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SceneNode<'m> {
    /// The node's name, e.g. `hardpoint_weapon_nose/AEGS_Mount_Gimbal_S3`.
    pub name: String,
    /// The index of the parent node, which comes before this one, or `None` for a root.
    pub parent: Option<usize>,
    /// The transform from the node's space to its parent's, in `CryEngine` coordinates.
    pub transform: Transform,
    /// The model placed at the node, if it has one.
    pub part: Option<ScenePart<'m>>,
}

impl<'m> SceneNode<'m> {
    /// Creates a node named `name` below the node `parent`.
    #[must_use]
    pub fn new(name: impl Into<String>, parent: Option<usize>, transform: Transform) -> Self {
        Self {
            name: name.into(),
            parent,
            transform,
            part: None,
        }
    }

    /// Places `part` at the node.
    #[must_use]
    pub const fn with_part(mut self, part: ScenePart<'m>) -> Self {
        self.part = Some(part);
        self
    }
}

/// A tree of placed models in `CryEngine` coordinates, Z up and in metres, for
/// [`write_scene_glb`](crate::write_scene_glb).
///
/// Every node comes after its parent, so a scene is a forest by construction.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Scene<'m> {
    nodes: Vec<SceneNode<'m>>,
}

impl<'m> Scene<'m> {
    /// Creates a scene without nodes.
    #[must_use]
    pub const fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Adds `node` and returns its index.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError::InvalidSceneNode`] when the node's parent is not an earlier node.
    pub fn push(&mut self, node: SceneNode<'m>) -> Result<usize, GltfError> {
        let index = self.nodes.len();
        if let Some(parent) = node.parent
            && parent >= index
        {
            return Err(GltfError::InvalidSceneNode {
                index,
                name: node.name,
                parent,
            });
        }
        self.nodes.push(node);
        Ok(index)
    }

    /// Returns the nodes in the order they were added.
    #[must_use]
    pub fn nodes(&self) -> &[SceneNode<'m>] {
        &self.nodes
    }
}
