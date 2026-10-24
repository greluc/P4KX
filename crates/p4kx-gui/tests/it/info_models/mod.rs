// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Models with something to list in the Info tab: a chunk of an unknown type and a material
//! library the Archive lacks, in an Archive of four Entries so that every row stays in view.

use p4kx_testkit::{TestEntry, TestModel};

use crate::viewing::Site;

pub(crate) fn site() -> Site {
    let extra = TestModel::cube().with_chunk(0x0BAD_F00D, vec![0; 8]);
    let painted = TestModel::cube().with_material_library("objects/gone");
    Site {
        entries: vec![
            TestEntry::zstd("Data\\Objects\\extra.cgf", extra.cgf()),
            TestEntry::zstd("Data\\Objects\\extra.cgfm", extra.cgfm()),
            TestEntry::zstd("Data\\Objects\\painted.cgf", painted.cgf()),
            TestEntry::zstd("Data\\Objects\\painted.cgfm", painted.cgfm()),
        ],
        installation: false,
        key: false,
    }
}
