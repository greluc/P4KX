// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::{Deserialize, Serialize};

use crate::body_reader::{BodyReader, unexpected};
use crate::body_writer::{Put, count_as};
use crate::{ChfError, Label};

/// The value every DNA block starts with: the CRC-32C of `dna matrix 1.0`.
pub const DNA_MAGIC: u32 = 0xfcd0_9394;

/// The most face parts a DNA block may hold; the game uses 12 or 13.
pub const MAX_FACE_PARTS: usize = 64;

const DNA_HEADER_LEN: usize = 24;
const BLEND_LEN: usize = 4;

/// The face of a Character: for every face part, blends of the stock heads it mixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dna {
    /// CRC-32C of the rig name (`protos_human_<gender>_face_t1_pu`), naming the head mesh family
    /// the blends refer to.
    pub gender_hash: u32,
    /// The name of the rig, read from `gender_hash_label` and never written.
    #[serde(default, rename = "gender_hash_label", skip_serializing)]
    pub label: Label,
    /// The low half of the rig's identity; its hash function is unknown.
    pub variant_hash: u32,
    /// Number of blends each face part holds.
    pub blends_per_part: u16,
    /// A header field of unknown meaning, kept as read.
    pub header_unknown: u16,
    /// The highest head id the blends refer to, as stored.
    pub max_head_id: u16,
    /// The face parts in stored order, each with exactly `blends_per_part` blends.
    #[serde(with = "named_parts")]
    pub face_parts: Vec<Vec<DnaBlend>>,
}

/// One stock head mixed into a face part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnaBlend {
    /// The weight of the head, 0 to 65535.
    pub value: u16,
    /// The stock head.
    pub head_id: u16,
}

const PART_NAMES: [&str; 13] = [
    "eyebrow_left",
    "eyebrow_right",
    "eye_left",
    "eye_right",
    "nose",
    "ear_left",
    "ear_right",
    "cheek_left",
    "cheek_right",
    "mouth",
    "jaw",
    "crown",
    "neck",
];

fn block_size(part_count: u64, blends_per_part: u16) -> u64 {
    DNA_HEADER_LEN as u64 + part_count * u64::from(blends_per_part) * BLEND_LEN as u64
}

/// The name of the face part at `index` in stored order.
#[must_use]
pub fn face_part_name(index: usize) -> std::borrow::Cow<'static, str> {
    PART_NAMES
        .get(index)
        .map_or_else(|| format!("part_{index}").into(), |name| (*name).into())
}

impl Dna {
    /// Reads the size-prefixed DNA block, which is empty in a CHF that holds no face.
    pub(crate) fn read(reader: &mut BodyReader<'_>) -> Result<Option<Self>, ChfError> {
        let size_offset = reader.offset();
        let size = reader.u64()?;
        let len = reader.count(size, 1)?;
        if len == 0 {
            return Ok(None);
        }
        let base = reader.offset();
        let mut dna = BodyReader::with_base(reader.bytes(len)?, base);
        dna.expect_u32("DNA magic", DNA_MAGIC)?;
        let gender_hash = dna.u32()?;
        let variant_hash = dna.u32()?;
        dna.expect_u32("DNA reserved field", 0)?;
        let part_count_offset = dna.offset();
        let part_count = usize::from(dna.u16()?);
        if part_count > MAX_FACE_PARTS {
            return Err(unexpected(
                part_count_offset,
                "DNA face part count at most",
                MAX_FACE_PARTS as u128,
                part_count as u128,
            ));
        }
        let blends_per_part = dna.u16()?;
        let header_unknown = dna.u16()?;
        let max_head_id = dna.u16()?;
        let expected = block_size(part_count as u64, blends_per_part);
        if size != expected {
            return Err(unexpected(
                size_offset,
                "DNA size",
                expected.into(),
                size.into(),
            ));
        }
        let mut face_parts: Vec<Vec<DnaBlend>> = (0..part_count)
            .map(|_| Vec::with_capacity(usize::from(blends_per_part)))
            .collect();
        for _ in 0..blends_per_part {
            for part in &mut face_parts {
                part.push(DnaBlend {
                    value: dna.u16()?,
                    head_id: dna.u16()?,
                });
            }
        }
        Ok(Some(Self {
            gender_hash,
            label: Label,
            variant_hash,
            blends_per_part,
            header_unknown,
            max_head_id,
            face_parts,
        }))
    }

    /// Writes the size-prefixed DNA block, empty without a face.
    pub(crate) fn write(dna: Option<&Self>, out: &mut Vec<u8>) -> Result<(), ChfError> {
        let Some(dna) = dna else {
            out.put_u64(0);
            return Ok(());
        };
        dna.write_block(out)
    }

    fn write_block(&self, out: &mut Vec<u8>) -> Result<(), ChfError> {
        if self.face_parts.len() > MAX_FACE_PARTS {
            return Err(ChfError::Unwritable {
                reason: format!(
                    "{} DNA face parts, at most {MAX_FACE_PARTS} are supported",
                    self.face_parts.len()
                ),
            });
        }
        let part_count: u16 = count_as(self.face_parts.len(), "DNA face parts")?;
        let per_part = usize::from(self.blends_per_part);
        if let Some(index) = self
            .face_parts
            .iter()
            .position(|part| part.len() != per_part)
        {
            return Err(ChfError::Unwritable {
                reason: format!(
                    "DNA face part {} has {} blends instead of blends_per_part {per_part}",
                    face_part_name(index),
                    self.face_parts.get(index).map_or(0, Vec::len)
                ),
            });
        }
        out.put_u64(block_size(part_count.into(), self.blends_per_part));
        out.put_u32(DNA_MAGIC);
        out.put_u32(self.gender_hash);
        out.put_u32(self.variant_hash);
        out.put_u32(0);
        out.put_u16(part_count);
        out.put_u16(self.blends_per_part);
        out.put_u16(self.header_unknown);
        out.put_u16(self.max_head_id);
        for blend in 0..per_part {
            for part in &self.face_parts {
                if let Some(entry) = part.get(blend) {
                    out.put_u16(entry.value);
                    out.put_u16(entry.head_id);
                }
            }
        }
        Ok(())
    }
}

/// Face parts as JSON objects that name each part by its position.
mod named_parts {
    use serde::de::Error as _;
    use serde::ser::SerializeSeq as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{DnaBlend, face_part_name};

    #[derive(Serialize)]
    struct NamedRef<'a> {
        part: &'a str,
        blends: &'a [DnaBlend],
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Named<'a> {
        #[serde(borrow)]
        part: std::borrow::Cow<'a, str>,
        blends: Vec<DnaBlend>,
    }

    pub(super) fn serialize<S: Serializer>(
        parts: &[Vec<DnaBlend>],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(parts.len()))?;
        for (index, blends) in parts.iter().enumerate() {
            seq.serialize_element(&NamedRef {
                part: &face_part_name(index),
                blends,
            })?;
        }
        seq.end()
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Vec<DnaBlend>>, D::Error> {
        let named = Vec::<Named<'de>>::deserialize(deserializer)?;
        named
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                let expected = face_part_name(index);
                if part.part == expected {
                    Ok(part.blends)
                } else {
                    Err(D::Error::custom(format_args!(
                        "face part {index} must be named {expected}, not {}",
                        part.part
                    )))
                }
            })
            .collect()
    }
}
