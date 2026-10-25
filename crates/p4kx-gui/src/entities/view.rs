// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod failure;
mod interior;
mod job;

use std::collections::HashSet;
use std::sync::Arc;

use p4kx_assembly::EntityNode;
use p4kx_settings::ModelMaterials;

pub use failure::ViewFailure;
pub use job::{ViewProgress, ViewStage};

use crate::viewer::{AssemblyDraw, hidden_mask};
use crate::{ASSEMBLY_LODS, AssemblyScene, Camera, DrawList, EntryIndex, Matrix};

/// The level of detail the view opens with: light enough for a capital ship.
pub const VIEW_LOD: u32 = 1;

#[derive(Debug)]
struct Shown {
    scene: Arc<AssemblyScene>,
    mask: Vec<bool>,
    issues: usize,
    interior_issues: usize,
    with_interior: bool,
    list: Option<(ListKey, Arc<DrawList>)>,
    mask_serial: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ListKey {
    view: Matrix,
    height: f32,
    mask_serial: u64,
}

/// The 3D view of the Loadout of the selected Entity: the Assembly built from the Loadout tree
/// as it resolved, the choices of detail, materials and Interior, the branches hidden and the
/// camera.
///
/// Every new tree or choice builds the Assembly again off the UI thread; the view built before
/// stays shown until the new one arrives. The Interior is built once it is switched on; switching
/// it off hides it without building again, and switching it on again shows it as long as the
/// view was not built anew without it.
pub struct AssemblyView {
    index: Arc<EntryIndex>,
    wake: Arc<dyn Fn() + Send + Sync>,
    lod: u32,
    materials: ModelMaterials,
    interior: bool,
    chosen: bool,
    hidden: HashSet<String>,
    tree: Option<Arc<EntityNode>>,
    job: Option<job::Job>,
    shown: Option<Shown>,
    failure: Option<ViewFailure>,
    camera: Option<Camera>,
    cancelled: bool,
    counter: u64,
    #[cfg(feature = "test-hooks")]
    gate: Option<Arc<crate::hold::Gate>>,
}

impl std::fmt::Debug for AssemblyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssemblyView")
            .field("lod", &self.lod)
            .field("materials", &self.materials)
            .field("interior", &self.interior)
            .field("hidden", &self.hidden)
            .field("building", &self.job.is_some())
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

impl AssemblyView {
    /// Starts at the level of detail `initial` names, though at least [`VIEW_LOD`] to stay light,
    /// and the materials it names.
    pub(super) fn new(
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
        initial: (u32, ModelMaterials),
    ) -> Self {
        let lod = if ASSEMBLY_LODS.contains(&initial.0) {
            initial.0.max(VIEW_LOD)
        } else {
            VIEW_LOD
        };
        Self {
            index: Arc::clone(index),
            wake,
            lod,
            materials: initial.1,
            interior: false,
            chosen: false,
            hidden: HashSet::new(),
            tree: None,
            job: None,
            shown: None,
            failure: None,
            camera: None,
            cancelled: false,
            counter: 0,
            #[cfg(feature = "test-hooks")]
            gate: None,
        }
    }

    /// Makes the builds of the view stay in their Interior step until the returned hold is
    /// dropped.
    #[cfg(feature = "test-hooks")]
    pub fn hold_interior(&mut self) -> crate::Hold {
        let (hold, gate) = crate::hold::Hold::new();
        self.gate = Some(gate);
        hold
    }

    /// Returns the level of detail the view is built at.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Builds the view at another level of detail; one the export does not offer is ignored.
    pub fn set_lod(&mut self, lod: u32) {
        if lod != self.lod && ASSEMBLY_LODS.contains(&lod) {
            self.lod = lod;
            self.chosen = true;
            self.build();
        }
    }

    /// Returns whether the view draws plain colours or the textures over them.
    #[must_use]
    pub const fn materials(&self) -> ModelMaterials {
        self.materials
    }

    /// Builds the view with other materials.
    pub fn set_materials(&mut self, materials: ModelMaterials) {
        if materials != self.materials {
            self.materials = materials;
            self.chosen = true;
            self.build();
        }
    }

    /// Returns whether the user changed the level of detail, the materials or the Interior of
    /// the view; the export then starts with them, else with the choices remembered from the
    /// last export.
    #[must_use]
    pub const fn is_chosen(&self) -> bool {
        self.chosen
    }

    /// Returns whether the branch below the port `path` is hidden in the view.
    #[must_use]
    pub fn is_hidden(&self, path: &str) -> bool {
        self.hidden.contains(path)
    }

    /// Returns how many branches are hidden.
    #[must_use]
    pub fn hidden_count(&self) -> usize {
        self.hidden.len()
    }

    /// Hides the branch below the port `path` in the view, or shows it again; the Loadout and the
    /// export are not changed, and nothing is built again.
    pub fn set_hidden(&mut self, path: &str, hidden: bool) {
        let changed = if hidden {
            self.hidden.insert(path.to_owned())
        } else {
            self.hidden.remove(path)
        };
        if changed {
            self.remask();
        }
    }

    /// Shows every hidden branch again.
    pub fn show_all(&mut self) {
        if !self.hidden.is_empty() {
            self.hidden.clear();
            self.remask();
        }
    }

    fn remask(&mut self) {
        self.counter += 1;
        if let Some(shown) = &mut self.shown {
            shown.mask = hidden_mask(shown.scene.parts(), &self.hidden, self.interior);
            shown.mask_serial = self.counter;
            shown.list = None;
        }
    }

    /// Returns the scene last built, if any.
    #[must_use]
    pub fn scene(&self) -> Option<&Arc<AssemblyScene>> {
        self.shown.as_ref().map(|shown| &shown.scene)
    }

