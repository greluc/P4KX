// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The joint delta records a `RigLogic` rig carries for its own head, in the layout of the delta
//! records of a DNA rig.

use crate::rig_bytes::{f64_at, u32_at};
use crate::rig_deltas::{DeltaRecords, step_of};
use crate::{DeltaSet, RigLogicError};

const SCALES_AT: usize = 0x48;
const COUNTS_AT: usize = 0x58;
const AXES: usize = 3;
const MAIN_SLOT: usize = 3;
const SECONDARY_SLOT: usize = 4;

/// One set of delta records with the step of its coefficients.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeltaSection {
    count: u32,
    step: f64,
    bytes: Vec<u8>,
}

impl DeltaSection {
    pub(crate) const fn step(&self) -> f64 {
        self.step
    }

    pub(crate) fn records(&self) -> DeltaRecords<'_> {
        DeltaRecords::over(&self.bytes, self.count)
    }
}

/// The main and secondary delta records of a `RigLogic` rig: the translations and the Euler
/// angles each joint axis takes from the blendshapes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LogicDeltas {
    main: DeltaSection,
    secondary: DeltaSection,
}

impl LogicDeltas {
    pub(crate) const fn set(&self, set: DeltaSet) -> &DeltaSection {
        match set {
            DeltaSet::Main => &self.main,
            DeltaSet::Secondary => &self.secondary,
        }
    }
}

/// What the sections of a rig are checked against: the sizes of its joints and blendshapes.
pub(crate) struct Bounds {
    pub(crate) joints: usize,
    pub(crate) blendshapes: usize,
}

impl LogicDeltas {
    /// Reads the sets from the `main` and `secondary` sections of `rig`; `None` when the header
    /// counts no record in either.
    ///
    /// The header holds the two scales as `f64` at 0x48 and the two record counts as `u32` at
    /// 0x58. Every record must lie in its section, move a joint of the rig and name only
    /// blendshapes of the rig.
    pub(crate) fn read<'a>(
        rig: &[u8],
        mut sections: impl FnMut(usize) -> Result<&'a [u8], RigLogicError>,
        bounds: &Bounds,
    ) -> Result<Option<Self>, RigLogicError> {
        let counts = [u32_at(rig, COUNTS_AT), u32_at(rig, COUNTS_AT + 4)];
        let scales = [f64_at(rig, SCALES_AT), f64_at(rig, SCALES_AT + 8)];
        let [Some(main), Some(secondary)] = counts else {
            return Ok(None);
        };
        if main == 0 && secondary == 0 {
            return Ok(None);
        }
        let mut next = |slot, count, scale: Option<f64>| {
            let scale = scale
                .filter(|scale| scale.is_finite() && *scale > 0.0)
                .ok_or(RigLogicError::Malformed(
                    "a delta scale is not a positive number",
                ))?;
            checked(sections(slot)?, count, scale, bounds)
        };
        let main = next(MAIN_SLOT, main, scales[0])?;
        let secondary = next(SECONDARY_SLOT, secondary, scales[1])?;
        Ok(Some(Self { main, secondary }))
    }
}

fn checked(
    section: &[u8],
    count: u32,
    scale: f64,
    bounds: &Bounds,
) -> Result<DeltaSection, RigLogicError> {
    let outputs = bounds.joints.saturating_mul(AXES);
    for record in DeltaRecords::over(section, count) {
        let record = record
            .map_err(|_| RigLogicError::Malformed("a delta record is cut short by its section"))?;
        if usize::from(record.output) >= outputs {
            return Err(RigLogicError::Malformed(
                "a delta record moves a joint the rig lacks",
            ));
        }
        if record
            .entries()
            .any(|entry| usize::from(entry.blendshape) >= bounds.blendshapes)
        {
            return Err(RigLogicError::Malformed(
                "a delta record names a blendshape the rig lacks",
            ));
        }
    }
    Ok(DeltaSection {
        count,
        step: step_of(scale),
        bytes: section.to_vec(),
    })
}
