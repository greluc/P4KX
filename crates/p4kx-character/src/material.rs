// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

use crate::body_reader::BodyReader;
use crate::body_writer::{Put, count_as};
use crate::{ChfError, Guid};

/// The value the layout puts in front of every group of sub-materials and between them.
pub(crate) const GROUP_MARKER: u32 = 5;

const MATERIAL_LEN: usize = 48;
const TEXTURE_LEN: usize = 21;
const PARAM_LEN: usize = 12;
const SUB_MATERIAL_LEN: usize = 24;

/// The material choices a Character makes for one attachment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// CRC-32 (IEEE, not CRC-32C) of the lower-cased item port name followed by `_skinitemport`.
    pub attachment: u32,
    /// GUID of the base material.
    pub base_material: Guid,
    /// Material flags, kept as read.
    pub flags: u32,
    /// The customised sub-materials.
    pub sub_materials: Vec<SubMaterial>,
}

/// The textures, float and colour parameters a Character sets on one sub-material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubMaterial {
    /// CRC-32C of the lower-cased sub-material name, e.g. `shader_eyeInner`.
    pub name: u32,
    /// The textures set by slot.
    pub textures: Vec<Texture>,
    /// The float parameters.
    pub floats: Vec<FloatParam>,
    /// The colour parameters.
    pub colors: Vec<ColorParam>,
}

/// A texture set into one slot of a sub-material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texture {
    /// The texture slot.
    pub slot: u8,
    /// GUID of the texture.
    pub texture: Guid,
}

/// A named float parameter of a sub-material.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatParam {
    /// CRC-32C of the exact parameter name, e.g. `DyeAmount`.
    pub name: u32,
    /// The value, compared and written by its exact bits.
    #[serde(with = "crate::json_values::float")]
    pub value: f32,
}

impl PartialEq for FloatParam {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.value.to_bits() == other.value.to_bits()
    }
}

impl Eq for FloatParam {}

/// A named colour parameter of a sub-material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorParam {
    /// CRC-32C of the exact parameter name when known; some, such as `BodyColor`, match no known
    /// name.
    pub name: u32,
    /// Red, green, blue and alpha, 0 to 255 each.
    pub rgba: [u8; 4],
}

impl Material {
    /// Reads materials until only `trailer_len` bytes are left.
    pub(crate) fn read_all(
        reader: &mut BodyReader<'_>,
        trailer_len: usize,
    ) -> Result<Vec<Self>, ChfError> {
        let mut materials = Vec::new();
        while reader.remaining() > trailer_len {
            reader.ensure_remaining(MATERIAL_LEN + trailer_len)?;
            let attachment = reader.u32()?;
            let base_material = reader.guid()?;
            let flags = reader.u32()?;
            reader.expect_empty_guid("material reserved GUID")?;
            let sub_count = reader.u32()?;
            reader.expect_u32("sub-material group marker", GROUP_MARKER)?;
            let sub_count = reader.count(sub_count.into(), SUB_MATERIAL_LEN)?;
            let mut sub_materials = Vec::with_capacity(sub_count);
            for index in 0..sub_count {
                sub_materials.push(SubMaterial::read(reader)?);
                let is_last = index + 1 == sub_count;
                if !is_last {
                    reader.expect_u32("sub-material group marker", GROUP_MARKER)?;
                } else if reader.remaining() > trailer_len {
                    reader.expect_u32("sub-material group marker", GROUP_MARKER)?;
                    reader.ensure_remaining(MATERIAL_LEN + trailer_len)?;
                }
            }
            materials.push(Self {
                attachment,
                base_material,
                flags,
                sub_materials,
            });
        }
        Ok(materials)
    }

    /// Writes every material with the group markers between their sub-materials.
    pub(crate) fn write_all(materials: &[Self], out: &mut Vec<u8>) -> Result<(), ChfError> {
        for (index, material) in materials.iter().enumerate() {
            let last_material = index + 1 == materials.len();
            out.put_u32(material.attachment);
            out.put_guid(material.base_material);
            out.put_u32(material.flags);
            out.put_guid(Guid::default());
            out.put_u32(count_as(material.sub_materials.len(), "sub-materials")?);
            out.put_u32(GROUP_MARKER);
            for (sub_index, sub) in material.sub_materials.iter().enumerate() {
                sub.write(out)?;
                if !(last_material && sub_index + 1 == material.sub_materials.len()) {
                    out.put_u32(GROUP_MARKER);
                }
            }
        }
        Ok(())
    }
}

impl SubMaterial {
    fn read(reader: &mut BodyReader<'_>) -> Result<Self, ChfError> {
        let name = reader.u32()?;
        let texture_count = reader.u32()?;
        let texture_count = reader.count(texture_count.into(), TEXTURE_LEN)?;
        let mut textures = Vec::with_capacity(texture_count);
        for _ in 0..texture_count {
            reader.expect_u32("texture reserved field", 0)?;
            let slot = reader.u8()?;
            let texture = reader.guid()?;
            textures.push(Texture { slot, texture });
        }
        let float_count = reader.u64()?;
        let float_count = reader.count(float_count, PARAM_LEN)?;
        let mut floats = Vec::with_capacity(float_count);
        for _ in 0..float_count {
            let name = reader.u32()?;
            let value = f32::from_bits(reader.u32()?);
            reader.expect_u32("float parameter reserved field", 0)?;
            floats.push(FloatParam { name, value });
        }
        let color_count = reader.u64()?;
        let color_count = reader.count(color_count, PARAM_LEN)?;
        let mut colors = Vec::with_capacity(color_count);
        for _ in 0..color_count {
            let name = reader.u32()?;
            let mut rgba = [0; 4];
            rgba.copy_from_slice(reader.bytes(4)?);
            reader.expect_u32("colour parameter reserved field", 0)?;
            colors.push(ColorParam { name, rgba });
        }
        Ok(Self {
            name,
            textures,
            floats,
            colors,
        })
    }

    fn write(&self, out: &mut Vec<u8>) -> Result<(), ChfError> {
        out.put_u32(self.name);
        out.put_u32(count_as(self.textures.len(), "textures")?);
        for texture in &self.textures {
            out.put_u32(0);
            out.put_u8(texture.slot);
            out.put_guid(texture.texture);
        }
        out.put_u64(count_as(self.floats.len(), "float parameters")?);
        for param in &self.floats {
            out.put_u32(param.name);
            out.put_u32(param.value.to_bits());
            out.put_u32(0);
        }
        out.put_u64(count_as(self.colors.len(), "colour parameters")?);
        for param in &self.colors {
            out.put_u32(param.name);
            out.extend_from_slice(&param.rgba);
            out.put_u32(0);
        }
        Ok(())
    }
}
