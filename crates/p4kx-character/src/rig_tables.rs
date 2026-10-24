// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::rig::{SLOT_JOINT_WEIGHTS, SLOT_REGION_MASKS, SLOT_VARIANT_JOINTS};
use crate::rig_bytes::{f32_at, vec3_at};
use crate::{DnaRig, RigError};

const VEC3_LEN: usize = 12;
const MASK_TRAILER: usize = 8;

/// The translation of each joint of the rig for each head: parent-local offsets, with the joints
/// in the order of the rig's joint list and the heads in the order of
/// [`DnaRig::variant_names`].
#[derive(Debug, Clone, Copy)]
pub struct VariantJoints<'a> {
    data: &'a [u8],
    variants: usize,
    joints: usize,
}

impl VariantJoints<'_> {
    /// Returns the number of heads.
    #[must_use]
    pub const fn variants(&self) -> usize {
        self.variants
    }

    /// Returns the number of joints.
    #[must_use]
    pub const fn joints(&self) -> usize {
        self.joints
    }

    /// Returns the translation of `joint` for `variant` in metres, relative to its parent joint,
    /// or `None` outside the table.
    #[must_use]
    pub fn translation(&self, variant: usize, joint: usize) -> Option<[f32; 3]> {
        if variant >= self.variants || joint >= self.joints {
            return None;
        }
        let at = variant
            .checked_mul(self.joints)?
            .checked_add(joint)?
            .checked_mul(VEC3_LEN)?;
        vec3_at(self.data, at)
    }
}

/// How much each region of the face moves each joint of the rig: `joints` rows of one weight per
/// region, each row summing to 1 or, for a joint no region moves, to 0.
#[derive(Debug, Clone, Copy)]
pub struct JointWeights<'a> {
    data: &'a [u8],
    joints: usize,
    regions: usize,
}

impl JointWeights<'_> {
    /// Returns the number of joints.
    #[must_use]
    pub const fn joints(&self) -> usize {
        self.joints
    }

    /// Returns the number of regions.
    #[must_use]
    pub const fn regions(&self) -> usize {
        self.regions
    }

    /// Returns the weight of `region` for `joint`, or `None` outside the table.
    #[must_use]
    pub fn weight(&self, joint: usize, region: usize) -> Option<f32> {
        if joint >= self.joints || region >= self.regions {
            return None;
        }
        let at = joint.checked_mul(self.regions)?.checked_add(region)?;
        f32_at(self.data, at.checked_mul(4)?)
    }
}

/// How much each vertex of the mesh belongs to each region the file covers: for every vertex, the
/// weights over the regions sum to 1.
#[derive(Debug, Clone, Copy)]
pub struct RegionMasks<'a> {
    data: &'a [u8],
    vertices: usize,
    covered: usize,
}

impl RegionMasks<'_> {
    /// Returns the number of vertices.
    #[must_use]
    pub const fn vertices(&self) -> usize {
        self.vertices
    }

    /// Returns the weight of the covered region number `local` for `vertex`, or `None` outside
    /// the table.
    #[must_use]
    pub fn weight(&self, local: usize, vertex: usize) -> Option<f32> {
        if local >= self.covered || vertex >= self.vertices {
            return None;
        }
        let at = local.checked_mul(self.vertices)?.checked_add(vertex)?;
        f32_at(self.data, at.checked_mul(4)?)
    }
}

impl<'a> DnaRig<'a> {
    /// Returns the number of joints of the rig, from the size of the head joint table.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] or [`RigError::SlotTooShort`] without a joint table.
    pub fn joint_count(&self) -> Result<usize, RigError> {
        let data = self.slot(SLOT_VARIANT_JOINTS, "head joint offsets")?;
        Ok(data.len() / (self.variants() * VEC3_LEN))
    }

    /// Reads the table of the joint translations of each head.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] or [`RigError::SlotTooShort`] when the table is missing or
    /// holds no joint.
    pub fn variant_joints(&self) -> Result<VariantJoints<'a>, RigError> {
        let data = self.slot(SLOT_VARIANT_JOINTS, "head joint offsets")?;
        let joints = self.joint_count()?;
        if joints == 0 {
            return Err(RigError::SlotTooShort {
                what: "head joint offsets",
                slot: SLOT_VARIANT_JOINTS,
                need: (self.variants() * VEC3_LEN) as u64,
                have: data.len(),
            });
        }
        Ok(VariantJoints {
            data,
            variants: self.variants(),
            joints,
        })
    }

    /// Reads the table of the weights of each region for each joint.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] or [`RigError::SlotTooShort`] when the table is missing or
    /// has fewer rows than the rig has joints.
    pub fn joint_weights(&self) -> Result<JointWeights<'a>, RigError> {
        let data = self.slot(SLOT_JOINT_WEIGHTS, "joint weights")?;
        let joints = self.joint_count()?;
        let regions = self.regions();
        let need = joints.saturating_mul(regions).saturating_mul(4);
        if data.len() < need {
            return Err(RigError::SlotTooShort {
                what: "joint weights",
                slot: SLOT_JOINT_WEIGHTS,
                need: need as u64,
                have: data.len(),
            });
        }
        Ok(JointWeights {
            data,
            joints,
            regions,
        })
    }

    /// Returns the number of vertices of the mesh this file belongs to.
    ///
    /// The region masks give it exactly, since they hold one weight per covered region and vertex
    /// and a trailer; the header's count is used when they do not divide evenly.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] without region masks and [`RigError::Malformed`] when they
    /// cover no region.
    pub fn vertex_count(&self) -> Result<usize, RigError> {
        let data = self.slot(SLOT_REGION_MASKS, "region masks")?;
        let covered = self.covered_regions()?.len();
        if covered == 0 {
            return Err(RigError::Malformed {
                what: "region masks",
                slot: SLOT_REGION_MASKS,
                reason: "the file covers no region",
            });
        }
        let row = covered * 4;
        let body = data.len().saturating_sub(MASK_TRAILER);
        if body > 0 && body % row == 0 {
            Ok(body / row)
        } else {
            Ok(usize::try_from(self.header().vertices).unwrap_or(usize::MAX))
        }
    }

    /// Reads the region masks of the mesh.
    ///
    /// # Errors
    ///
    /// Returns [`RigError::SlotUnset`] or [`RigError::SlotTooShort`] when the masks are missing or
    /// shorter than the vertex count needs.
    pub fn region_masks(&self) -> Result<RegionMasks<'a>, RigError> {
        let data = self.slot(SLOT_REGION_MASKS, "region masks")?;
        let covered = self.covered_regions()?.len();
        let vertices = self.vertex_count()?;
        let need = vertices
            .checked_mul(covered)
            .and_then(|cells| cells.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(MASK_TRAILER))
            .filter(|&need| need <= data.len())
            .ok_or_else(|| RigError::SlotTooShort {
                what: "region masks",
                slot: SLOT_REGION_MASKS,
                need: (vertices as u64)
                    .saturating_mul(covered as u64)
                    .saturating_mul(4)
                    .saturating_add(MASK_TRAILER as u64),
                have: data.len(),
            })?;
        Ok(RegionMasks {
            data: data.get(..need).unwrap_or(data),
            vertices,
            covered,
        })
    }
}
