// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use p4kx_gltf::TextureLoader;
use p4kx_texture::{TextureError, write_png};
use rayon::prelude::*;

use crate::{Cancelled, PackageError, PackageResult};

/// How many textures are decoded and encoded at once; a texture can hold 64 megapixels.
const BATCH: usize = 4;

/// A texture file some material names.
#[derive(Debug, Clone)]
pub(crate) struct Wanted {
    pub(crate) path: String,
    pub(crate) file: String,
    pub(crate) material: String,
}

/// What became of one wanted texture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Written { width: u32, height: u32, bytes: u64 },
    Failed(String),
}

/// Returns the file name of the texture at position `index` of the wanted ones.
pub(crate) fn id_of(index: usize) -> String {
    format!("t{:04}", index + 1)
}

/// Loads every wanted texture at `mip` and writes it as `<folder>/<id>.png`.
///
/// A texture that cannot be loaded or encoded is a [`Outcome::Failed`] and leaves no file.
pub(crate) fn write_all(
    wanted: &[Wanted],
    loader: &dyn TextureLoader,
    mip: u32,
    folder: &Path,
    cancelled: &Cancelled<'_>,
) -> PackageResult<Vec<Outcome>> {
    let mut outcomes = Vec::with_capacity(wanted.len());
    for (batch, chunk) in wanted.chunks(BATCH).enumerate() {
        cancelled.check()?;
        let done: Vec<Outcome> = chunk
            .par_iter()
            .enumerate()
            .map(|(offset, texture)| {
                cancelled.check()?;
                write_one(texture, &id_of(batch * BATCH + offset), loader, mip, folder)
            })
            .collect::<PackageResult<_>>()?;
        outcomes.extend(done);
    }
    Ok(outcomes)
}

fn write_one(
    texture: &Wanted,
    id: &str,
    loader: &dyn TextureLoader,
    mip: u32,
    folder: &Path,
) -> PackageResult<Outcome> {
    let decoded = match loader.load(&texture.path, mip) {
        Ok(decoded) => decoded,
        Err(error) => return Ok(Outcome::Failed(error.to_string())),
    };
    let image = decoded.image;
    let path = folder.join(format!("{id}.png"));
    let file = File::create_new(&path).map_err(|source| PackageError::write(&path, source))?;
    let mut writer = BufWriter::new(file);
    let written = write_png(&image, &mut writer);
    match written {
        Ok(()) => {}
        Err(TextureError::Write(source)) => return Err(PackageError::write(&path, source)),
        Err(error) => {
            drop(writer);
            std::fs::remove_file(&path).map_err(|source| PackageError::write(&path, source))?;
            return Ok(Outcome::Failed(error.to_string()));
        }
    }
    writer
        .flush()
        .map_err(|source| PackageError::write(&path, source))?;
    let bytes = writer
        .get_ref()
        .metadata()
        .map_err(|source| PackageError::write(&path, source))?
        .len();
    Ok(Outcome::Written {
        width: image.width(),
        height: image.height(),
        bytes,
    })
}
