// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_model::Model;

use super::EntityInterior;
use crate::interior::model_vertices;
use crate::{Assembly, InteriorOptions, IssueKind};

/// What is left of the limits of one Entity's Interior after the containers built so far.
pub(super) struct Budget {
    options: InteriorOptions,
    hull_vertices: usize,
}

impl Budget {
    pub(super) fn new(assembly: &Assembly, options: &InteriorOptions) -> Self {
        let mut seen: Vec<*const Model> = Vec::new();
        let mut hull_vertices = 0_usize;
        for model in assembly
            .parts()
            .iter()
            .filter_map(|part| part.model.as_ref())
        {
            let pointer = Arc::as_ptr(model);
            if !seen.contains(&pointer) {
                seen.push(pointer);
                hull_vertices = hull_vertices.saturating_add(model_vertices(model));
            }
        }
        Self {
            options: *options,
            hull_vertices,
        }
    }

    fn used_vertices(&self, built: &EntityInterior) -> usize {
        built
            .containers
            .iter()
            .fold(self.hull_vertices, |sum, container| {
                sum.saturating_add(container.interior.vertices())
            })
    }

    fn used_parts(built: &EntityInterior) -> usize {
        built.parts()
    }

    fn used_containers(built: &EntityInterior) -> usize {
        built
            .containers
            .iter()
            .map(|container| container.interior.containers())
            .sum()
    }

    /// Returns why no further container may be built, or `None` while something is left.
    pub(super) fn exhausted(&self, built: &EntityInterior) -> Option<IssueKind> {
        let options = &self.options;
        if Self::used_containers(built) >= options.max_containers {
            let limit = options.max_containers;
            return Some(IssueKind::TooManyContainers { limit });
        }
        if Self::used_parts(built) >= options.max_parts {
            let limit = options.max_parts;
            return Some(IssueKind::TooManyParts { limit });
        }
        if self.used_vertices(built) >= options.max_vertices {
            let limit = options.max_vertices;
            return Some(IssueKind::TooManyVertices { limit });
        }
        None
    }

    /// Returns the options for the next container: the limits less what was used.
    pub(super) fn options(&self, built: &EntityInterior) -> InteriorOptions {
        let mut options = self.options;
        options.max_containers = options
            .max_containers
            .saturating_sub(Self::used_containers(built));
        options.max_parts = options.max_parts.saturating_sub(Self::used_parts(built));
        options.max_vertices = options
            .max_vertices
            .saturating_sub(self.used_vertices(built));
        options
    }
}
