// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

use crate::body_reader::BodyReader;
use crate::body_writer::Put;
use crate::material::GROUP_MARKER;
use crate::{CHF_SIZE, ChfError, Dna, Guid, ItemPort, Material};

/// The female versions this crate reads and writes.
pub const FEMALE_VERSIONS: &[u32] = &[2];
/// The male versions this crate reads and writes; from 8 on, the body ends with a zero `u32`.
pub const MALE_VERSIONS: &[u32] = &[7, 8];

/// The data of a CHF file: body type, face DNA, worn items and material choices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    /// The inner versions of the body, one per body type.
    pub versions: Versions,
    /// GUID of the body type.
    pub body_type: Guid,
    /// GUID of the voice, all zeros when none is chosen.
    pub voice: Guid,
    /// The face, absent in a CHF that only holds materials.
    pub dna: Option<Dna>,
    /// The root of the item port tree, absent when the CHF holds no ports.
    pub item_ports: Option<ItemPort>,
    /// The count the layout keeps behind the item port count, as read.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub item_port_tail: u32,
    /// The material choices, one per attachment.
    pub materials: Vec<Material>,
}

/// The inner versions a CHF body starts with, one per body type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Versions {
    /// The female body version, one of [`FEMALE_VERSIONS`].
    pub female: u32,
    /// The male body version, one of [`MALE_VERSIONS`].
    pub male: u32,
}

impl Versions {
    fn check(self) -> Result<Self, ChfError> {
        let known = |field, version, supported: &'static [u32]| {
            if supported.contains(&version) {
                Ok(())
            } else {
                Err(ChfError::UnknownVersion {
                    field,
                    version,
                    supported,
                })
            }
        };
        known("female", self.female, FEMALE_VERSIONS)?;
        known("male", self.male, MALE_VERSIONS)?;
        Ok(self)
    }

    const fn trailer_len(self) -> usize {
        if self.male >= 8 { 4 } else { 0 }
    }
}

impl Character {
    /// Parses a decompressed CHF body.
    ///
    /// # Errors
    ///
    /// [`ChfError::UnknownVersion`] for a version this crate does not read, and the other
    /// [`ChfError`] variants for a body that is truncated or breaks the layout.
    pub fn read(body: &[u8]) -> Result<Self, ChfError> {
        let mut reader = BodyReader::new(body);
        let versions = Versions {
            female: reader.u32()?,
            male: reader.u32()?,
        }
        .check()?;
        let body_type = reader.guid()?;
        let voice = reader.guid()?;
        let dna = Dna::read(&mut reader)?;
        let (item_ports, item_port_tail) = ItemPort::read_tree(&mut reader)?;
        reader.expect_u32("material group marker", GROUP_MARKER)?;
        let materials = Material::read_all(&mut reader, versions.trailer_len())?;
        if versions.trailer_len() > 0 {
            reader.expect_u32("body trailer", 0)?;
        }

        Ok(Self {
            versions,
            body_type,
            voice,
            dna,
            item_ports,
            item_port_tail,
            materials,
        })
    }

    /// Writes the decompressed CHF body.
    ///
    /// # Errors
    ///
    /// [`ChfError::UnknownVersion`] for a version this crate does not write,
    /// [`ChfError::TooDeep`] for an item port tree nested too deep, and [`ChfError::Unwritable`]
    /// for values the layout cannot store.
    pub fn write(&self) -> Result<Vec<u8>, ChfError> {
        let versions = self.versions.check()?;
        let mut out = Vec::with_capacity(CHF_SIZE);
        out.put_u32(versions.female);
        out.put_u32(versions.male);
        out.put_guid(self.body_type);
        out.put_guid(self.voice);
        Dna::write(self.dna.as_ref(), &mut out)?;
        ItemPort::write_tree(self.item_ports.as_ref(), self.item_port_tail, &mut out)?;
        out.put_u32(GROUP_MARKER);
        Material::write_all(&self.materials, &mut out)?;
        if versions.trailer_len() > 0 {
            out.put_u32(0);
        }
        Ok(out)
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde calls skip_serializing_if by reference"
)]
const fn is_zero(value: &u32) -> bool {
    *value == 0
}
