// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Archive, PathIndex};

use super::budget::Budget;
use super::{EntityInterior, find};
use crate::{
    Assembly, AssemblyError, EntityNode, Interior, InteriorForge, InteriorOptions,
    InteriorProgress, InteriorWatch, IssueKind, ModelProvider,
};

impl EntityInterior {
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
    /// `models` must read from the same Archive as `archive`.
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
}
