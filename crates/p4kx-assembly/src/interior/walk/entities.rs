// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_model::{MaterialStatus, SocEntities, Transform};

use super::{Placement, Todo, Walker};
use crate::IssueKind;
use crate::interior::files::stem;
use crate::interior::light::scene_light;
use crate::interior::{InteriorPart, InteriorPartKind};

/// A part found in a `.soc` file, before it has a place in the part list.
pub(super) struct Placed<'a> {
    pub(super) name: String,
    pub(super) kind: InteriorPartKind,
    pub(super) path: Option<&'a str>,
    pub(super) transform: Transform,
}

impl Walker<'_> {
    /// Adds the part below the part `at`, whose world transform is `world`; returns whether there
    /// was room for it.
    pub(super) fn add_placed(
        &mut self,
        todo: &Todo,
        at: usize,
        world: Transform,
        placed: Placed<'_>,
    ) -> bool {
        if !self.room(todo) {
            return false;
        }
        if placed.path.is_some() {
            self.placements.push(Placement {
                part: self.parts.len(),
                chain: Arc::clone(&todo.chain),
                container: Arc::clone(&todo.path),
            });
        }
        self.parts.push(InteriorPart {
            name: placed.name,
            kind: placed.kind,
            parent: Some(at),
            local: placed.transform,
            world: world.then(&placed.transform),
            model_path: placed.path.map(str::to_owned),
            model: None,
            materials: Arc::new(MaterialStatus::Unnamed),
            light: None,
        });
        true
    }

    /// Adds the props and lights of a `.soc` file named `file` below the part `at`; returns
    /// whether there was room for all of them.
    pub(super) fn add_entities(
        &mut self,
        todo: &Todo,
        at: usize,
        world: Transform,
        file: &str,
        entities: &SocEntities,
    ) -> bool {
        if entities.invalid > 0 {
            let count = entities.invalid;
            self.note(&todo.path, file, IssueKind::EntitiesLeftOut { count });
        }
        for prop in &entities.props {
            let name = if prop.name.is_empty() {
                stem(&prop.model)
            } else {
                &prop.name
            };
            let placed = Placed {
                name: name.to_owned(),
                kind: InteriorPartKind::Prop,
                path: Some(&prop.model),
                transform: prop.transform,
            };
            if !self.add_placed(todo, at, world, placed) {
                return false;
            }
        }
        let mut unsupported: Vec<(String, usize)> = Vec::new();
        for light in &entities.lights {
            if scene_light(light).is_none() {
                match unsupported
                    .iter_mut()
                    .find(|(kind, _)| kind.eq_ignore_ascii_case(&light.light_type))
                {
                    Some((_, count)) => *count += 1,
                    None => unsupported.push((light.light_type.clone(), 1)),
                }
                continue;
            }
            let placed = Placed {
                name: light.name.clone(),
                kind: InteriorPartKind::Light,
                path: None,
                transform: light.transform,
            };
            if !self.add_placed(todo, at, world, placed) {
                return false;
            }
            if let Some(part) = self.parts.last_mut() {
                part.light = Some(light.clone());
            }
        }
        for (light_type, count) in unsupported {
            self.note(
                &todo.path,
                file,
                IssueKind::LightUnsupported { light_type, count },
            );
        }
        true
    }
}
