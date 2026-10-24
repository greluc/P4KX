// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_assembly::{ArchiveModels, ModelFailure, ModelProvider, ProvidedModel};
use p4kx_extraction::model::MaterialStatus;
use p4kx_extraction::{
    CancelToken, ConversionError, LoadedTexture, TextureLoadError, TextureLoader,
};

pub(crate) struct Stoppable<'a, 'b> {
    pub(crate) models: &'a ArchiveModels<'b>,
    pub(crate) cancel: &'a CancelToken,
}

impl ModelProvider for Stoppable<'_, '_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        if self.cancel.is_cancelled() {
            return Err(ModelFailure::Fatal(Box::new(ConversionError::Cancelled)));
        }
        self.models.model(path, lod)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.models.library(entry, name)
    }
}

pub(crate) struct StoppableTextures<'a> {
    pub(crate) textures: &'a dyn TextureLoader,
    pub(crate) cancel: &'a CancelToken,
}

impl TextureLoader for StoppableTextures<'_> {
    fn load(&self, path: &str, mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        if self.cancel.is_cancelled() {
            return Err(TextureLoadError::Unreadable(
                "the export stopped".to_owned(),
            ));
        }
        self.textures.load(path, mip)
    }
}

pub(crate) struct Observed<'a, W> {
    pub(crate) inner: W,
    pub(crate) cancel: &'a CancelToken,
    pub(crate) written: &'a AtomicU64,
}

impl<W: Write> Write for Observed<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(io::Error::other("the export stopped"));
        }
        let written = self.inner.write(buffer)?;
        self.written
            .fetch_add(u64::try_from(written).unwrap_or(0), Ordering::Relaxed);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
