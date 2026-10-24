// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::Archive;

use super::models::attach;
use super::walk::walk;
use super::{Interior, InteriorOptions};
use crate::AssemblyError;

impl Interior {
    /// Builds the Interior of the socpak container `layers.last()`, which was opened from the
    /// Entry `entry` of the Archive or container before it in `layers`; `layers` runs from the
    /// Archive that holds the game's files to the container.
    ///
    /// The container's description (`<name>.xml`) names the containers nested in it, each with
    /// its position and rotation in the parent's space, and the `.soc` files that place models;
    /// nested containers are entered recursively and their transforms composed, so a child is
    /// moved by the parent's rotation as well as its position. A container that is nested in
    /// itself is not entered again. A name is looked for in the container that names it, then in
    /// the containers around it, then below `Data` in the first Archive.
    ///
    /// The models are looked for inside the container of the `.soc` file first, then in the
    /// containers around it and last in the Archive, and are read at the level of detail in
    /// `options`, each distinct file once and in parallel; the library a model names is read
    /// once. Whatever cannot be resolved is left out and listed as an [`Issue`](crate::Issue).
    /// The parts, their nesting, the containers and the vertices are limited by
    /// [`MAX_INTERIOR_PARTS`](crate::MAX_INTERIOR_PARTS),
    /// [`MAX_INTERIOR_DEPTH`](crate::MAX_INTERIOR_DEPTH),
    /// [`MAX_INTERIOR_CONTAINERS`](crate::MAX_INTERIOR_CONTAINERS) and
    /// [`InteriorOptions::max_vertices`].
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Provider`] when a file cannot be read for every container, e.g.
    /// because an Archive key is missing.
    pub fn build(
        layers: &[&Archive],
        entry: &str,
        options: &InteriorOptions,
    ) -> Result<Self, AssemblyError> {
        let mut walked = walk(layers, entry, options)?;
        attach(&mut walked, options)?;
        Ok(Self {
            parts: walked.parts,
            issues: walked.issues,
            lod: options.lod,
            containers: walked.containers,
        })
    }
}
