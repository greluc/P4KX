// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Blender package service of P4KX: writes an [`Assembly`](p4kx_assembly::Assembly) as a
//! decomposed package that does not depend on Blender's file format.
//!
//! A package is a folder:
//!
//! ```text
//! <package>/
//!   scene.json        the manifest: version, source, node hierarchy, meshes, textures, issues
//!   meshes/m0001.glb  one GLB per distinct mesh set (model, material library and palette)
//!   textures/t0001.png  one PNG per distinct texture of the materials
//! ```
//!
//! [`write_package`] writes it; [`Manifest`] is the type of `scene.json` and [`manifest_schema`]
//! its JSON Schema, checked in as `schemas/blender-package.schema.json`. The file names are made
//! of ids, never of names from the Archive, so no name can leave the package folder. The format
//! is documented in `docs/blender-package.md`.

mod build;
mod collect;
mod error;
mod manifest;
mod meshes;
mod plan;
mod schema;
mod stage;
mod textures;
mod write;

pub use error::PackageError;
pub use manifest::{
    AlphaMeaning, Basis, BuildInfo, ColourSpace, Coordinates, EntityRef, FORMAT, FORMAT_VERSION,
    Format, Generator, Issue, IssueOrigin, MANIFEST_FILE, MESHES_FOLDER, Manifest,
    MaterialTextures, Mesh, MeshMaterial, Node, NodeKind, Options, Replacement, Source,
    TEXTURES_FOLDER, Texture, TextureFormat, Unit,
};
pub use schema::manifest_schema;
pub use stage::check_output;
pub use write::{PackageInfo, PackageSummary, write_package};

pub(crate) type PackageResult<T> = Result<T, PackageError>;

/// Asks the caller whether to stop.
pub(crate) struct Cancelled<'a>(&'a (dyn Fn() -> bool + Sync));

impl<'a> Cancelled<'a> {
    pub(crate) const fn new(check: &'a (dyn Fn() -> bool + Sync)) -> Self {
        Self(check)
    }

    pub(crate) fn check(&self) -> PackageResult<()> {
        if (self.0)() {
            Err(PackageError::Cancelled)
        } else {
            Ok(())
        }
    }
}
