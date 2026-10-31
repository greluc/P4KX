// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::ChunkFile;

use crate::mtl_load::ModelMaterials;
use crate::{CompanionResolver, LodSelection, Model, ModelError, PositionBox};

impl Model {
    /// Reads the levels of detail `lods` of the model `file` like [`Self::read_with`], but leaves
    /// the material library the model names unread: [`Self::materials`] is
    /// [`MaterialStatus::Skipped`](crate::MaterialStatus::Skipped) and [`Self::material_library`]
    /// still holds the name, for a caller that chooses and caches the library itself, see
    /// [`MaterialStatus::load`](crate::MaterialStatus::load).
    ///
    /// # Errors
    ///
    /// As [`Self::read_with`].
    pub fn read_without_materials<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
    ) -> Result<Self, ModelError> {
        Self::read_without_materials_with_box(name, file, companions, lods, PositionBox::default())
    }

    /// Reads the model like [`Self::read_without_materials`], dequantising quantised vertex
    /// positions with the box `position_box`, see [`Self::read_with_box`].
    ///
    /// # Errors
    ///
    /// As [`Self::read_with`].
    pub fn read_without_materials_with_box<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
        position_box: PositionBox,
    ) -> Result<Self, ModelError> {
        Self::read_selecting(
            name,
            file,
            companions,
            lods,
            ModelMaterials::Skipped,
            position_box,
        )
    }

    /// Reads the levels of detail `lods` of the model `file` like [`Self::read_with`], but reads
    /// the material library without the layer files and diffuse maps that only a caller that
    /// draws needs, see [`MaterialStatus::load_plain`](crate::MaterialStatus::load_plain).
    ///
    /// # Errors
    ///
    /// As [`Self::read_with`].
    pub fn read_plain_materials<B: AsRef<[u8]>>(
        name: &str,
        file: &ChunkFile<B>,
        companions: &mut dyn CompanionResolver,
        lods: LodSelection,
    ) -> Result<Self, ModelError> {
        Self::read_selecting(
            name,
            file,
            companions,
            lods,
            ModelMaterials::Plain,
            PositionBox::default(),
        )
    }
}
