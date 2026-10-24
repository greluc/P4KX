// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::build::{self, Built, Request, Shared};
use super::task::Task;
use super::{Building, CharactersTab, Shown, expressions};
use crate::{Camera, Control, PartIssue, Playback, ViewFailure};

impl CharactersTab {
    fn request(&self) -> Request {
        Request {
            variant: self.body(),
            character: self.head.as_ref().map(|head| Arc::clone(&head.character)),
            pieces: self.pieces.iter().map(super::Pick::guid).collect(),
            loadout: self.loadout.clone(),
            lod: self.lod,
            expressions: self.expressions.enabled,
        }
    }

    /// Builds the Character again off the UI thread from the current choices, replacing a build
    /// that runs; the view built before stays shown.
    pub(super) fn rebuild(&mut self) {
        let Some(ctx) = &self.ctx else {
            return;
        };
        let request = self.request();
        let shared = Arc::new(Shared::default());
        let (index, working) = (Arc::clone(&ctx.index), Arc::clone(&shared));
        self.cancelled = false;
        let spawned = Task::spawn("character-build", Arc::clone(&ctx.wake), move |cancel| {
            build::run(&index, &request, cancel, &working)
        });
        match spawned {
            Ok(task) => {
                self.building = Some(Building { task, shared });
                self.failure = None;
            }
            Err(reason) => {
                self.building = None;
                self.failure = Some(ViewFailure::Failed(reason));
            }
        }
    }

    pub(super) fn install(&mut self, built: Built) {
        let Built {
            armour,
            scene,
            spans,
            replaced,
        } = built;
        let issues = armour.issues().iter().map(PartIssue::of).collect();
        let kept = self
            .expressions
            .chosen
            .and_then(|at| self.expressions.list.get(at))
            .map(|expression| expression.name().to_owned());
        self.expressions.list = expressions::listed(&armour);
        self.expressions.chosen = kept.and_then(|name| {
            self.expressions
                .list
                .iter()
                .position(|expression| expression.name() == name)
        });
        let framing = Camera::framing(scene.bounds());
        if self.camera.as_ref().is_none_or(Camera::is_home) {
            self.camera = Some(framing);
        }
        let skeleton_shown = self.playback.as_ref().is_some_and(Playback::shows_skeleton);
        self.playback = scene
            .skeleton()
            .map(|skeleton| Playback::new(Arc::clone(skeleton), Vec::new()));
        if let (true, Some(playback)) = (skeleton_shown, &mut self.playback) {
            playback.apply(Control::Skeleton(true));
        }
        let base = Arc::new(scene);
        self.shown = Some(Shown {
            armour: Arc::new(armour),
            scene: Arc::clone(&base),
            base,
            spans,
            issues,
            replaced,
        });
        self.failure = None;
        self.apply_expression();
    }

    /// Stops the build that runs; the view built before stays.
    pub fn cancel(&mut self) {
        if let Some(building) = self.building.take() {
            building.task.cancel();
            self.cancelled = true;
        }
    }

    /// Builds the Character again, e.g. after it was stopped.
    pub fn restart(&mut self) {
        self.rebuild();
    }
}