    /// Returns how many parts the last build could not resolve: those of the Loadout, and those
    /// of the Interior while it is shown.
    #[must_use]
    pub fn issues(&self) -> usize {
        self.shown.as_ref().map_or(0, |shown| {
            shown.issues + shown.interior_issues * usize::from(self.interior)
        })
    }

    /// Returns how many parts of the Loadout are hidden, with the hidden branches' parts; the
    /// parts of the Interior are not counted.
    #[must_use]
    pub fn hidden_parts(&self) -> usize {
        self.shown.as_ref().map_or(0, |shown| {
            shown
                .mask
                .iter()
                .zip(shown.scene.parts())
                .filter(|(hidden, part)| **hidden && !part.is_interior())
                .count()
        })
    }

    /// Returns how far the build that runs has come, or `None` when none runs.
    #[must_use]
    pub fn progress(&self) -> Option<ViewProgress> {
        self.job.as_ref().map(job::Job::progress)
    }

    /// Returns whether the view is being built.
    #[must_use]
    pub const fn is_building(&self) -> bool {
        self.job.is_some()
    }

    /// Returns why the last build gave nothing, or `None`.
    #[must_use]
    pub const fn failure(&self) -> Option<&ViewFailure> {
        self.failure.as_ref()
    }

    /// Returns whether the last build was stopped by the user and nothing was built before.
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        self.cancelled && self.job.is_none()
    }

    /// Stops the build that runs; the view built before stays.
    pub fn cancel(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel();
            self.cancelled = true;
            if self.interior
                && self
                    .shown
                    .as_ref()
                    .is_some_and(|shown| !shown.with_interior)
            {
                self.interior = false;
                self.remask();
            }
        }
    }

    /// Builds the view again, e.g. after it was stopped.
    pub fn restart(&mut self) {
        self.build();
    }

    /// Returns the camera once a scene was built.
    #[must_use]
    pub const fn camera(&self) -> Option<&Camera> {
        self.camera.as_ref()
    }

    /// Returns the camera for moving it.
    pub const fn camera_mut(&mut self) -> Option<&mut Camera> {
        self.camera.as_mut()
    }

    /// Returns what a frame draws seen through `view_proj` in a view `height` pixels high: the
    /// scene with the parts that are not hidden, inside the view and large enough. The pick is
    /// kept while the view, the hidden branches and the scene stay as they are.
    pub(crate) fn draw(&mut self, view_proj: &Matrix, height: f32) -> Option<AssemblyDraw> {
        let shown = self.shown.as_mut()?;
        let key = ListKey {
            view: *view_proj,
            height,
            mask_serial: shown.mask_serial,
        };
        let list = match &shown.list {
            Some((cached, list)) if *cached == key => Arc::clone(list),
            _ => {
                let list = Arc::new(DrawList::of(&shown.scene, &shown.mask, view_proj, height));
                shown.list = Some((key, Arc::clone(&list)));
                list
            }
        };
        Some(AssemblyDraw {
            scene: Arc::clone(&shown.scene),
            list,
        })
    }

    /// Returns what a frame draws like the 3D view does, for a view `height` pixels high seen
    /// through `view_proj`.
    pub fn draw_list(&mut self, view_proj: &Matrix, height: f32) -> Option<Arc<DrawList>> {
        self.draw(view_proj, height).map(|drawn| drawn.list)
    }

    /// Returns, per part of the scene, whether the hidden branches hide it.
    #[must_use]
    pub fn hidden_mask(&self) -> &[bool] {
        self.shown.as_ref().map_or(&[], |shown| &shown.mask)
    }

    /// Takes the tree the Loadout resolved to and builds the view from it.
    pub(super) fn set_tree(&mut self, tree: EntityNode) {
        self.tree = Some(Arc::new(tree));
        self.build();
    }

    fn build(&mut self) {
        let Some(tree) = &self.tree else {
            return;
        };
        let request = job::Request {
            tree: Arc::clone(tree),
            lod: self.lod,
            materials: self.materials,
            interior: self.interior,
            #[cfg(feature = "test-hooks")]
            gate: self.gate.clone(),
        };
        self.cancelled = false;
        match job::start(&self.index, request, Arc::clone(&self.wake)) {
            Ok(job) => {
                self.job = Some(job);
                self.failure = None;
            }
            Err(reason) => {
                self.job = None;
                self.failure = Some(ViewFailure::Failed(reason));
            }
        }
    }

    /// Takes the result of the build once it ended; returns whether something changed.
    pub fn poll(&mut self) -> bool {
        let Some(ended) = self.job.as_ref().and_then(job::Job::poll) else {
            return false;
        };
        self.job = None;
        match ended {
            job::Ended::Built(built) => {
                self.install(*built);
            }
            job::Ended::Cancelled => self.cancelled = true,
            job::Ended::Empty => {
                self.shown = None;
                self.failure = Some(ViewFailure::Empty);
            }
            job::Ended::Failed(failure) => self.failure = Some(failure.into()),
        }
        true
    }

    fn install(&mut self, built: job::Built) {
        let job::Built {
            scene,
            issues,
            interior_issues,
            interior,
        } = built;
        self.counter += 1;
        let mask = hidden_mask(scene.parts(), &self.hidden, self.interior);
        let framing = Camera::framing(scene.bounds());
        if self.camera.as_ref().is_none_or(Camera::is_home) {
            self.camera = Some(framing);
        }
        self.failure = None;
        self.shown = Some(Shown {
            scene: Arc::new(scene),
            mask,
            issues,
            interior_issues,
            with_interior: interior,
            list: None,
            mask_serial: self.counter,
        });
    }
}
