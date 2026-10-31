// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reads a model Entry without its material library.

use p4kx_extraction::model::{LodSelection, MaterialReading, PositionBox};
use p4kx_extraction::{ConversionError, ModelSource, PathIndex};

/// Reads the model Entry at `path` of the Archive `index` is over, at the levels `lods`, without
/// its material library.
pub(crate) fn without_materials(
    index: &PathIndex<'_>,
    path: &str,
    lods: LodSelection,
) -> Result<ModelSource, ConversionError> {
    ModelSource::from_index_reading(
        index,
        path,
        lods,
        PositionBox::default(),
        MaterialReading::Skipped,
    )
}
